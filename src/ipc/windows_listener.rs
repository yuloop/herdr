use std::{fs, io, path::Path};

use super::{windows_socket_marker, LocalListener};

/// Creates a same-user Windows named pipe that remains reachable across UAC
/// integrity levels. Without an explicit medium mandatory label, a pipe
/// created by an elevated server inherits high integrity and rejects the
/// medium-integrity client started by Explorer before its DACL is evaluated.
/// The DACL must also name the actual token user rather than Owner Rights:
/// elevated tokens can make Administrators the object's default owner.
#[cfg(windows)]
pub(super) fn bind_windows_local_listener(path: &Path) -> io::Result<LocalListener> {
    use interprocess::local_socket::{prelude::*, GenericNamespaced, ListenerOptions};
    use interprocess::os::windows::local_socket::ListenerOptionsExt as _;
    use interprocess::os::windows::security_descriptor::SecurityDescriptor;
    use widestring::U16CString;

    let sddl = U16CString::from_str(windows_user_local_pipe_sddl()?)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err))?;
    let security_descriptor = SecurityDescriptor::deserialize(&sddl)?;
    let name = path.to_string_lossy().to_string();
    let name = name.to_ns_name::<GenericNamespaced>()?;
    let listener = ListenerOptions::new()
        .name(name)
        .reclaim_name(false)
        .security_descriptor(security_descriptor)
        .create_sync()?;
    fs::write(path, windows_socket_marker())?;
    Ok(listener)
}

#[cfg(windows)]
fn windows_user_local_pipe_sddl() -> io::Result<String> {
    let user_sid = windows_current_user_sid_string()?;
    Ok(format!(
        "D:P(A;;GA;;;SY)(A;;GA;;;{user_sid})S:(ML;;NW;;;ME)"
    ))
}

#[cfg(windows)]
fn windows_current_user_sid_string() -> io::Result<String> {
    use std::fmt::Write as _;
    use std::ptr::null_mut;
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
    use windows_sys::Win32::Security::{
        GetSidSubAuthority, GetTokenInformation, IsValidSid, TokenUser, SID, TOKEN_QUERY,
        TOKEN_USER,
    };
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    let mut token: HANDLE = null_mut();
    // SAFETY: `token` is a valid writable handle slot and the pseudo process
    // handle is always valid for the current process.
    if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) } == 0 {
        return Err(io::Error::last_os_error());
    }

    let result = (|| {
        let mut required = 0_u32;
        // SAFETY: the zero-length probe intentionally supplies no buffer and
        // asks Windows for the required TOKEN_USER allocation size.
        unsafe {
            GetTokenInformation(token, TokenUser, null_mut(), 0, &mut required);
        }
        if required == 0 {
            return Err(io::Error::last_os_error());
        }

        // `usize` storage supplies alignment suitable for TOKEN_USER while
        // still providing the exact byte capacity requested by Windows.
        let word_count = (required as usize).div_ceil(std::mem::size_of::<usize>());
        let mut storage = vec![0_usize; word_count];
        // SAFETY: `storage` is writable for at least `required` bytes and
        // remains alive while the returned SID pointer is inspected.
        if unsafe {
            GetTokenInformation(
                token,
                TokenUser,
                storage.as_mut_ptr().cast(),
                required,
                &mut required,
            )
        } == 0
        {
            return Err(io::Error::last_os_error());
        }

        // SAFETY: a successful TokenUser query starts with TOKEN_USER and its
        // SID points into `storage` for the lifetime of this scope.
        let token_user = unsafe { &*storage.as_ptr().cast::<TOKEN_USER>() };
        let sid = token_user.User.Sid;
        // SAFETY: Windows returned `sid` from the successful token query.
        if sid.is_null() || unsafe { IsValidSid(sid) } == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "current Windows token contains an invalid user SID",
            ));
        }

        // SAFETY: IsValidSid confirmed the fixed SID header and subauthority
        // count before either is read.
        let sid_header = unsafe { &*sid.cast::<SID>() };
        let authority = sid_header
            .IdentifierAuthority
            .Value
            .iter()
            .fold(0_u64, |value, byte| (value << 8) | u64::from(*byte));
        let mut sid_string = format!("S-{}-{authority}", sid_header.Revision);
        for index in 0..u32::from(sid_header.SubAuthorityCount) {
            // SAFETY: `index` is bounded by the validated SID's advertised
            // subauthority count.
            let subauthority = unsafe { GetSidSubAuthority(sid, index) };
            if subauthority.is_null() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "current Windows user SID has a missing subauthority",
                ));
            }
            // SAFETY: GetSidSubAuthority returned a pointer inside the valid
            // SID for this bounded index.
            write!(sid_string, "-{}", unsafe { *subauthority })
                .map_err(|_| io::Error::other("could not format Windows user SID"))?;
        }
        Ok(sid_string)
    })();

    // SAFETY: OpenProcessToken returned this owned handle above.
    unsafe {
        CloseHandle(token);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn named_pipe_security_descriptor_targets_current_user_at_medium_integrity() {
        let user_sid = windows_current_user_sid_string().unwrap();
        let sddl = windows_user_local_pipe_sddl().unwrap();

        assert!(user_sid.starts_with("S-1-"));
        assert!(sddl.contains(&format!("(A;;GA;;;{user_sid})")));
        assert!(sddl.ends_with("S:(ML;;NW;;;ME)"));
        assert!(!sddl.contains(";;;OW)"));
    }
}
