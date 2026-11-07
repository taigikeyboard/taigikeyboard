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
# The OS's architecture, not this process's.
$IsArm64 = (Get-ItemProperty 'HKLM:\SYSTEM\CurrentControlSet\Control\Session Manager\Environment').PROCESSOR_ARCHITECTURE -eq 'ARM64'

function Fail([string] $Message) { throw $Message }

function Get-PeMachine([string] $Path) {
    $header = New-Object byte[] 4096
    $stream = [IO.File]::OpenRead($Path)
    try { [void] $stream.Read($header, 0, $header.Length) } finally { $stream.Dispose() }
    $peOffset = [BitConverter]::ToInt32($header, 0x3C)
    '{0:X4}' -f [BitConverter]::ToUInt16($header, $peOffset + 4)
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
        Write-Host "==> Reading back ($(if ($IsArm64) { 'Arm64' } else { 'x64' }))"
        if (-not (Test-Path $UninstallKey)) { Fail "no uninstall entry at $UninstallKey" }
        if (-not (Test-Path $TipKey)) { Fail "no TSF registration at $TipKey" }
        Assert-Equal 'WOW6432Node InProcServer32' (Get-InProcServer $Clsid32Key) (Join-Path $AppDir 'TaigiKeyboard32.dll')
        $native = if ($IsArm64) { 'TaigiKeyboard64X.dll' } else { 'TaigiKeyboard.dll' }
        Assert-Equal 'InProcServer32' (Get-InProcServer $Clsid64Key) (Join-Path $AppDir $native)
        # The shared TSF profile names whichever service registered last, which
        # must be the native 64-bit one (TaigiKeyboard.iss RegisterEverything).
        $nativeService = if ($IsArm64) { 'TaigiKeyboardArm64.dll' } else { 'TaigiKeyboard.dll' }
        $profile = Get-ChildItem "$TipKey\LanguageProfile" -Recurse | Get-ItemProperty | Where-Object { $_.IconFile } | Select-Object -First 1
        Assert-Equal 'profile icon' $profile.IconFile (Join-Path $AppDir $nativeService)
        $machine = if ($IsArm64) { 'AA64' } else { '8664' }
        Assert-Equal 'settings exe machine' (Get-PeMachine (Join-Path $AppDir 'TaigiKeyboardSettings.exe')) $machine
        Assert-Equal 'runtime machine' (Get-PeMachine (Join-Path $AppDir 'microsoft.ui.xaml.dll')) $machine
        foreach ($name in 'TaigiKeyboard.dll', 'TaigiKeyboard32.dll') {
            if (-not (Test-Path (Join-Path $AppDir $name))) { Fail "$name is not installed" }
        }
        foreach ($name in 'TaigiKeyboardArm64.dll', 'TaigiKeyboard64X.dll') {
            $present = Test-Path (Join-Path $AppDir $name)
            if ($present -ne $IsArm64) { Fail "$name installed: $present, expected: $IsArm64" }
        }
        Write-Host 'OK  installed'
    }
    'Uninstall' {
        $uninstaller = Join-Path $AppDir 'unins000.exe'
        if (-not (Test-Path $uninstaller)) { Fail "no $uninstaller" }
        Write-Host "==> $uninstaller"
        # The uninstaller runs from a copy in %TEMP% and ends this process when
        # it is done; the entry going away is waited on too, as the outcome.
        $process = Start-Process $uninstaller -ArgumentList '/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART' -Wait -PassThru
        if ($process.ExitCode -ne 0) { Fail "$uninstaller exited $($process.ExitCode)" }
        $deadline = (Get-Date).AddMinutes(3)
        while ((Test-Path $UninstallKey) -and (Get-Date) -lt $deadline) { Start-Sleep -Seconds 2 }
        foreach ($key in $UninstallKey, $Clsid64Key, $Clsid32Key, $TipKey) {
            if (Test-Path $key) { Fail "$key is still there after uninstalling" }
        }
        foreach ($name in 'TaigiKeyboard.dll', 'TaigiKeyboard32.dll', 'TaigiKeyboardArm64.dll', 'TaigiKeyboard64X.dll', 'TaigiKeyboardSettings.exe') {
            if (Test-Path (Join-Path $AppDir $name)) { Fail "$name is still installed" }
        }
        Write-Host 'OK  uninstalled'
    }
}
