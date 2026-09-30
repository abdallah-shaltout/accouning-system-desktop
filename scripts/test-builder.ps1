# Background test builder (user request 2026-09-29: make the test pass fast with many agents).
# Rebuilds the `all` integration test binary whenever Rust sources change, through the shared
# cargo lock (scripts/cargo-safe.ps1), and publishes each good build as a versioned exe so agents
# can run it without building themselves (a running exe can't be overwritten on Windows).
#
# Status: .diagnostics/tests/build-status.json
#   { startedUtc, finishedUtc, exit, exe, log }   — `exe` is the newest successful build.
# An agent that edited Rust at time T waits until startedUtc > T and finishedUtc is set, then:
#   - exit != 0 → read `log` (compile errors) and fix its own files;
#   - exit == 0 → run `exe` from src-tauri/ with EQUAL_TEST_DATABASE_URL set and a suite filter.
#
# Usage: powershell -NoProfile -File scripts/test-builder.ps1   (runs until stopped)
$repo = Split-Path -Parent $PSScriptRoot
Set-Location $repo
$testsDir = Join-Path $repo ".diagnostics\tests"
New-Item -ItemType Directory -Force $testsDir | Out-Null
$statusFile = Join-Path $testsDir "build-status.json"
$watch = @("src-tauri\src", "src-tauri\tests", "src-tauri\migration\src")

function Get-LatestChange {
    $latest = [DateTime]::MinValue
    foreach ($dir in $watch) {
        Get-ChildItem $dir -Recurse -File -Filter *.rs -ErrorAction SilentlyContinue | ForEach-Object {
            if ($_.LastWriteTimeUtc -gt $latest) { $latest = $_.LastWriteTimeUtc }
        }
    }
    $latest
}

$lastExe = $null
while ($true) {
    $started = [DateTime]::UtcNow
    @{ startedUtc = $started.ToString('o'); finishedUtc = $null; exit = $null; exe = $lastExe; log = ".diagnostics/cargo/test-builder.err" } |
        ConvertTo-Json | Set-Content $statusFile -Encoding utf8

    & powershell -NoProfile -File (Join-Path $repo "scripts\cargo-safe.ps1") test-builder test --manifest-path src-tauri/Cargo.toml --test all --no-run | Out-Null
    $err = Get-Content (Join-Path $repo ".diagnostics\cargo\test-builder.err") -ErrorAction SilentlyContinue
    $ok = -not ($err | Select-String -Pattern '^error' -Quiet)
    if ($ok) {
        $built = ($err | Select-String -Pattern 'Executable tests\\all\\main\.rs \((.+)\)').Matches | Select-Object -First 1
        if ($built) {
            $src = Join-Path $repo $built.Groups[1].Value
            $dst = Join-Path $testsDir ("all-" + $started.ToString('yyyyMMdd-HHmmss') + ".exe")
            Copy-Item $src $dst -Force
            $lastExe = $dst
        }
    }
    # Also publish the parity host (plan 21 Part 04 Wave 2): `bun run parity` picks the newest
    # .diagnostics/parity/bin/parity_host-*.exe, so a good build here is used automatically.
    $hostOk = $null
    if ($ok) {
        $hostOut = & powershell -NoProfile -File (Join-Path $repo "scripts\parity\build-host.ps1") 2>&1
        $hostOk = [bool]($hostOut -match '^copied:')
    }
    @{ startedUtc = $started.ToString('o'); finishedUtc = [DateTime]::UtcNow.ToString('o'); exit = $(if ($ok) { 0 } else { 1 }); exe = $lastExe; parityHostOk = $hostOk; log = ".diagnostics/cargo/test-builder.err"; hostLog = ".diagnostics/cargo/parity-host-build.err" } |
        ConvertTo-Json | Set-Content $statusFile -Encoding utf8

    # Wait for the next source change after this build started.
    while ((Get-LatestChange) -le $started) { Start-Sleep -Seconds 10 }
    Start-Sleep -Seconds 15  # let a burst of edits settle
}
