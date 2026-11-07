# Links the Arm64X pure forwarder TaigiKeyboard64X.dll - the DLL an Arm64
# machine registers as InProcServer32, so Arm64-native processes load
# TaigiKeyboardArm64.dll and x64-emulated ones load TaigiKeyboard.dll through
# the one path both read (docs/architecture/windows-arm64-roadmap.md
# section Forwarder). mozc's recipe: references/mozc/src/win32/tip/build_tip_forwarder_dll.py.
#
#   .\build-forwarder.ps1 -OutDir <directory>
#
# Used by install-dev.ps1 (Arm64 development install) and, from Phase 3, by
# release-app.sh. Needs Visual Studio 2022 with the "MSVC ARM64/ARM64EC build
# tools" component and a Windows SDK; runs on an x64 or an Arm64 host.

[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string] $OutDir
)

$ErrorActionPreference = 'Stop'

$WindowsDir = Split-Path -Parent $PSScriptRoot
$ForwarderSources = Join-Path $WindowsDir 'forwarder'
# registration.rs FORWARDER_DLL; scripts/lib/identity.sh FORWARDER_DLL.
$ForwarderDll = 'TaigiKeyboard64X.dll'
# Resolved now: the build below runs in a temporary directory.
$OutDir = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($OutDir)

function Fail([string] $Message) {
    Write-Error $Message
    exit 1
}

function Invoke-Tool([string] $Tool, [string[]] $Arguments) {
    & $Tool @Arguments
    if ($LASTEXITCODE -ne 0) { Fail "$Tool exited $LASTEXITCODE" }
}

# The workspace version (`[workspace.package]` in windows/Cargo.toml), the one
# every other Windows binary's VERSIONINFO carries - the PowerShell twin of
# SHORT_VERSION in scripts/lib/identity.sh.
function Get-WorkspaceVersion {
    $inPackage = $false
    foreach ($line in Get-Content (Join-Path $WindowsDir 'Cargo.toml')) {
        if ($line -match '^\s*\[') {
            $inPackage = $line -match '^\s*\[workspace\.package\]'
            continue
        }
        if ($inPackage -and $line -match '^\s*version\s*=\s*"(\d+)\.(\d+)\.(\d+)"') {
            return @($Matches[1], $Matches[2], $Matches[3])
        }
    }
    Fail 'no [workspace.package] version in windows/Cargo.toml'
}

# cl / link / rc / dumpbin for an Arm64 target, in this process.
function Enter-Arm64DevShell {
    $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
    if (-not (Test-Path $vswhere)) { Fail "no vswhere at ${vswhere}; install Visual Studio 2022 (Build Tools)" }
    $vs = & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.ARM64EC -property installationPath
    if (-not $vs) { Fail 'no Visual Studio with the "MSVC ARM64/ARM64EC build tools" component' }
    # The OS's architecture, not this process's: an x64-emulated PowerShell
    # reports AMD64 in its own environment.
    $machine = (Get-ItemProperty 'HKLM:\SYSTEM\CurrentControlSet\Control\Session Manager\Environment').PROCESSOR_ARCHITECTURE
    $hostArch = if ($machine -eq 'ARM64') { 'arm64' } else { 'amd64' }
    Import-Module (Join-Path $vs 'Common7\Tools\Microsoft.VisualStudio.DevShell.dll')
    # `-HostArch` accepts only x86 / amd64 (VS 17.14), so the host goes
    # through VsDevCmd's own argument.
    Enter-VsDevShell -VsInstallPath $vs -SkipAutomaticLocation -DevCmdArguments "-arch=arm64 -host_arch=$hostArch -no_logo" | Out-Null
}

$version = Get-WorkspaceVersion
$versionText = $version -join '.'
Enter-Arm64DevShell
$work = Join-Path ([IO.Path]::GetTempPath()) ('taigi-forwarder-' + [Guid]::NewGuid().ToString('N').Substring(0, 8))
New-Item -ItemType Directory -Path $work | Out-Null
Push-Location $work
try {
    Copy-Item (Join-Path $ForwarderSources '*') $work
    New-Item -ItemType File -Path 'empty.cc' | Out-Null
    # One empty object per view pulls in that view's CRT load config. Without
    # them the link only warns (LNK4294), and an x64 process then reads the
    # forwarder as a plain Arm64 DLL, follows the native forward and fails
    # with error 193 - the service works in Arm64 apps and silently not in x64
    # ones (spike 2026-11-07).
    Invoke-Tool 'cl.exe' @('/nologo', '/c', '/Foempty_arm64.obj', 'empty.cc')
    Invoke-Tool 'cl.exe' @('/nologo', '/c', '/arm64EC', '/Foempty_x64.obj', 'empty.cc')
    # One import library per view. /ignore:4104 because the exports stay
    # public: PRIVATE drops them from these libraries and the final link
    # fails with LNK2001.
    Invoke-Tool 'link.exe' @('/lib', '/nologo', '/machine:arm64EC', '/ignore:4104', '/def:x64.def', '/out:x64.lib')
    Invoke-Tool 'link.exe' @('/lib', '/nologo', '/machine:arm64', '/ignore:4104', '/def:arm64.def', '/out:arm64.lib')
    Invoke-Tool 'rc.exe' @('/nologo', "/dTAIGI_VERSION=$versionText", "/dTAIGI_VERSION_COMMAS=$($version -join ',')", '/foforwarder.res', 'forwarder.rc')
    Invoke-Tool 'link.exe' @(
        '/dll', '/nologo', '/noentry', '/machine:arm64x',
        '/dynamicbase', '/highentropyva', '/largeaddressaware', '/ignore:4104',
        '/defArm64Native:arm64.def', '/def:x64.def', "/out:$ForwarderDll",
        'empty_arm64.obj', 'empty_x64.obj', 'arm64.lib', 'x64.lib', 'forwarder.res'
    )
    # The marker that the x64 view exists; a plain Arm64 DLL prints only
    # "AA64 machine (ARM64)".
    $headers = & dumpbin.exe /nologo /headers $ForwarderDll
    if (-not ($headers -match 'AA64 machine \(ARM64\) \(ARM64X\)')) {
        Fail "$ForwarderDll is not an Arm64X image (dumpbin /headers has no '(ARM64X)')"
    }
    New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
    Copy-Item $ForwarderDll $OutDir -Force
    Write-Host "  linked $(Join-Path $OutDir $ForwarderDll) ($versionText, Arm64X)"
} finally {
    Pop-Location
    Remove-Item $work -Recurse -Force -ErrorAction SilentlyContinue
}
