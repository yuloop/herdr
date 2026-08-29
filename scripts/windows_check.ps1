param(
    [ValidateSet("lint", "check")]
    [string]$Mode = "check"
)

$ErrorActionPreference = "Stop"

function Invoke-Checked {
    param([string]$Command, [string[]]$Arguments)

    & $Command @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "command failed with exit code $LASTEXITCODE`: $Command $($Arguments -join ' ')"
    }
}

function Invoke-CargoWithZigCacheRecovery {
    param([string[]]$Arguments)

    & cargo @Arguments
    if ($LASTEXITCODE -eq 0) {
        return
    }

    Write-Warning "cargo compile failed; clearing Zig build caches and retrying once"
    Remove-Item -Recurse -Force .zig-cache -ErrorAction SilentlyContinue
    Remove-Item -Recurse -Force vendor/libghostty-vt/.zig-cache -ErrorAction SilentlyContinue
    Remove-Item -Recurse -Force vendor/libghostty-vt/zig-out -ErrorAction SilentlyContinue
    Invoke-Checked cargo $Arguments
}

function Invoke-CargoTestFilter {
    param(
        [Parameter(Mandatory)]
        [string]$Filter,
        [switch]$Exact
    )

    $commonArguments = @(
        "test",
        "--locked",
        "--target",
        "x86_64-pc-windows-msvc",
        "--bin",
        "herdr",
        $Filter
    )
    $harnessArguments = @("--list")
    if ($Exact) {
        $harnessArguments += "--exact"
    }

    $listArguments = $commonArguments + @("--") + $harnessArguments
    $listOutput = @(& cargo @listArguments)
    if ($LASTEXITCODE -ne 0) {
        throw "could not enumerate tests for filter '$Filter': $($listOutput -join [Environment]::NewLine)"
    }

    $testNames = @(
        foreach ($line in $listOutput) {
            $match = [regex]::Match([string]$line, '^\s*(\S+): test\s*$')
            if ($match.Success) {
                $match.Groups[1].Value
            }
        }
    )
    if ($testNames.Count -eq 0) {
        throw "test filter '$Filter' selected zero tests"
    }

    Write-Host "Running $($testNames.Count) test(s) for '$Filter'"
    # Several Windows tests temporarily replace PATH while probing command shims.
    # Keep filtered suites serial so unrelated process-spawning tests cannot observe
    # another test's temporary environment.
    $runArguments = $commonArguments + @("--", "--test-threads=1")
    if ($Exact) {
        $runArguments += "--exact"
    }
    Invoke-Checked cargo $runArguments
}

Invoke-Checked rustup @("target", "add", "x86_64-pc-windows-msvc")
Invoke-Checked rustup @("target", "add", "x86_64-unknown-linux-musl")
Invoke-Checked cargo @("fmt", "--check")
Invoke-CargoWithZigCacheRecovery @(
    "clippy",
    "--bin",
    "herdr",
    "--locked",
    "--target",
    "x86_64-pc-windows-msvc",
    "--",
    "-D",
    "warnings"
)

$previousLibghosttyVtSimd = $env:LIBGHOSTTY_VT_SIMD
try {
    # A Windows host cannot run Linux tests, but linting the Linux release
    # target catches cfg(unix), Linux-only warning, shared-interface, and
    # target dependency drift before the native Ubuntu gate runs.
    $env:LIBGHOSTTY_VT_SIMD = "false"
    Invoke-CargoWithZigCacheRecovery @(
        "clippy",
        "--bin",
        "herdr",
        "--locked",
        "--target",
        "x86_64-unknown-linux-musl",
        "--",
        "-D",
        "warnings"
    )
} finally {
    if ($null -eq $previousLibghosttyVtSimd) {
        Remove-Item Env:LIBGHOSTTY_VT_SIMD -ErrorAction SilentlyContinue
    } else {
        $env:LIBGHOSTTY_VT_SIMD = $previousLibghosttyVtSimd
    }
}

if ($Mode -eq "lint") {
    return
}

Invoke-Checked python @(
    "-m",
    "unittest",
    "scripts.test_agent_detection_manifest_check",
    "scripts.test_changelog",
    "scripts.test_config_reference_check",
    "scripts.test_cross_platform_gate",
    "scripts.test_docs_translation_parity",
    "scripts.test_i18n_key_check",
    "scripts.test_herdr_automation_issue",
    "scripts.test_herdr_deploy",
    "scripts.test_herdr_deploy_integration",
    "scripts.test_sync_upstream",
    "scripts.test_hermes_integration_asset",
    "scripts.test_package_windows_conpty",
    "scripts.test_preview",
    "scripts.test_vendor_libghostty_vt",
    "scripts.test_vendor_portable_pty"
)
Invoke-Checked powershell.exe @(
    "-NoProfile",
    "-ExecutionPolicy",
    "Bypass",
    "-File",
    (Join-Path $PSScriptRoot "windows_terminal_profile_test.ps1")
)
Invoke-CargoTestFilter "windows_"
Invoke-CargoTestFilter "server::client_transport::tests"
Invoke-CargoTestFilter "app::tests::native_repeats_and_releases_follow_the_pressed_pane" -Exact
Invoke-CargoTestFilter "ui::"
Invoke-Checked cargo @("build", "--locked", "--target", "x86_64-pc-windows-msvc")
