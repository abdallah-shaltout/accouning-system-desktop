# Builds the Rust parity host (plan 21 Part 04 B-4, `src-tauri/src/bin/parity_host.rs`) through the
# one cargo wrapper (scripts/cargo-safe.ps1: machine-wide lock, low priority), then copies the exe to
# `.diagnostics/parity/bin/parity_host-<stamp>.exe`, where `bun run parity` picks the newest one
# (scripts/parity/transport.ts `newestHostExe`). A stamped copy, not the target/ file itself, so a
# later cargo build can never swap the exe under a running parity pass.
#
# Usage (from anywhere):
#   powershell -NoProfile -File scripts/parity/build-host.ps1            # build + copy
#   powershell -NoProfile -File scripts/parity/build-host.ps1 -CopyOnly  # copy the last build only
param([switch]$CopyOnly)

$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)

if (-not $CopyOnly) {
    $out = & powershell -NoProfile -File (Join-Path $repo 'scripts\cargo-safe.ps1') parity-host-build `
        build --manifest-path src-tauri/Cargo.toml --bin parity_host --features parity
    $out | ForEach-Object { $_ }
    if (-not ($out -match '^exit=0$')) {
        Write-Error 'parity_host build failed - see .diagnostics/cargo/parity-host-build.err'
    }
}

$exe = Join-Path $repo 'src-tauri\target\debug\parity_host.exe'
if (-not (Test-Path $exe)) { Write-Error "no built host at $exe" }

$binDir = Join-Path $repo '.diagnostics\parity\bin'
New-Item -ItemType Directory -Force $binDir | Out-Null
$dest = Join-Path $binDir ("parity_host-{0}.exe" -f (Get-Date -Format 'yyyyMMdd-HHmmss'))
Copy-Item $exe $dest
"copied: $dest"
