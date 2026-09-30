# The one way to run cargo in this repo, for people and agents alike (user request 2026-09-29).
# - One cargo at a time, machine-wide: a named mutex makes a second caller wait for the first
#   instead of running beside it (two parallel cargo builds froze the PC on 2026-09-27).
# - BelowNormal priority, inherited by every rustc/linker child, so the desktop stays responsive.
# - Output goes to .diagnostics/cargo/<log>.out / .err (gitignored); the exit code is printed last.
# - Queued `check` runs coalesce: a caller that waited for the lock reuses the result of an
#   identical `cargo check` that *started after* it asked (that run already saw its edits), instead
#   of running the same check again. Other commands (build/test) always run.
#
# Usage (from anywhere):
#   powershell -NoProfile -File scripts/cargo-safe.ps1 <log-name> <cargo args...>
#   e.g. scripts/cargo-safe.ps1 check check --manifest-path src-tauri/Cargo.toml --workspace --all-targets
param(
    [Parameter(Mandatory = $true)][string]$log,
    [Parameter(ValueFromRemainingArguments = $true)][string[]]$cargoArgs
)

$repo = Split-Path -Parent $PSScriptRoot
$logDir = Join-Path $repo ".diagnostics\cargo"
New-Item -ItemType Directory -Force $logDir | Out-Null
Set-Location $repo

$argKey = ($cargoArgs -join ' ')
$isCheck = $cargoArgs.Count -gt 0 -and $cargoArgs[0] -eq 'check'
$lastCheckFile = Join-Path $logDir "last-check.json"
$requestedAt = [DateTime]::UtcNow

$mutex = New-Object System.Threading.Mutex($false, "Global\equal-cargo")
$waited = [Diagnostics.Stopwatch]::StartNew()
try {
    try { [void]$mutex.WaitOne() } catch [System.Threading.AbandonedMutexException] { }
    if ($waited.Elapsed.TotalSeconds -ge 1) { "waited $([int]$waited.Elapsed.TotalSeconds)s for another cargo run" }

    if ($isCheck -and (Test-Path $lastCheckFile)) {
        $last = Get-Content $lastCheckFile -Raw | ConvertFrom-Json
        if ($last.args -eq $argKey -and [DateTime]::Parse($last.startedUtc).ToUniversalTime() -gt $requestedAt) {
            Copy-Item (Join-Path $logDir "$($last.log).err") (Join-Path $logDir "$log.err") -Force
            Copy-Item (Join-Path $logDir "$($last.log).out") (Join-Path $logDir "$log.out") -Force
            "reused an identical check that started after this request (log '$($last.log)')"
            "log: .diagnostics/cargo/$log.err"
            "exit=$($last.exit)"
            return
        }
    }

    $startedUtc = [DateTime]::UtcNow
    $sw = [Diagnostics.Stopwatch]::StartNew()
    $p = Start-Process -FilePath "cargo" -ArgumentList $cargoArgs -NoNewWindow -PassThru `
        -RedirectStandardError (Join-Path $logDir "$log.err") -RedirectStandardOutput (Join-Path $logDir "$log.out")
    $null = $p.Handle  # cache the handle now, or ExitCode reads back empty after WaitForExit
    try { $p.PriorityClass = 'BelowNormal' } catch { }
    $p.WaitForExit()
    if ($isCheck) {
        @{ args = $argKey; startedUtc = $startedUtc.ToString('o'); log = $log; exit = $p.ExitCode } |
            ConvertTo-Json | Set-Content $lastCheckFile -Encoding utf8
    }
    "took $([int]$sw.Elapsed.TotalSeconds)s"
    "log: .diagnostics/cargo/$log.err"
    "exit=$($p.ExitCode)"
}
finally {
    $mutex.ReleaseMutex()
    $mutex.Dispose()
}
