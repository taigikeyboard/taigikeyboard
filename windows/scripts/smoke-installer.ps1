# Installs or uninstalls the release installer silently and reads back what
# the machine was left with: the registrations each architecture should have
# after an install, and nothing of them after an uninstall
# (docs/architecture/windows-arm64-roadmap.md section Verification). The windows-build
# CI jobs run it on an x64 and an Arm64 runner; it also runs on a dogfood VM.
# Typing is not exercised.
#
#   powershell -File smoke-installer.ps1 -Action Install -Installer <exe> [-Previous <exe>]
#   powershell -File smoke-installer.ps1 -Action Uninstall
#
# ASCII only: Windows PowerShell 5.1 reads a BOM-less script in the ANSI code
# page, where UTF-8 punctuation can decode to a curly quote that ends a string.

[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)] [ValidateSet('Install', 'Uninstall')] [string] $Action,
    # The installer under test.
    [string] $Installer,
    # An earlier release to install first, so -Installer runs as an upgrade.
    [string] $Previous
)

$ErrorActionPreference = 'Stop'
trap {
    Write-Host "smoke-installer.ps1 failed: $($_.Exception.Message)"
    Write-Host $_.InvocationInfo.PositionMessage
    exit 1
}

# TaigiKeyboard.iss AppId and install directory; guids.rs CLSID_TEXT_SERVICE.
$AppDir = Join-Path $env:ProgramFiles 'TaigiKeyboard'
$UninstallKey = 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\{6F0D8C2B-4A57-4E3F-9B1E-2D7C3A8E5F61}_is1'
$Clsid = '{32C28A51-8939-4C8F-8F29-037F9FD3CF0A}'
$Clsid64Key = "HKLM:\SOFTWARE\Classes\CLSID\$Clsid"
$Clsid32Key = "HKLM:\SOFTWARE\WOW6432Node\Classes\CLSID\$Clsid"
$TipKey = "HKLM:\SOFTWARE\Microsoft\CTF\TIP\$Clsid"
# The OS's architecture, not this process's, and what it should be left with.
$IsArm64 = (Get-ItemProperty 'HKLM:\SYSTEM\CurrentControlSet\Control\Session Manager\Environment').PROCESSOR_ARCHITECTURE -eq 'ARM64'
$SharedFiles = @('TaigiKeyboard.dll', 'TaigiKeyboard32.dll', 'TaigiKeyboardSettings.exe')
$Arm64Files = @('TaigiKeyboardArm64.dll', 'TaigiKeyboard64X.dll')
if ($IsArm64) {
    $MachineName = 'Arm64'
    $Machine = 'AA64'
    $NativeService = 'TaigiKeyboardArm64.dll'
    $InProcServer = 'TaigiKeyboard64X.dll'
} else {
    $MachineName = 'x64'
    $Machine = '8664'
    $NativeService = 'TaigiKeyboard.dll'
    $InProcServer = 'TaigiKeyboard.dll'
}

function Fail([string] $Message) { throw $Message }

# Twin of Get-PeMachine in install-dev.ps1 (scripts here share no module).
function Get-PeMachine([string] $Path) {
    $header = New-Object byte[] 4096
    $stream = [IO.File]::OpenRead($Path)
    try { [void] $stream.Read($header, 0, $header.Length) } finally { $stream.Dispose() }
    $peOffset = [BitConverter]::ToInt32($header, 0x3C)
    '{0:X4}' -f [BitConverter]::ToUInt16($header, $peOffset + 4)
}

$SessionManagerKey = 'HKLM:\SYSTEM\CurrentControlSet\Control\Session Manager'
# Maps a DLL as an image whatever its machine, without running it: the mapping
# a host's loaded copy makes, as far as deleting the file is concerned.
$LoadAsImageResource = 0x20
Add-Type -Namespace TaigiSmoke -Name Kernel32 -MemberDefinition @'
[DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
public static extern IntPtr LoadLibraryExW(string path, IntPtr file, uint flags);
[DllImport("kernel32.dll")]
public static extern bool FreeLibrary(IntPtr module);
'@

# PendingFileRenameOperations holds (source, target) pairs; a delete is a
# source with an empty target. Sources are NT paths (`\??\C:\...`); some carry
# a `*1` prefix (measured on Windows 11 Arm64, 26300).
function Test-PendingDelete([string[]] $Pending, [string] $Path) {
    for ($i = 0; $i + 1 -lt $Pending.Count; $i += 2) {
        if (($Pending[$i] -replace '^\*\d+', '') -eq "\??\$Path" -and $Pending[$i + 1] -eq '') { return $true }
    }
    $false
}

# What an uninstall still has to finish, or $null once it is done: the
# registrations gone, no file under its install name, and everything left in
# {app} (files a host holds, under their moved-aside names) plus {app} itself
# queued for deletion at restart.
function Get-UninstallShortfall {
    foreach ($key in $UninstallKey, $Clsid64Key, $Clsid32Key, $TipKey) {
        if (Test-Path $key) { return "$key is still there" }
    }
    foreach ($name in $SharedFiles + $Arm64Files) {
        if (Test-Path (Join-Path $AppDir $name)) { return "$name is still installed" }
    }
    if (-not (Test-Path $AppDir)) { return $null }
    $pending = @((Get-ItemProperty $SessionManagerKey -Name PendingFileRenameOperations -ErrorAction SilentlyContinue).PendingFileRenameOperations)
    foreach ($path in @(Get-ChildItem $AppDir -Recurse -Force -File | ForEach-Object { $_.FullName }) + $AppDir) {
        if (-not (Test-PendingDelete $pending $path)) { return "$path is left behind and not queued for deletion at restart" }
    }
    $null
}

function Get-InProcServer([string] $ClsidKey) {
    (Get-ItemProperty -Path "$ClsidKey\InProcServer32").'(default)'
}

function Invoke-Setup([string] $Exe) {
    $log = Join-Path $env:TEMP ("taigi-setup-" + [IO.Path]::GetFileNameWithoutExtension($Exe) + '.log')
    Write-Host "==> $Exe"
    $process = Start-Process $Exe -ArgumentList '/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', "/LOG=`"$log`"" -Wait -PassThru
    if ($process.ExitCode -ne 0) {
        Get-Content $log -Tail 30 -ErrorAction SilentlyContinue | Write-Host
        Fail "$Exe exited $($process.ExitCode) (log: $log)"
    }
}

function Assert-Equal([string] $What, $Actual, $Expected) {
    Write-Host "  $What = $Actual"
    if ($Actual -ne $Expected) { Fail "$What is '$Actual', expected '$Expected'" }
}

switch ($Action) {
    'Install' {
        if (-not $Installer) { Fail '-Installer is required for -Action Install' }
        if ($Previous) { Invoke-Setup $Previous }
        Invoke-Setup $Installer
        Write-Host "==> Reading back ($MachineName)"
        if (-not (Test-Path $UninstallKey)) { Fail "no uninstall entry at $UninstallKey" }
        if (-not (Test-Path $TipKey)) { Fail "no TSF registration at $TipKey" }
        Assert-Equal 'WOW6432Node InProcServer32' (Get-InProcServer $Clsid32Key) (Join-Path $AppDir 'TaigiKeyboard32.dll')
        Assert-Equal 'InProcServer32' (Get-InProcServer $Clsid64Key) (Join-Path $AppDir $InProcServer)
        # The shared TSF profile names whichever service registered last, which
        # must be the native 64-bit one (TaigiKeyboard.iss RegisterEverything).
        $profile = Get-ChildItem "$TipKey\LanguageProfile" -Recurse | Get-ItemProperty | Where-Object { $_.IconFile } | Select-Object -First 1
        Assert-Equal 'profile icon' $profile.IconFile (Join-Path $AppDir $NativeService)
        Assert-Equal 'settings exe machine' (Get-PeMachine (Join-Path $AppDir 'TaigiKeyboardSettings.exe')) $Machine
        Assert-Equal 'runtime machine' (Get-PeMachine (Join-Path $AppDir 'microsoft.ui.xaml.dll')) $Machine
        foreach ($name in $SharedFiles) {
            if (-not (Test-Path (Join-Path $AppDir $name))) { Fail "$name is not installed" }
        }
        foreach ($name in $Arm64Files) {
            $present = Test-Path (Join-Path $AppDir $name)
            if ($present -ne $IsArm64) { Fail "$name installed: $present, expected: $IsArm64" }
        }
        Write-Host 'OK  installed'
    }
    'Uninstall' {
        $uninstaller = Join-Path $AppDir 'unins000.exe'
        if (-not (Test-Path $uninstaller)) { Fail "no $uninstaller" }
        # Uninstall the way a user does: with a host still holding the 64-bit
        # service (and on Arm64 the forwarder) mapped. An image mapping blocks
        # the delete, so those files must be moved off their install names and
        # queued for the next restart, the directory after them
        # (TaigiKeyboard.iss DiscardLeftoverTree).
        $held = @($NativeService)
        if ($IsArm64) { $held += $InProcServer }
        $handles = foreach ($name in $held) {
            $handle = [TaigiSmoke.Kernel32]::LoadLibraryExW((Join-Path $AppDir $name), [IntPtr]::Zero, $LoadAsImageResource)
            if ($handle -eq [IntPtr]::Zero) { Fail "could not map $name" }
            $handle
        }
        Write-Host "==> $uninstaller (holding $($held -join ', '))"
        # The uninstaller hands over to a copy in %TEMP% and this process
        # returns early; that copy deletes unins000.exe and the uninstall key
        # BEFORE the [Code] cleanup runs, so neither marks the end. The outcome
        # itself is polled for instead.
        $process = Start-Process $uninstaller -ArgumentList '/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART' -Wait -PassThru
        if ($process.ExitCode -ne 0) { Fail "$uninstaller exited $($process.ExitCode)" }
        $deadline = (Get-Date).AddMinutes(3)
        while (($shortfall = Get-UninstallShortfall) -and (Get-Date) -lt $deadline) { Start-Sleep -Seconds 2 }
        if ($shortfall) { Fail "after uninstalling: $shortfall" }
        if (Test-Path $AppDir) {
            Get-ChildItem $AppDir -Recurse -Force -File | ForEach-Object { Write-Host "  queued for restart: $($_.FullName)" }
        }
        foreach ($handle in $handles) { [void] [TaigiSmoke.Kernel32]::FreeLibrary($handle) }
        Write-Host 'OK  uninstalled'
    }
}
