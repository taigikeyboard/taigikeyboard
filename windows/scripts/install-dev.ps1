# The development install: build the text service from this checkout and
# register it, the Windows counterpart of `macos/scripts/install-app.sh`.
# NOT the shipping installer - that is `release-app.sh` + Inno Setup, which
# installs into Program Files, signs, and registers a scheduled task. This
# registers the build tree in place so a rebuild is one command.
#
# On an Arm64 machine it builds the Arm64 and the x64 text service, links the
# Arm64X forwarder between them and registers a staged copy under
# `target\arm64x-dev` (docs/architecture/windows-arm64-roadmap.md section Install
# layout) - cargo writes both DLLs under one name, so they cannot sit side by
# side in a target directory.
#
#   .\install-dev.ps1 install     build, register, refresh the indicator
#   .\install-dev.ps1 uninstall   unregister (user data under %APPDATA% stays)
#   .\install-dev.ps1 unlock      free the dll names the linker writes through,
#                                 so a release build can link over a dev install
#   .\install-dev.ps1 reload      re-register what is already built
#
# Requires an ELEVATED shell: a text service is loaded into every process of
# every user, so `regsvr32` writes HKLM.
#
# PowerShell rather than the `.sh` its neighbours are written in: every step
# here is Win32-shaped - elevation, the registry, process enumeration, the
# input indicator - and a bash version would shell out to PowerShell for all
# of it. `release-app.sh` stays bash because its work (sha256, curl, gh,
# cygpath) is bash-shaped.

[CmdletBinding()]
param(
    [Parameter(Position = 0)]
    [ValidateSet('install', 'uninstall', 'reload', 'unlock')]
    [string] $Action = 'install'
)

$ErrorActionPreference = 'Stop'

# A 32-bit host sees a REDIRECTED registry: HKLM\SOFTWARE\Classes\CLSID
# resolves to WOW6432Node, where a 64-bit text service is not registered. Every
# read below would then be wrong - `uninstall` would report nothing registered
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
$X64Target = 'x86_64-pc-windows-msvc'
# The OS's architecture, not this process's: an x64-emulated PowerShell
# reports AMD64 in its own environment.
$Machine = (Get-ItemProperty 'HKLM:\SYSTEM\CurrentControlSet\Control\Session Manager\Environment').PROCESSOR_ARCHITECTURE
$IsArm64 = $Machine -eq 'ARM64'
# registration.rs FORWARDER_DLL; build.rs's Arm64 OriginalFilename.
$ForwarderName = 'TaigiKeyboard64X.dll'
$Arm64ServiceName = 'TaigiKeyboardArm64.dll'
$ReleaseTarget = if ($IsArm64) { 'aarch64-pc-windows-msvc' } else { $X64Target }
$TargetDir = Join-Path $WindowsDir "target\$ReleaseTarget\release"
if ($IsArm64) {
    # What is registered and loaded: staged copies under their install names.
    $InstallDir = Join-Path $WindowsDir 'target\arm64x-dev'
    $ServiceDll = Join-Path $InstallDir $Arm64ServiceName
    $X64ServiceDll = Join-Path $InstallDir 'TaigiKeyboard.dll'
    # Hosts map the staged copies, never cargo's output, so these are the
    # names to free before staging again.
    $ServiceDllNames = @($ServiceDll, $X64ServiceDll, (Join-Path $InstallDir $ForwarderName))
} else {
    $InstallDir = $TargetDir
    $ServiceDll = Join-Path $TargetDir 'TaigiKeyboard.dll'
    # The linker writes `release\deps\TaigiKeyboard.dll` and cargo hard-links it
    # into `release\`: TWO NAMES, ONE FILE. Moving one aside does not free the
    # other, so a loaded dll still failed the link with LNK1104 until both were
    # moved (observed 2026-08-30, fixed 2026-08-31). Every name the linker needs
    # free, in the order the sweep visits them.
    $ServiceDllNames = @($ServiceDll, (Join-Path $TargetDir 'deps\TaigiKeyboard.dll'))
}
$SettingsExe = Join-Path $InstallDir 'TaigiKeyboardSettings.exe'
# The one committed copy every platform packages from (#14, `f4e0fb20`);
# it used to be reached through the iOS bundle's own folder, which no longer
# exists.
$DictionariesSource = Join-Path $RepositoryDir 'assets\dictionaries'
$FontsSource = Join-Path $RepositoryDir 'assets\fonts\font'
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
this needs an elevated shell - regsvr32 writes HKLM.
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
# anything. A memory-mapped data file is the same: a host with the dictionary
# open makes overwriting it fail ("the requested operation cannot be performed
# on a file with a user-mapped section open") while a rename goes through. The
# installer plays the same trick (TaigiKeyboard.iss, MakeWay in [Code]). The
# stamp matters: an earlier set-aside copy can still be held, and renaming onto
# its name fails.
function Clear-FileName([string] $Path) {
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
    # honest test for "can the linker replace this" - renaming the file to its
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
        Write-Host "  previous copy still held$(if ($holders) { " by $holders" }); kept as $aside"
    }
}

# Frees every name the linker writes the service dll through. A host that
# already mapped the old dll keeps running the OLD code - no installer can
# reach into another process - but the names being free is what lets the
# build produce a new one at all, so a host started afterwards gets it.
function Clear-BuildOutput {
    foreach ($name in $ServiceDllNames) {
        Clear-FileName $name
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
        if ($IsArm64) {
            # The text service only: x64-emulated hosts load it through the
            # forwarder, while the settings window runs as the Arm64 exe.
            & cargo build --release --target $X64Target -p taigi-windows-tsf
            if ($LASTEXITCODE -ne 0) { Fail "cargo build --target $X64Target failed" }
        }
    } finally {
        Pop-Location
    }
}

# The dll resolves these from its OWN directory, so a development install has
# to reproduce the layout release-app.sh stages. A re-run finds the previous
# copies still mapped by every host the service ran in, so each one is moved
# aside before its replacement is written (Clear-FileName).
function Copy-Resources {
    Write-Host '==> Staging dictionaries and fonts beside the dll'
    $dictionaries = Join-Path $InstallDir 'Dictionaries'
    $fonts = Join-Path $InstallDir 'Fonts'
    New-Item -ItemType Directory -Force -Path $dictionaries, $fonts | Out-Null
    # A missing syllables.fst is a corrupt install, not an optional degrade
    # (dictionary_artifacts.rs).
    foreach ($artifact in 'dictionary.fst', 'dictionary.bin', 'association.bin', 'syllables.fst') {
        $source = Join-Path $DictionariesSource $artifact
        if (-not (Test-Path $source) -or (Get-Item $source).Length -eq 0) {
            Fail "missing or empty dictionary artifact $source (run 'make dict' + 'make build' on the Mac and pull)"
        }
        Clear-FileName (Join-Path $dictionaries $artifact)
        Copy-Item $source $dictionaries -Force
    }
    $faces = Get-ChildItem $FontsSource -Include *.ttf, *.otf -Recurse -ErrorAction SilentlyContinue
    if (-not $faces) { Fail "no font files in $FontsSource" }
    foreach ($face in $faces) {
        Clear-FileName (Join-Path $fonts $face.Name)
        Copy-Item $face.FullName $fonts -Force
    }
    Write-Host "  4 dictionary artifacts, $($faces.Count) fonts"
}

# The PE machine field of a binary, as dumpbin prints it (`AA64`, `8664`).
# Twin of Get-PeMachine in smoke-installer.ps1 (scripts here share no module).
function Get-PeMachine([string] $Path) {
    $header = New-Object byte[] 4096
    $stream = [IO.File]::OpenRead($Path)
    try { [void] $stream.Read($header, 0, $header.Length) } finally { $stream.Dispose() }
    $peOffset = [BitConverter]::ToInt32($header, 0x3C)
    '{0:X4}' -f [BitConverter]::ToUInt16($header, $peOffset + 4)
}

# The Arm64 install layout (roadmap section Install layout): both text services
# under their install names, the forwarder between them, and the Arm64
# settings exe with the Windows App Runtime it loads from its own directory -
# the text service starts the exe from the DLL's directory (module.rs).
function Copy-Arm64Install {
    Write-Host '==> Staging the Arm64 install'
    New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
    $staged = [ordered] @{
        (Join-Path $TargetDir 'TaigiKeyboard.dll') = $ServiceDll
        (Join-Path $WindowsDir "target\$X64Target\release\TaigiKeyboard.dll") = $X64ServiceDll
        (Join-Path $TargetDir 'TaigiKeyboardSettings.exe') = $SettingsExe
    }
    foreach ($built in $staged.Keys) {
        if (-not (Test-Path $built)) { Fail "the build produced no $built" }
        Copy-Item $built $staged[$built] -Force
    }
    $runtimeList = Join-Path $WindowsDir 'build-support\windows-app-runtime-files.txt'
    foreach ($name in Get-Content $runtimeList) {
        $name = $name.Trim()
        if (-not $name -or $name.StartsWith('#')) { continue }
        $source = Join-Path $TargetDir $name
        if (-not (Test-Path $source)) { Fail "missing Windows App Runtime file $name in $TargetDir (windows-reactor-setup staging)" }
        Copy-Item $source $InstallDir -Recurse -Force
    }
    # windows-reactor-setup caches the extracted runtime once per machine, not
    # per architecture: after an x64 settings build it stages x64 DLLs beside
    # the Arm64 exe, and the window never opens.
    $xaml = Join-Path $InstallDir 'microsoft.ui.xaml.dll'
    $machine = Get-PeMachine $xaml
    if ($machine -ne 'AA64') {
        Fail @"
$xaml is PE machine $machine, not AA64: windows-reactor-setup reused an x64 runtime extraction.
Delete the .msix_extract directory under $env:LOCALAPPDATA\windows-reactor-setup and run this again.
"@
    }
    # Its own process: it enters a Visual Studio developer environment, which
    # would otherwise stay in this one.
    & powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot 'build-forwarder.ps1') -OutDir $InstallDir
    if ($LASTEXITCODE -ne 0) { Fail "build-forwarder.ps1 exited $LASTEXITCODE" }
}

# The indicator caches what it drew; restarting the process that draws it is
# what makes a re-registration visible without signing out. Windows restarts it
# on its own.
#
# Only THIS session's. The script is elevated, so an unfiltered `Get-Process
# ctfmon` reaches every logged-on user - another account's or another RDP
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

# A running settings window holds its own exe open, and cargo replaces that
# exe on every build - so leaving it up fails the link with `os error 5`
# (access denied), an error that names neither the file nor the holder. The
# installer stops it for the same reason (TaigiKeyboard.iss,
# `StopSettingsWindow` in PrepareToInstall).
#
# Matched by PATH, not by name or session: what has to go is whatever runs
# THIS build output. An installed copy under Program Files is somebody else's
# window and stays up, while the desktop's own window is stopped even when
# this script is driven over SSH - session 0 there, session 1 for the window,
# so a session filter would spare the one process actually in the way.
function Stop-SettingsWindow {
    $running = Get-Process -Name ([System.IO.Path]::GetFileNameWithoutExtension($SettingsExe)) -ErrorAction SilentlyContinue |
        Where-Object {
            try { $_.MainModule.FileName -eq $SettingsExe } catch { $false }
        }
    foreach ($process in $running) {
        Stop-Process -InputObject $process -Force -ErrorAction SilentlyContinue
        Write-Host "  closed the settings window ($($process.Id))"
    }
}

function Unregister-Service {
    $registered = Get-RegisteredDll
    if (-not $registered) {
        Write-Host '  nothing registered'
        return
    }
    # On Arm64 the registry names the forwarder, which exports no
    # DllUnregisterServer: the Arm64 DLL beside it registered it and is the one
    # that unregisters it.
    if ((Split-Path $registered -Leaf) -eq $ForwarderName) {
        $registered = Join-Path (Split-Path $registered -Parent) $Arm64ServiceName
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
    # when a piece of the registration was already gone - TSF's UnregisterProfile
    # fails on an absent profile and registration.rs propagates that - and a dev
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
        Stop-SettingsWindow
        Clear-BuildOutput
        Build-Service
        if ($IsArm64) { Copy-Arm64Install }
        if (-not (Test-Path $ServiceDll)) { Fail "the build produced no $ServiceDll" }
        if (-not (Test-Path $SettingsExe)) { Fail "the build produced no $SettingsExe" }
        Copy-Resources
        Write-Host '==> Registering'
        $code = Invoke-Regsvr32 $ServiceDll
        if ($code -ne 0) { Fail "regsvr32 exited $code" }
        Restart-InputIndicator
        Write-Host ''
        Write-Host "OK  registered $ServiceDll"
        Write-Host '    Win+Space to switch to it. Settings is Ctrl+Alt+S while it is active.'
    }
    'unlock' {
        # No registration change and no build: just the names. A release build
        # runs from the same tree the dev install is registered from, and a host
        # that has the old dll mapped stops the linker replacing it.
        if ($IsArm64) {
            # Hosts map the staged copies under target\arm64x-dev, which the
            # linker never writes; freeing them would only break the install.
            Write-Host 'OK  nothing to free: an Arm64 dev install runs staged copies.'
            return
        }
        Write-Host '==> Freeing the build output'
        Stop-SettingsWindow
        Clear-BuildOutput
        Write-Host ''
        Write-Host 'OK  the linker can write the service dll.'
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
        if (-not (Test-Path $ServiceDll)) { Fail "nothing built at $ServiceDll - run 'make install' first" }
        Write-Host '==> Re-registering the existing build'
        Unregister-Service
        $code = Invoke-Regsvr32 $ServiceDll
        if ($code -ne 0) { Fail "regsvr32 exited $code" }
        Restart-InputIndicator
        Write-Host 'OK  re-registered (no rebuild).'
    }
}
