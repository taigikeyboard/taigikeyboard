# Asks COM for the text service's class object in THIS process and checks
# which service DLL came with it - the installer-arm64 CI job runs it once from an
# Arm64 process and once from an x64-emulated one, after installing (the Arm64
# DLL registers the Arm64X forwarder as InProcServer32;
# docs/architecture/windows-arm64-roadmap.md section Forwarder). Registration and
# typing are not exercised: activation through the forwarder is the claim.
#
#   powershell -File smoke-activation.ps1 -Architecture ARM64 -Expected TaigiKeyboardArm64.dll -Unexpected TaigiKeyboard.dll
#   pwsh -STA -File smoke-activation.ps1 -Architecture AMD64 -Expected TaigiKeyboard.dll -Unexpected TaigiKeyboardArm64.dll

[CmdletBinding()]
param(
    # This process's architecture as `PROCESSOR_ARCHITECTURE` names it - an
    # x64-emulated process reads AMD64 there.
    [Parameter(Mandatory = $true)] [ValidateSet('ARM64', 'AMD64')] [string] $Architecture,
    [Parameter(Mandatory = $true)] [string] $Expected,
    [Parameter(Mandatory = $true)] [string] $Unexpected
)

$ErrorActionPreference = 'Stop'
# Any failure names its line: a CI log otherwise shows the message alone.
# ASCII only: Windows PowerShell 5.1 reads a BOM-less script in the ANSI code
# page, where UTF-8 punctuation can decode to a curly quote that ends a string.
trap {
    Write-Host "smoke-activation.ps1 failed: $($_.Exception.Message)"
    Write-Host $_.InvocationInfo.PositionMessage
    exit 1
}

# guids.rs CLSID_TEXT_SERVICE; registration.rs FORWARDER_DLL.
$Clsid = [Guid] '32C28A51-8939-4C8F-8F29-037F9FD3CF0A'
$Forwarder = 'TaigiKeyboard64X.dll'
$IidClassFactory = [Guid] '00000001-0000-0000-C000-000000000046'
$ClsctxInprocServer = 1

function Fail([string] $Message) {
    Write-Error $Message
    exit 1
}

if ($env:PROCESSOR_ARCHITECTURE -ne $Architecture) {
    Fail "this process is $env:PROCESSOR_ARCHITECTURE, not ${Architecture}: the check would prove nothing"
}
# A Windows PowerShell thread is STA already and `pwsh -STA` makes it one, so
# the class object is created in this apartment rather than marshalled.
$apartment = [Threading.Thread]::CurrentThread.GetApartmentState()
if ($apartment -ne 'STA') { Fail "this thread is $apartment; run pwsh with -STA" }

Add-Type -Namespace TaigiSmoke -Name Native -MemberDefinition @'
[DllImport("ole32.dll")]
public static extern int CoGetClassObject(ref Guid clsid, uint context, IntPtr server, ref Guid iid, out IntPtr factory);
[DllImport("kernel32.dll", CharSet = CharSet.Unicode)]
public static extern IntPtr GetModuleHandleW(string name);
'@

$factory = [IntPtr]::Zero
$hresult = [TaigiSmoke.Native]::CoGetClassObject([ref] $Clsid, $ClsctxInprocServer, [IntPtr]::Zero, [ref] $IidClassFactory, [ref] $factory)
if ($hresult -ne 0) { Fail ('CoGetClassObject failed: 0x{0:X8}' -f $hresult) }
[void] [Runtime.InteropServices.Marshal]::Release($factory)

$loaded = @{}
foreach ($name in $Forwarder, $Expected, $Unexpected) {
    $loaded[$name] = [TaigiSmoke.Native]::GetModuleHandleW($name) -ne [IntPtr]::Zero
}
Write-Host ("{0}: {1}" -f $Architecture, (($loaded.GetEnumerator() | ForEach-Object { "$($_.Key)=$($_.Value)" }) -join ' '))
if (-not $loaded[$Forwarder]) { Fail "$Forwarder is not loaded: InProcServer32 does not name the forwarder" }
if (-not $loaded[$Expected]) { Fail "$Expected is not loaded: the forwarder sent this process elsewhere" }
if ($loaded[$Unexpected]) { Fail "$Unexpected is loaded: the wrong architecture's service" }
Write-Host "OK  $Architecture process activated the text service through $Forwarder into $Expected"
