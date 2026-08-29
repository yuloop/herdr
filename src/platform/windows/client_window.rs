use std::{
    collections::{HashMap, HashSet},
    mem::size_of,
    path::{Path, PathBuf},
    ptr::null_mut,
    sync::{
        atomic::{AtomicBool, Ordering as AtomicOrdering},
        Arc,
    },
    time::{Duration, Instant},
};

use windows_sys::Win32::{
    Foundation::{CloseHandle, HANDLE, HWND, LPARAM, RECT},
    Storage::FileSystem::{MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH},
    System::{
        Console::{GetConsoleProcessList, GetConsoleTitleW, GetConsoleWindow},
        Threading::{
            CreateMutexW, GetCurrentProcess, ReleaseMutex, WaitForSingleObject,
            PROCESS_QUERY_LIMITED_INFORMATION,
        },
    },
    UI::WindowsAndMessaging::{
        EnumWindows, GetClassNameW, GetForegroundWindow, GetPropW, GetSystemMetrics,
        GetWindowPlacement, GetWindowTextW, GetWindowThreadProcessId, IsWindowVisible, IsZoomed,
        MessageBoxW, RemovePropW, SetPropW, SetWindowPlacement, MB_ICONERROR, MB_OK,
        MB_SETFOREGROUND, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN,
        SM_YVIRTUALSCREEN, SW_SHOWMAXIMIZED, SW_SHOWNORMAL, WINDOWPLACEMENT,
        WPF_RESTORETOMAXIMIZED,
    },
};

use super::{
    extended_length_path, process_creation_time, snapshot_processes, wide_null, ProcessHandle,
    WindowsProcessEntry,
};

const CLIENT_WINDOW_STATE_VERSION: u8 = 1;
const CLIENT_WINDOW_STATE_FILE_NAME: &str = "windows-client-window.json";
const MAX_CLIENT_WINDOW_STATE_BYTES: u64 = 4 * 1024;
const CLIENT_WINDOW_STATE_POLL_INTERVAL: Duration = Duration::from_millis(100);
const CLIENT_WINDOW_DISCOVERY_INTERVAL: Duration = Duration::from_millis(25);
const CLIENT_WINDOW_DISCOVERY_STABILITY: Duration = Duration::from_millis(300);
const CLIENT_WINDOW_DISCOVERY_TIMEOUT: Duration = Duration::from_secs(3);
const CLIENT_WINDOW_DISCOVERY_MUTEX_NAME: &str = "Local\\Herdr.ClientWindowDiscovery.v1";
const CLIENT_WINDOW_OWNER_PROPERTY: &str = "Herdr.ClientWindowOwner.v1";
const CLASSIC_CONSOLE_WINDOW_CLASS: &str = "ConsoleWindowClass";
// GetProcessTimes reports 100-nanosecond ticks. A directly launched console
// client and a newly created default-terminal process start almost together.
// Window ownership additionally requires an exact console-title match and an
// unclaimed window. The time bound only rejects a newly created Terminal
// process that is too late to belong to this launch.
const DIRECT_LAUNCH_TERMINAL_MAX_CREATION_DELTA: u64 = 5 * 10_000_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
struct ClientWindowState {
    version: u8,
    maximized: bool,
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

impl ClientWindowState {
    fn is_valid(self) -> bool {
        let width = i64::from(self.right) - i64::from(self.left);
        let height = i64::from(self.bottom) - i64::from(self.top);
        self.version == CLIENT_WINDOW_STATE_VERSION
            && (64..=131_072).contains(&width)
            && (48..=131_072).contains(&height)
    }

    fn normal_rect(self) -> RECT {
        RECT {
            left: self.left,
            top: self.top,
            right: self.right,
            bottom: self.bottom,
        }
    }

    fn intersects(self, desktop: VirtualDesktopBounds) -> bool {
        i64::from(self.right) > desktop.left
            && i64::from(self.left) < desktop.right
            && i64::from(self.bottom) > desktop.top
            && i64::from(self.top) < desktop.bottom
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct VirtualDesktopBounds {
    left: i64,
    top: i64,
    right: i64,
    bottom: i64,
}

fn current_virtual_desktop_bounds() -> Option<VirtualDesktopBounds> {
    let left = unsafe { GetSystemMetrics(SM_XVIRTUALSCREEN) };
    let top = unsafe { GetSystemMetrics(SM_YVIRTUALSCREEN) };
    let width = unsafe { GetSystemMetrics(SM_CXVIRTUALSCREEN) };
    let height = unsafe { GetSystemMetrics(SM_CYVIRTUALSCREEN) };
    if width <= 0 || height <= 0 {
        return None;
    }
    Some(VirtualDesktopBounds {
        left: i64::from(left),
        top: i64::from(top),
        right: i64::from(left) + i64::from(width),
        bottom: i64::from(top) + i64::from(height),
    })
}

fn client_window_state_from_placement(
    placement: &WINDOWPLACEMENT,
    window_is_zoomed: bool,
) -> Option<ClientWindowState> {
    let rect = placement.rcNormalPosition;
    let state = ClientWindowState {
        version: CLIENT_WINDOW_STATE_VERSION,
        maximized: window_is_zoomed
            || placement.showCmd == SW_SHOWMAXIMIZED as u32
            || placement.flags & WPF_RESTORETOMAXIMIZED != 0,
        left: rect.left,
        top: rect.top,
        right: rect.right,
        bottom: rect.bottom,
    };
    state.is_valid().then_some(state)
}

fn window_placement_from_client_state(state: ClientWindowState) -> WINDOWPLACEMENT {
    WINDOWPLACEMENT {
        length: size_of::<WINDOWPLACEMENT>() as u32,
        flags: if state.maximized {
            WPF_RESTORETOMAXIMIZED
        } else {
            0
        },
        showCmd: if state.maximized {
            SW_SHOWMAXIMIZED as u32
        } else {
            SW_SHOWNORMAL as u32
        },
        rcNormalPosition: state.normal_rect(),
        ..WINDOWPLACEMENT::default()
    }
}

fn capture_client_window_state(hwnd: HWND) -> Option<ClientWindowState> {
    let mut placement = WINDOWPLACEMENT {
        length: size_of::<WINDOWPLACEMENT>() as u32,
        ..WINDOWPLACEMENT::default()
    };
    if unsafe { GetWindowPlacement(hwnd, &mut placement) } == 0 {
        return None;
    }
    client_window_state_from_placement(&placement, unsafe { IsZoomed(hwnd) } != 0)
}

fn apply_client_window_state(hwnd: HWND, state: ClientWindowState) -> bool {
    if !state.is_valid()
        || !current_virtual_desktop_bounds().is_some_and(|desktop| state.intersects(desktop))
    {
        return false;
    }
    let placement = window_placement_from_client_state(state);
    (unsafe { SetWindowPlacement(hwnd, &placement) }) != 0
}

fn load_client_window_state(path: &Path) -> Option<ClientWindowState> {
    let metadata = match std::fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return None,
        Err(err) => {
            tracing::warn!(path = %path.display(), %err, "failed to inspect Windows client window state");
            return None;
        }
    };
    if metadata.len() > MAX_CLIENT_WINDOW_STATE_BYTES {
        tracing::warn!(
            path = %path.display(),
            bytes = metadata.len(),
            "ignoring oversized Windows client window state"
        );
        return None;
    }
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(err) => {
            tracing::warn!(path = %path.display(), %err, "failed to read Windows client window state");
            return None;
        }
    };
    match serde_json::from_slice::<ClientWindowState>(&bytes) {
        Ok(state) if state.is_valid() => Some(state),
        Ok(_) => {
            tracing::warn!(path = %path.display(), "ignoring invalid Windows client window state");
            None
        }
        Err(err) => {
            tracing::warn!(path = %path.display(), %err, "failed to parse Windows client window state");
            None
        }
    }
}

fn save_client_window_state(path: &Path, state: ClientWindowState) -> std::io::Result<()> {
    if !state.is_valid() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "invalid Windows client window state",
        ));
    }
    let parent = path.parent().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Windows client window state path has no parent",
        )
    })?;
    std::fs::create_dir_all(parent)?;

    let mut bytes = serde_json::to_vec_pretty(&state).map_err(std::io::Error::other)?;
    bytes.push(b'\n');
    let tmp_path = path.with_extension(format!("json.tmp.{}", std::process::id()));
    std::fs::write(&tmp_path, bytes)?;

    let source = extended_length_path(&tmp_path)?;
    let destination = extended_length_path(path)?;
    if unsafe {
        MoveFileExW(
            source.as_ptr(),
            destination.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    } == 0
    {
        let err = std::io::Error::last_os_error();
        let _ = std::fs::remove_file(&tmp_path);
        return Err(err);
    }
    Ok(())
}

fn console_process_list_is_owned(
    current_pid: u32,
    reported_count: u32,
    process_ids: &[u32],
) -> bool {
    reported_count == 1 && process_ids.first().copied() == Some(current_pid)
}

/// Returns true when this process is the only process attached to its console.
///
/// A Windows Explorer double-click can inherit `HERDR_ENV=1` from an Explorer
/// process that was opened inside a Herdr pane. The new application still owns
/// a separate console, so it is not actually a nested Herdr invocation.
pub(crate) fn current_process_has_standalone_console() -> bool {
    if unsafe { GetConsoleWindow() }.is_null() {
        return false;
    }
    let mut process_ids = [0_u32; 2];
    let count = unsafe {
        GetConsoleProcessList(
            process_ids.as_mut_ptr(),
            u32::try_from(process_ids.len()).unwrap_or(u32::MAX),
        )
    };
    console_process_list_is_owned(std::process::id(), count, &process_ids)
}

fn windows_terminal_ancestor_pid(current_pid: u32, entries: &[WindowsProcessEntry]) -> Option<u32> {
    let entries_by_pid: HashMap<_, _> = entries.iter().map(|entry| (entry.pid, entry)).collect();
    let mut current = current_pid;
    let mut visited = HashSet::new();
    while visited.insert(current) {
        let parent_pid = entries_by_pid.get(&current)?.parent_pid;
        if parent_pid == 0 {
            return None;
        }
        let parent = entries_by_pid.get(&parent_pid)?;
        if parent.name.eq_ignore_ascii_case("WindowsTerminal.exe") {
            return Some(parent.pid);
        }
        current = parent.pid;
    }
    None
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WindowsTerminalOwner {
    Ancestor(u32),
    DirectLaunch {
        process_id: u32,
        window_handle: usize,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct VisibleWindowsTerminalWindow {
    process_id: u32,
    window_handle: usize,
    creation_time: u64,
    claimed_by_herdr_client: bool,
    title_matches_client: bool,
}

fn select_windows_terminal_owner(
    current_pid: u32,
    entries: &[WindowsProcessEntry],
    current_creation_time: Option<u64>,
    visible_terminal_windows: &[VisibleWindowsTerminalWindow],
) -> Option<WindowsTerminalOwner> {
    if let Some(process_id) = windows_terminal_ancestor_pid(current_pid, entries) {
        return Some(WindowsTerminalOwner::Ancestor(process_id));
    }

    let current_creation_time = current_creation_time?;
    let mut matching = visible_terminal_windows.iter().filter(|window| {
        entries.iter().any(|entry| {
            entry.pid == window.process_id && entry.name.eq_ignore_ascii_case("WindowsTerminal.exe")
        }) && !window.claimed_by_herdr_client
            && window.title_matches_client
            && (window.creation_time < current_creation_time
                || window
                    .creation_time
                    .checked_sub(current_creation_time)
                    .is_some_and(|delta| delta <= DIRECT_LAUNCH_TERMINAL_MAX_CREATION_DELTA))
    });
    let selected = *matching.next()?;
    if matching.next().is_some() {
        return None;
    }

    Some(WindowsTerminalOwner::DirectLaunch {
        process_id: selected.process_id,
        window_handle: selected.window_handle,
    })
}

fn window_process_id(hwnd: HWND) -> Option<u32> {
    let mut process_id = 0;
    if unsafe { GetWindowThreadProcessId(hwnd, &mut process_id) } == 0 || process_id == 0 {
        return None;
    }
    Some(process_id)
}

fn window_class_name(hwnd: HWND) -> Option<String> {
    let mut buffer = [0_u16; 256];
    let copied = unsafe {
        GetClassNameW(
            hwnd,
            buffer.as_mut_ptr(),
            i32::try_from(buffer.len()).unwrap_or(i32::MAX),
        )
    };
    let copied = usize::try_from(copied).ok()?;
    (copied > 0).then(|| String::from_utf16_lossy(&buffer[..copied]))
}

fn is_owned_classic_console_candidate(
    current_pid: u32,
    standalone_console: bool,
    visible: bool,
    window_process_id: Option<u32>,
    class_name: Option<&str>,
) -> bool {
    standalone_console
        && visible
        && window_process_id == Some(current_pid)
        && class_name == Some(CLASSIC_CONSOLE_WINDOW_CLASS)
}

enum ClassicConsoleWindowProbe {
    Absent,
    Owned(HWND),
    Untrusted,
}

fn probe_current_classic_console_window() -> ClassicConsoleWindowProbe {
    let hwnd = unsafe { GetConsoleWindow() };
    if hwnd.is_null() || unsafe { IsWindowVisible(hwnd) } == 0 {
        // Under ConPTY, GetConsoleWindow may return an invisible message-only
        // window. That is not the user's host window; Windows Terminal
        // discovery below must identify the visible host instead.
        return ClassicConsoleWindowProbe::Absent;
    }

    let class_name = window_class_name(hwnd);
    let window_process_id = window_process_id(hwnd);
    let standalone_console = current_process_has_standalone_console();
    let owned = is_owned_classic_console_candidate(
        std::process::id(),
        standalone_console,
        true,
        window_process_id,
        class_name.as_deref(),
    );
    tracing::debug!(
        hwnd = hwnd as usize,
        ?window_process_id,
        ?class_name,
        standalone_console,
        owned,
        "evaluated current classic console window"
    );
    if owned {
        ClassicConsoleWindowProbe::Owned(hwnd)
    } else {
        ClassicConsoleWindowProbe::Untrusted
    }
}

struct ProcessWindowCollector {
    process_id: u32,
    handles: Vec<usize>,
}

unsafe extern "system" fn collect_visible_process_window(hwnd: HWND, lparam: LPARAM) -> i32 {
    let collector = unsafe { &mut *(lparam as *mut ProcessWindowCollector) };
    if unsafe { IsWindowVisible(hwnd) } != 0
        && window_process_id(hwnd) == Some(collector.process_id)
    {
        collector.handles.push(hwnd as usize);
    }
    1
}

fn try_visible_process_windows(process_id: u32) -> Option<Vec<usize>> {
    let mut collector = ProcessWindowCollector {
        process_id,
        handles: Vec::new(),
    };
    let completed = unsafe {
        EnumWindows(
            Some(collect_visible_process_window),
            &mut collector as *mut ProcessWindowCollector as LPARAM,
        )
    };
    if completed == 0 {
        return None;
    }
    Some(collector.handles)
}

fn visible_process_windows(process_id: u32) -> Vec<usize> {
    try_visible_process_windows(process_id).unwrap_or_default()
}

fn client_window_owner_pid(hwnd: HWND) -> Option<u32> {
    let property = wide_null(CLIENT_WINDOW_OWNER_PROPERTY);
    let owner = unsafe { GetPropW(hwnd, property.as_ptr()) };
    (!owner.is_null()).then_some(owner as usize as u32)
}

fn claim_client_window(hwnd: HWND) -> bool {
    let current_pid = std::process::id();
    if let Some(owner_pid) = client_window_owner_pid(hwnd) {
        return owner_pid == current_pid;
    }

    let property = wide_null(CLIENT_WINDOW_OWNER_PROPERTY);
    let owner = current_pid as usize as HANDLE;
    (unsafe { SetPropW(hwnd, property.as_ptr(), owner) }) != 0
        && client_window_owner_pid(hwnd) == Some(current_pid)
}

fn release_client_window_claim(hwnd: HWND) {
    if client_window_owner_pid(hwnd) != Some(std::process::id()) {
        return;
    }
    let property = wide_null(CLIENT_WINDOW_OWNER_PROPERTY);
    unsafe {
        RemovePropW(hwnd, property.as_ptr());
    }
}

struct ClientWindowDiscoveryLock(HANDLE);

impl ClientWindowDiscoveryLock {
    fn acquire() -> Option<Self> {
        const WAIT_OBJECT_0: u32 = 0;
        const WAIT_ABANDONED: u32 = 0x80;

        let name = wide_null(CLIENT_WINDOW_DISCOVERY_MUTEX_NAME);
        let handle = unsafe { CreateMutexW(null_mut(), 0, name.as_ptr()) };
        if handle.is_null() {
            return None;
        }
        match unsafe { WaitForSingleObject(handle, 100) } {
            WAIT_OBJECT_0 | WAIT_ABANDONED => Some(Self(handle)),
            _ => {
                unsafe {
                    CloseHandle(handle);
                }
                None
            }
        }
    }
}

impl Drop for ClientWindowDiscoveryLock {
    fn drop(&mut self) {
        unsafe {
            ReleaseMutex(self.0);
            CloseHandle(self.0);
        }
    }
}

fn window_title(hwnd: HWND) -> Option<String> {
    let mut buffer = [0_u16; 512];
    let copied = unsafe {
        GetWindowTextW(
            hwnd,
            buffer.as_mut_ptr(),
            i32::try_from(buffer.len()).unwrap_or(i32::MAX),
        )
    };
    let copied = usize::try_from(copied).ok()?;
    (copied > 0).then(|| String::from_utf16_lossy(&buffer[..copied]))
}

fn current_console_title() -> Option<String> {
    let mut title = vec![0_u16; 32_768];
    let copied = unsafe {
        GetConsoleTitleW(
            title.as_mut_ptr(),
            u32::try_from(title.len()).unwrap_or(u32::MAX),
        )
    };
    let copied = usize::try_from(copied).ok()?;
    (copied > 0).then(|| String::from_utf16_lossy(&title[..copied]))
}

fn select_process_window(
    process_id: u32,
    foreground: Option<(usize, u32)>,
    visible_handles: &[usize],
) -> Option<usize> {
    if let Some((foreground_handle, foreground_process_id)) = foreground {
        if foreground_process_id == process_id && visible_handles.contains(&foreground_handle) {
            return Some(foreground_handle);
        }
    }
    match visible_handles {
        [only] => Some(*only),
        _ => None,
    }
}

fn visible_windows_terminal_windows(
    entries: &[WindowsProcessEntry],
    client_title: Option<&str>,
) -> Vec<VisibleWindowsTerminalWindow> {
    let mut windows = Vec::new();
    for entry in entries
        .iter()
        .filter(|entry| entry.name.eq_ignore_ascii_case("WindowsTerminal.exe"))
    {
        let Some(creation_time) = process_creation_time_by_pid(entry.pid) else {
            continue;
        };
        windows.extend(
            visible_process_windows(entry.pid)
                .into_iter()
                .map(|window_handle| VisibleWindowsTerminalWindow {
                    process_id: entry.pid,
                    window_handle,
                    creation_time,
                    claimed_by_herdr_client: client_window_owner_pid(window_handle as HWND)
                        .is_some(),
                    title_matches_client: client_title.is_some_and(|expected| {
                        window_title(window_handle as HWND).as_deref() == Some(expected)
                    }),
                }),
        );
    }
    windows
}

fn process_creation_time_by_pid(process_id: u32) -> Option<u64> {
    let process = ProcessHandle::open(process_id, PROCESS_QUERY_LIMITED_INFORMATION)?;
    process_creation_time(process.0)
}

fn owned_windows_terminal_window(client_title: Option<&str>) -> Option<HWND> {
    let entries = snapshot_processes();
    let foreground = unsafe { GetForegroundWindow() };
    let foreground = if foreground.is_null() {
        None
    } else {
        window_process_id(foreground).map(|process_id| (foreground as usize, process_id))
    };
    let direct_launch_windows =
        if windows_terminal_ancestor_pid(std::process::id(), &entries).is_none() {
            visible_windows_terminal_windows(&entries, client_title)
        } else {
            Vec::new()
        };
    let owner = select_windows_terminal_owner(
        std::process::id(),
        &entries,
        process_creation_time(unsafe { GetCurrentProcess() }),
        &direct_launch_windows,
    )?;
    let (terminal_pid, selected) = match owner {
        WindowsTerminalOwner::Ancestor(process_id) => {
            let visible_handles = visible_process_windows(process_id)
                .into_iter()
                .filter(|window_handle| client_window_owner_pid(*window_handle as HWND).is_none())
                .collect::<Vec<_>>();
            let selected = select_process_window(process_id, foreground, &visible_handles)
                .or_else(|| select_process_window(process_id, None, &visible_handles));
            (process_id, selected)
        }
        WindowsTerminalOwner::DirectLaunch {
            process_id,
            window_handle,
        } => (process_id, Some(window_handle)),
    };
    tracing::debug!(
        terminal_pid,
        direct_launch = matches!(owner, WindowsTerminalOwner::DirectLaunch { .. }),
        direct_launch_candidate_count = direct_launch_windows.len(),
        selected = selected.is_some(),
        "evaluated owning Windows Terminal window"
    );
    selected.map(|hwnd| hwnd as HWND)
}

fn wait_for_owned_windows_terminal_window() -> Option<HWND> {
    let deadline = Instant::now() + CLIENT_WINDOW_DISCOVERY_TIMEOUT;
    let mut stable_candidate: Option<(usize, Instant)> = None;
    loop {
        let discovery_lock = ClientWindowDiscoveryLock::acquire();
        let candidate = discovery_lock.as_ref().and_then(|_| {
            let client_title = current_console_title();
            owned_windows_terminal_window(client_title.as_deref())
        });
        match candidate {
            Some(hwnd) => {
                let hwnd_bits = hwnd as usize;
                let stable_since = match stable_candidate {
                    Some((previous, since)) if previous == hwnd_bits => since,
                    _ => {
                        let since = Instant::now();
                        stable_candidate = Some((hwnd_bits, since));
                        since
                    }
                };
                if stable_since.elapsed() >= CLIENT_WINDOW_DISCOVERY_STABILITY
                    && claim_client_window(hwnd)
                {
                    return Some(hwnd);
                }
            }
            None => stable_candidate = None,
        }
        drop(discovery_lock);
        let now = Instant::now();
        if now >= deadline {
            tracing::warn!(
                timeout_ms = CLIENT_WINDOW_DISCOVERY_TIMEOUT.as_millis(),
                "could not identify the owning Windows Terminal window"
            );
            return None;
        }
        std::thread::sleep(CLIENT_WINDOW_DISCOVERY_INTERVAL.min(deadline - now));
    }
}

fn owned_client_window() -> Option<HWND> {
    match probe_current_classic_console_window() {
        ClassicConsoleWindowProbe::Owned(hwnd) => {
            if claim_client_window(hwnd) {
                Some(hwnd)
            } else {
                tracing::warn!(
                    hwnd = hwnd as usize,
                    "could not claim the current classic console window"
                );
                None
            }
        }
        ClassicConsoleWindowProbe::Untrusted => {
            tracing::warn!("refusing to manage an untrusted visible console window");
            None
        }
        ClassicConsoleWindowProbe::Absent => wait_for_owned_windows_terminal_window(),
    }
}

pub(crate) fn show_startup_error_dialog(title: &str, message: &str) {
    let title = title
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let message = message
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    unsafe {
        MessageBoxW(
            null_mut(),
            message.as_ptr(),
            title.as_ptr(),
            MB_OK | MB_ICONERROR | MB_SETFOREGROUND,
        );
    }
}

/// Restores and continuously records the proven host window for this Herdr
/// client. Other Windows Terminal and classic console windows are untouched.
pub(crate) struct ClientWindowStateGuard {
    hwnd: Option<usize>,
    path: Option<PathBuf>,
    stop: Arc<AtomicBool>,
    watcher: Option<std::thread::JoinHandle<()>>,
}

impl ClientWindowStateGuard {
    fn disabled() -> Self {
        Self {
            hwnd: None,
            path: None,
            stop: Arc::new(AtomicBool::new(true)),
            watcher: None,
        }
    }
}

impl Drop for ClientWindowStateGuard {
    fn drop(&mut self) {
        self.stop.store(true, AtomicOrdering::Release);
        if let Some(watcher) = self.watcher.take() {
            watcher.thread().unpark();
            let _ = watcher.join();
        }
        if let (Some(hwnd), Some(path)) = (self.hwnd, self.path.as_deref()) {
            if let Some(state) = capture_client_window_state(hwnd as HWND) {
                if let Err(err) = save_client_window_state(path, state) {
                    tracing::warn!(path = %path.display(), %err, "failed to save Windows client window state on exit");
                }
            }
        }
        if let Some(hwnd) = self.hwnd {
            release_client_window_claim(hwnd as HWND);
        }
    }
}

fn should_attempt_windows_terminal_window_discovery() -> bool {
    // Window state persistence must work regardless of how the client was
    // launched. Directly launching herdr.exe (e.g. from Explorer) does not set
    // WT_SESSION, so gating on that env var would disable maximized-state
    // restore for the common launch path. Discovery still refuses to manage a
    // window unless it can prove ownership through the process tree or the
    // strict direct-launch process/window correlation.
    true
}

pub(crate) fn restore_and_watch_client_window_state() -> ClientWindowStateGuard {
    if !should_attempt_windows_terminal_window_discovery() {
        tracing::debug!("client window persistence disabled");
        return ClientWindowStateGuard::disabled();
    }

    let Some(hwnd) = owned_client_window() else {
        return ClientWindowStateGuard::disabled();
    };
    let path = crate::config::state_dir().join(CLIENT_WINDOW_STATE_FILE_NAME);
    let saved = load_client_window_state(&path);
    let restored = saved.is_some_and(|state| apply_client_window_state(hwnd, state));
    if saved.is_some() && !restored {
        tracing::warn!(
            path = %path.display(),
            "ignored Windows client window state outside the visible desktop"
        );
    }

    let initial_state = capture_client_window_state(hwnd);
    if !restored || saved.is_none() {
        if let Some(state) = initial_state {
            if let Err(err) = save_client_window_state(&path, state) {
                tracing::warn!(path = %path.display(), %err, "failed to initialize Windows client window state");
            }
        }
    }

    let stop = Arc::new(AtomicBool::new(false));
    let watcher_stop = stop.clone();
    let watcher_path = path.clone();
    let hwnd_bits = hwnd as usize;
    let watcher = match std::thread::Builder::new()
        .name("herdr-client-window-state".into())
        .spawn(move || {
            let hwnd = hwnd_bits as HWND;
            let mut last_observed = initial_state;
            while !watcher_stop.load(AtomicOrdering::Acquire) {
                if let Some(state) = capture_client_window_state(hwnd) {
                    if Some(state) != last_observed {
                        last_observed = Some(state);
                        if let Err(err) = save_client_window_state(&watcher_path, state) {
                            tracing::warn!(
                                path = %watcher_path.display(),
                                %err,
                                "failed to update Windows client window state"
                            );
                        }
                    }
                }
                std::thread::park_timeout(CLIENT_WINDOW_STATE_POLL_INTERVAL);
            }
        }) {
        Ok(watcher) => Some(watcher),
        Err(err) => {
            tracing::warn!(%err, "failed to start Windows client window state watcher");
            None
        }
    };

    ClientWindowStateGuard {
        hwnd: Some(hwnd_bits),
        path: Some(path),
        stop,
        watcher,
    }
}

#[cfg(test)]
mod tests {
    use windows_sys::Win32::{
        Foundation::RECT,
        UI::WindowsAndMessaging::{SW_SHOWMAXIMIZED, SW_SHOWMINIMIZED, WPF_RESTORETOMAXIMIZED},
    };

    fn test_entry(
        pid: u32,
        parent_pid: u32,
        name: &str,
        argv: &[&str],
    ) -> super::WindowsProcessEntry {
        super::WindowsProcessEntry {
            pid,
            parent_pid,
            name: name.to_string(),
            argv0: argv.first().map(|value| (*value).to_string()),
            argv: Some(argv.iter().map(|value| (*value).to_string()).collect()),
            cmdline: Some(argv.join(" ")),
        }
    }

    fn sample_client_window_state(maximized: bool) -> super::ClientWindowState {
        super::ClientWindowState {
            version: super::CLIENT_WINDOW_STATE_VERSION,
            maximized,
            left: 120,
            top: 80,
            right: 1320,
            bottom: 880,
        }
    }

    #[test]
    fn client_window_state_round_trips_json() {
        let state = sample_client_window_state(true);
        let json = serde_json::to_vec(&state).expect("serialize window state");
        assert_eq!(
            serde_json::from_slice::<super::ClientWindowState>(&json)
                .expect("deserialize window state"),
            state
        );
    }

    #[test]
    fn client_window_state_rejects_invalid_geometry_and_version() {
        let mut state = sample_client_window_state(false);
        assert!(state.is_valid());

        state.version += 1;
        assert!(!state.is_valid());
        state.version = super::CLIENT_WINDOW_STATE_VERSION;
        state.right = state.left;
        assert!(!state.is_valid());
        state.right = state.left + 1200;
        state.bottom = state.top + 20;
        assert!(!state.is_valid());
    }

    #[test]
    fn client_window_state_requires_visible_desktop_intersection() {
        let state = sample_client_window_state(false);
        let primary = super::VirtualDesktopBounds {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1080,
        };
        let disconnected_monitor = super::VirtualDesktopBounds {
            left: 2000,
            top: 0,
            right: 3920,
            bottom: 1080,
        };
        assert!(state.intersects(primary));
        assert!(!state.intersects(disconnected_monitor));
    }

    #[test]
    fn client_window_placement_preserves_normal_rect_and_maximized_state() {
        let state = sample_client_window_state(true);
        let placement = super::window_placement_from_client_state(state);
        assert_eq!(placement.showCmd, SW_SHOWMAXIMIZED as u32);
        assert_eq!(placement.flags, WPF_RESTORETOMAXIMIZED);
        assert_eq!(placement.rcNormalPosition.left, state.left);
        assert_eq!(placement.rcNormalPosition.top, state.top);
        assert_eq!(placement.rcNormalPosition.right, state.right);
        assert_eq!(placement.rcNormalPosition.bottom, state.bottom);
        assert_eq!(
            super::client_window_state_from_placement(&placement, false),
            Some(state)
        );

        let minimized_after_maximize =
            windows_sys::Win32::UI::WindowsAndMessaging::WINDOWPLACEMENT {
                showCmd: SW_SHOWMINIMIZED as u32,
                flags: WPF_RESTORETOMAXIMIZED,
                rcNormalPosition: RECT {
                    left: state.left,
                    top: state.top,
                    right: state.right,
                    bottom: state.bottom,
                },
                ..windows_sys::Win32::UI::WindowsAndMessaging::WINDOWPLACEMENT::default()
            };
        assert!(
            super::client_window_state_from_placement(&minimized_after_maximize, false)
                .expect("decode minimized placement")
                .maximized
        );
    }

    #[test]
    fn client_window_state_only_owns_a_single_process_console() {
        assert!(super::console_process_list_is_owned(42, 1, &[42, 0]));
        assert!(!super::console_process_list_is_owned(42, 0, &[0, 0]));
        assert!(!super::console_process_list_is_owned(42, 1, &[7, 0]));
        assert!(!super::console_process_list_is_owned(42, 2, &[42, 7]));
    }

    #[test]
    fn client_window_state_accepts_only_the_current_classic_console() {
        assert!(super::is_owned_classic_console_candidate(
            42,
            true,
            true,
            Some(42),
            Some(super::CLASSIC_CONSOLE_WINDOW_CLASS),
        ));

        // A visible console belonging to another process must never be used as
        // a foreground fallback, even when it has the expected window class.
        assert!(!super::is_owned_classic_console_candidate(
            42,
            true,
            true,
            Some(7),
            Some(super::CLASSIC_CONSOLE_WINDOW_CLASS),
        ));
        assert!(!super::is_owned_classic_console_candidate(
            42,
            false,
            true,
            Some(42),
            Some(super::CLASSIC_CONSOLE_WINDOW_CLASS),
        ));
        assert!(!super::is_owned_classic_console_candidate(
            42,
            true,
            false,
            Some(42),
            Some(super::CLASSIC_CONSOLE_WINDOW_CLASS),
        ));
        assert!(!super::is_owned_classic_console_candidate(
            42,
            true,
            true,
            Some(42),
            Some("CASCADIA_HOSTING_WINDOW_CLASS"),
        ));
    }

    #[test]
    fn client_window_state_finds_owning_windows_terminal_ancestor() {
        let entries = vec![
            test_entry(10, 20, "herdr.exe", &[]),
            test_entry(20, 30, "powershell.exe", &[]),
            test_entry(30, 0, "WindowsTerminal.exe", &[]),
            test_entry(40, 0, "WindowsTerminal.exe", &[]),
        ];

        assert_eq!(super::windows_terminal_ancestor_pid(10, &entries), Some(30));
        assert_eq!(super::windows_terminal_ancestor_pid(40, &entries), None);
        assert_eq!(super::windows_terminal_ancestor_pid(99, &entries), None);
    }

    #[test]
    fn client_window_state_correlates_direct_launch_with_unique_new_terminal() {
        let entries = vec![
            test_entry(10, 20, "herdr.exe", &[]),
            // The direct launcher's short-lived parent has already exited, so
            // there is deliberately no process-tree path to the Terminal.
            test_entry(30, 0, "WindowsTerminal.exe", &[]),
            test_entry(40, 0, "WindowsTerminal.exe", &[]),
        ];
        let client_created = 100_000_000;
        let visible_windows = [
            super::VisibleWindowsTerminalWindow {
                process_id: 30,
                window_handle: 300,
                creation_time: client_created + 500_000,
                claimed_by_herdr_client: false,
                title_matches_client: true,
            },
            super::VisibleWindowsTerminalWindow {
                process_id: 40,
                window_handle: 400,
                creation_time: client_created - 60_000_000,
                claimed_by_herdr_client: false,
                title_matches_client: false,
            },
        ];

        assert_eq!(
            super::select_windows_terminal_owner(
                10,
                &entries,
                Some(client_created),
                &visible_windows,
            ),
            Some(super::WindowsTerminalOwner::DirectLaunch {
                process_id: 30,
                window_handle: 300,
            })
        );
    }

    #[test]
    fn client_window_state_correlates_new_window_in_existing_terminal_process() {
        let entries = vec![
            test_entry(10, 20, "herdr.exe", &[]),
            test_entry(30, 0, "WindowsTerminal.exe", &[]),
        ];
        let client_created = 100_000_000;
        let terminal_created = client_created - 60_000_000;
        let visible_windows = [
            super::VisibleWindowsTerminalWindow {
                process_id: 30,
                window_handle: 300,
                creation_time: terminal_created,
                claimed_by_herdr_client: true,
                title_matches_client: true,
            },
            super::VisibleWindowsTerminalWindow {
                process_id: 30,
                window_handle: 301,
                creation_time: terminal_created,
                claimed_by_herdr_client: false,
                title_matches_client: true,
            },
        ];

        assert_eq!(
            super::select_windows_terminal_owner(
                10,
                &entries,
                Some(client_created),
                &visible_windows,
            ),
            Some(super::WindowsTerminalOwner::DirectLaunch {
                process_id: 30,
                window_handle: 301,
            })
        );
    }

    #[test]
    fn client_window_state_rejects_unrelated_direct_launch_windows() {
        let entries = vec![
            test_entry(10, 20, "herdr.exe", &[]),
            test_entry(30, 0, "WindowsTerminal.exe", &[]),
            test_entry(40, 0, "notepad.exe", &[]),
        ];
        let client_created = 100_000_000;
        let preexisting_terminal = [super::VisibleWindowsTerminalWindow {
            process_id: 30,
            window_handle: 300,
            // Even a Terminal created immediately before the Herdr client is
            // not owned by this direct launch.
            creation_time: client_created - 1,
            claimed_by_herdr_client: true,
            title_matches_client: true,
        }];
        let late_terminal = [super::VisibleWindowsTerminalWindow {
            process_id: 30,
            window_handle: 300,
            creation_time: client_created + super::DIRECT_LAUNCH_TERMINAL_MAX_CREATION_DELTA + 1,
            claimed_by_herdr_client: false,
            title_matches_client: true,
        }];
        let non_terminal = [super::VisibleWindowsTerminalWindow {
            process_id: 40,
            window_handle: 400,
            creation_time: client_created + 1,
            claimed_by_herdr_client: false,
            title_matches_client: true,
        }];
        let untitled_new_terminal = [super::VisibleWindowsTerminalWindow {
            process_id: 30,
            window_handle: 301,
            creation_time: client_created + 1,
            claimed_by_herdr_client: false,
            title_matches_client: false,
        }];

        assert_eq!(
            super::select_windows_terminal_owner(
                10,
                &entries,
                Some(client_created),
                &preexisting_terminal,
            ),
            None
        );
        assert_eq!(
            super::select_windows_terminal_owner(
                10,
                &entries,
                Some(client_created),
                &late_terminal,
            ),
            None
        );
        assert_eq!(
            super::select_windows_terminal_owner(10, &entries, Some(client_created), &non_terminal,),
            None
        );
        assert_eq!(
            super::select_windows_terminal_owner(
                10,
                &entries,
                Some(client_created),
                &untitled_new_terminal,
            ),
            None
        );
        assert_eq!(
            super::select_windows_terminal_owner(10, &entries, None, &late_terminal),
            None
        );
    }

    #[test]
    fn client_window_state_rejects_ambiguous_direct_launch_windows() {
        let entries = vec![
            test_entry(10, 20, "herdr.exe", &[]),
            test_entry(30, 0, "WindowsTerminal.exe", &[]),
            test_entry(40, 0, "WindowsTerminal.exe", &[]),
        ];
        let client_created = 100_000_000;
        let visible_windows = [
            super::VisibleWindowsTerminalWindow {
                process_id: 30,
                window_handle: 300,
                creation_time: client_created + 1,
                claimed_by_herdr_client: false,
                title_matches_client: true,
            },
            super::VisibleWindowsTerminalWindow {
                process_id: 40,
                window_handle: 400,
                creation_time: client_created + 2,
                claimed_by_herdr_client: false,
                title_matches_client: true,
            },
        ];

        assert_eq!(
            super::select_windows_terminal_owner(
                10,
                &entries,
                Some(client_created),
                &visible_windows,
            ),
            None
        );
    }

    #[test]
    fn client_window_state_rejects_multiple_windows_from_new_terminal() {
        let entries = vec![
            test_entry(10, 20, "herdr.exe", &[]),
            test_entry(30, 0, "WindowsTerminal.exe", &[]),
        ];
        let client_created = 100_000_000;
        let visible_windows = [
            super::VisibleWindowsTerminalWindow {
                process_id: 30,
                window_handle: 300,
                creation_time: client_created + 1,
                claimed_by_herdr_client: false,
                title_matches_client: true,
            },
            super::VisibleWindowsTerminalWindow {
                process_id: 30,
                window_handle: 301,
                creation_time: client_created + 1,
                claimed_by_herdr_client: false,
                title_matches_client: true,
            },
        ];

        assert_eq!(
            super::select_windows_terminal_owner(
                10,
                &entries,
                Some(client_created),
                &visible_windows,
            ),
            None
        );
    }

    #[test]
    fn client_window_state_selects_only_owning_terminal_window() {
        assert_eq!(
            super::select_process_window(30, Some((300, 30)), &[300, 301]),
            Some(300)
        );
        assert_eq!(
            super::select_process_window(30, Some((900, 90)), &[300]),
            Some(300)
        );
        assert_eq!(
            super::select_process_window(30, Some((900, 90)), &[300, 301]),
            None
        );
    }

    #[test]
    fn client_window_state_attempts_discovery_without_wt_session() {
        // Window state persistence must not depend on how the client was
        // launched (WT_SESSION is absent when herdr.exe is launched directly).
        assert!(super::should_attempt_windows_terminal_window_discovery());
    }
}
