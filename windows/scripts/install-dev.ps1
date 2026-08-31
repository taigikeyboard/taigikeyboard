# The development install: build the text service from this checkout and
# register it, the Windows counterpart of `macos/scripts/install-app.sh`.
# NOT the shipping installer — that is `release-app.sh` + Inno Setup, which
# installs into Program Files, signs, and registers a scheduled task. This
# registers the build tree in place so a rebuild is one command.
#
#   .\install-dev.ps1 install     build, register, refresh the indicator
#   .\install-dev.ps1 uninstall   unregister (user data under %APPDATA% stays)
#   .\install-dev.ps1 reload      re-register what is already built
#
# Requires an ELEVATED shell: a text service is loaded into every process of
# every user, so `regsvr32` writes HKLM.
#
# PowerShell rather than the `.sh` its neighbours are written in: every step
# here is Win32-shaped — elevation, the registry, process enumeration, the
# input indicator — and a bash version would shell out to PowerShell for all
# of it. `release-app.sh` stays bash because its work (sha256, curl, gh,
# cygpath) is bash-shaped.

[CmdletBinding()]
param(
    [Parameter(Position = 0)]
    [ValidateSet('install', 'uninstall', 'reload')]
    [string] $Action = 'install'
)

$ErrorActionPreference = 'Stop'

# A 32-bit host sees a REDIRECTED registry: HKLM\SOFTWARE\Classes\CLSID
# resolves to WOW6432Node, where a 64-bit text service is not registered. Every
# read below would then be wrong — `uninstall` would report nothing registered
# and silently do nothing, and `install` would skip unregistering the copy the
# linker is about to overwrite. This is not hypothetical: GNU make from
# ezwinports is itself x86, so `make install` launches PowerShell out of
# SysWOW64. Observed, before this guard: `regsvr32` still put the keys in the
# 64-bit view while every read around it saw the redirected one, so the whole
# thing failed quietly.
#
# `Sysnative` is the alias a 32-bit process uses to reach the real System32.
if (-not [Environment]::Is64BitProcess) {
    $native = Join-Path $env:SystemRoot 'Sysnative\WindowsPowerShell\v1.0\powershell.exe'
    if (-not (Test-Path $native)) {
        Write-Error "running 32-bit and cannot find the native PowerShell at $native"
        exit 1
    }
    & $native -NoProfile -NonInteractive -ExecutionPolicy Bypass -File $PSCommandPath $Action
    exit $LASTEXITCODE
}

# Same identity as scripts/lib/identity.sh; kept in step by name, not shared,
# because that file is bash.
$RepositoryDir = Split-Path -Parent (Split-Path -Parent (Split-Path -Parent $PSCommandPath))
$WindowsDir = Join-Path $RepositoryDir 'windows'
$ReleaseTarget = 'x86_64-pc-windows-msvc'
$TargetDir = Join-Path $WindowsDir "target\$ReleaseTarget\release"
$ServiceDll = Join-Path $TargetDir 'TaigiKeyboard.dll'
# The linker writes `release\deps\TaigiKeyboard.dll` and cargo hard-links it
# into `release\`: TWO NAMES, ONE FILE. Moving one aside does not free the
# other, so a loaded dll still failed the link with LNK1104 until both were
# moved (observed 2026-08-30, fixed 2026-08-31). Every name the linker needs
# free, in the order the sweep visits them.
$ServiceDllNames = @($ServiceDll, (Join-Path $TargetDir 'deps\TaigiKeyboard.dll'))
$SettingsExe = Join-Path $TargetDir 'TaigiKeyboardSettings.exe'
$DictionariesSource = Join-Path $RepositoryDir 'ios\Resources\Dictionaries'
$FontsSource = Join-Path $RepositoryDir 'ios\Resources\Fonts'
# guids.rs: CLSID_TEXT_SERVICE.
$Clsid = '{32C28A51-8939-4C8F-8F29-037F9FD3CF0A}'
$ClsidKey = "HKLM:\SOFTWARE\Classes\CLSID\$Clsid"

function Fail([string] $Message) {
    Write-Error $Message
    exit 1
}

function Assert-Elevated {
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = [Security.Principal.WindowsPrincipal] $identity
    if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
        Fail @'
this needs an elevated shell — regsvr32 writes HKLM.
  Start menu -> PowerShell -> right-click -> Run as administrator, then re-run.
'@
    }
}

# The path the registry currently names, which after a branch switch may not
# be the one we are about to build.
function Get-RegisteredDll {
    $key = Join-Path $ClsidKey 'InProcServer32'
    if (-not (Test-Path $key)) { return $null }
    (Get-ItemProperty -Path $key -ErrorAction SilentlyContinue).'(default)'
}

function Invoke-Regsvr32([string] $Path, [switch] $Unregister) {
    # By full native path: the guard above guarantees this process is 64-bit, so
    # System32 is the real one, and nothing here depends on PATH lookup.
    $regsvr32 = Join-Path $env:SystemRoot 'System32\regsvr32.exe'
    $arguments = @('/s')
    if ($Unregister) { $arguments += '/u' }
    $arguments += "`"$Path`""
    $process = Start-Process $regsvr32 -ArgumentList $arguments -Wait -PassThru
    return $process.ExitCode
}

function Get-DllHolders([string] $Path) {
    Get-Process | Where-Object {
        $_.Modules 2>$null | Where-Object { $_.FileName -eq $Path }
    }
}

# Windows lets a LOADED dll be renamed even though it cannot be deleted, which
# is how the linker gets to write a fresh one without asking anybody to close
# anything. The installer plays the same trick (TaigiKeyboard.iss, the rename
# lock probe). The stamp matters: an earlier set-aside copy can still be held,
# and renaming onto its name fails.
function Clear-DllName([string] $Path) {
    # Copies an earlier run could not delete because something still had them
    # open. Whatever is holding one may have exited since.
    Get-ChildItem "$Path.locked.*" -ErrorAction SilentlyContinue | ForEach-Object {
        try {
            Remove-Item $_.FullName -Force -ErrorAction Stop
            Write-Host "  swept $($_.Name)"
        } catch {
            Write-Host "  left in place, $($_.Name): $($_.Exception.Message)"
        }
    }
    if (-not (Test-Path $Path)) { return }

    # Set aside unconditionally rather than probing first. There is no cheap,
    # honest test for "can the linker replace this" — renaming the file to its
    # own name proves nothing, since the provider may treat it as a no-op or
    # refuse it without ever testing the sharing mode. So: rename (which
    # Windows allows even for a loaded dll, the trick TaigiKeyboard.iss uses
    # too), then try to delete the set-aside copy. Deleting succeeds exactly
    # when nothing held it, which both frees the name for the linker and
    # leaves no residue in the ordinary case.
    $aside = (Split-Path $Path -Leaf) + '.locked.' + [Guid]::NewGuid().ToString('N').Substring(0, 8)
    try {
        Rename-Item $Path $aside -ErrorAction Stop
    } catch {
        $holders = (Get-DllHolders $Path | ForEach-Object { "$($_.ProcessName) ($($_.Id))" }) -join ', '
        Fail "could not move $Path aside: $($_.Exception.Message). Held by: $holders"
    }
    $asidePath = Join-Path (Split-Path $Path -Parent) $aside
    try {
        Remove-Item $asidePath -Force -ErrorAction Stop
    } catch {
        $holders = (Get-DllHolders $asidePath | ForEach-Object { "$($_.ProcessName) ($($_.Id))" }) -join ', '
        Write-Host "  previous build still loaded by $holders; kept as $aside"
    }
}

# Frees every name the linker writes the service dll through. A host that
# already mapped the old dll keeps running the OLD code — no installer can
# reach into another process — but the names being free is what lets the
# build produce a new one at all, so a host started afterwards gets it.
function Clear-BuildOutput {
    foreach ($name in $ServiceDllNames) {
        Clear-DllName $name
    }
}

function Build-Service {
    Write-Host '==> Building'
    Push-Location $WindowsDir
    try {
        # The release script's variable, so the icon and VERSIONINFO are a hard
        # requirement here too rather than a warning.
        $env:TAIGI_REQUIRE_RESOURCES = '1'
        & cargo build --release --target $ReleaseTarget
        if ($LASTEXITCODE -ne 0) { Fail 'cargo build failed' }
    } finally {
        Pop-Location
    }
}

# The dll resolves these from its OWN directory, so a development install has
# to reproduce the layout release-app.sh stages.
function Copy-Resources {
    Write-Host '==> Staging dictionaries and fonts beside the dll'
    $dictionaries = Join-Path $TargetDir 'Dictionaries'
    $fonts = Join-Path $TargetDir 'Fonts'
    New-Item -ItemType Directory -Force -Path $dictionaries, $fonts | Out-Null
    # A missing syllables.fst is a corrupt install, not an optional degrade
    # (dictionary_artifacts.rs).
    foreach ($artifact in 'dictionary.fst', 'dictionary.bin', 'association.bin', 'syllables.fst') {
        $source = Join-Path $DictionariesSource $artifact
        if (-not (Test-Path $source) -or (Get-Item $source).Length -eq 0) {
            Fail "missing or empty dictionary artifact $source (run 'make dict' + 'make build' on the Mac and pull)"
        }
        Copy-Item $source $dictionaries -Force
    }
    $faces = Get-ChildItem $FontsSource -Include *.ttf, *.otf -Recurse -ErrorAction SilentlyContinue
    if (-not $faces) { Fail "no font files in $FontsSource" }
    $faces | Copy-Item -Destination $fonts -Force
    Write-Host "  4 dictionary artifacts, $($faces.Count) fonts"
}

# The indicator caches what it drew; restarting the process that draws it is
# what makes a re-registration visible without signing out. Windows restarts it
# on its own.
#
# Only THIS session's. The script is elevated, so an unfiltered `Get-Process
# ctfmon` reaches every logged-on user — another account's or another RDP
# session's input indicator is not a dev install's to restart.
function Restart-InputIndicator {
    $session = (Get-Process -Id $PID).SessionId
    $running = Get-Process ctfmon -ErrorAction SilentlyContinue |
        Where-Object { $_.SessionId -eq $session }
    if (-not $running) {
        # Normal when this is driven over SSH: OpenSSH runs in session 0 and the
        # desktop is session 1, so there is no indicator of ours to restart.
        Write-Host "  no input indicator in session $session; the taskbar will catch up on its own"
        return
    }
    Stop-Process -InputObject $running -Force
    Start-Sleep -Seconds 2
}

function Unregister-Service {
    $registered = Get-RegisteredDll
    if (-not $registered) {
        Write-Host '  nothing registered'
        return
    }
    if (-not (Test-Path $registered)) {
        # Deleting the keys by hand looks equivalent and is not: registration.rs
        # also unregisters four TSF categories through ITfCategoryMgr, and those
        # live wherever TSF decides to keep them. Guessing at more keys would
        # leave an asymmetric registration while reporting success, so say what
        # is wrong instead.
        Fail @"
the registered dll is gone, so it cannot unregister itself:
  $registered
Restore or rebuild that file, run this again, and it will unregister cleanly.
"@
    }
    $code = Invoke-Regsvr32 $registered -Unregister
    # Judged by outcome, not by exit code. `DllUnregisterServer` reports failure
    # when a piece of the registration was already gone — TSF's UnregisterProfile
    # fails on an absent profile and registration.rs propagates that — and a dev
    # tool must not abort because it removed something that was half-removed.
    # What matters is whether the keys are gone now.
    if (Test-Path $ClsidKey) {
        Fail "regsvr32 /u exited $code and $ClsidKey is still there"
    }
    if ($code -ne 0) {
        Write-Host "  unregistered $registered (regsvr32 reported $code; the keys are gone)"
    } else {
        Write-Host "  unregistered $registered"
    }
}

switch ($Action) {
    'install' {
        Assert-Elevated
        Write-Host '==> Unregistering the current build'
        # Before the build, not after: this stops new processes loading the dll,
        # so fewer of them are holding it when the linker wants to replace it.
        Unregister-Service
        Write-Host '==> Freeing the build output'
        Clear-BuildOutput
        Build-Service
        if (-not (Test-Path $ServiceDll)) { Fail "the build produced no $ServiceDll" }
        if (-not (Test-Path $SettingsExe)) { Fail "the build produced no $SettingsExe" }
        Copy-Resources
        Write-Host '==> Registering'
        $code = Invoke-Regsvr32 $ServiceDll
        if ($code -ne 0) { Fail "regsvr32 exited $code" }
        Restart-InputIndicator
        Write-Host ''
        Write-Host "OK  registered $ServiceDll"
        Write-Host '    Win+Space to switch to it. Settings is Ctrl+Shift+S while it is active.'
    }
    'uninstall' {
        Assert-Elevated
        Write-Host '==> Unregistering'
        Unregister-Service
        Restart-InputIndicator
        Write-Host ''
        Write-Host 'OK  unregistered.'
        Write-Host '    Your dictionaries and settings under %APPDATA%\TaigiKeyboard are untouched.'
    }
    'reload' {
        Assert-Elevated
        if (-not (Test-Path $ServiceDll)) { Fail "nothing built at $ServiceDll — run 'make install' first" }
        Write-Host '==> Re-registering the existing build'
        Unregister-Service
        $code = Invoke-Regsvr32 $ServiceDll
        if ($code -ne 0) { Fail "regsvr32 exited $code" }
        Restart-InputIndicator
        Write-Host 'OK  re-registered (no rebuild).'
    }
}
