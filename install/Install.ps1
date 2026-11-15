[CmdletBinding()]
param(
    [string]$ConfigPath
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# Constants
$ProductName = 'UrlBridge'
$Publisher = 'TripleT3005'
$ExeName = 'UrlBridge.exe'
$ConfigName = 'UrlBridge.json'
$UninstallName = 'Uninstall.ps1'
$LanguageFolder = 'lang'
$Marker = 'UrlBridge'
$SchemePattern = '^[a-z][a-z0-9+.-]{2,63}$'
$ClassesPath = 'SOFTWARE\Classes'
$SettingsPath = 'SOFTWARE\UrlBridge'
$UninstallPath = 'SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\UrlBridge'
$ExitSuccess = 0
$ExitUsage = 1
$ExitInvalidConfiguration = 2
$ExitCopyFailed = 3
$ExitRegistrationFailed = 4

# Helpers
function Write-Failure([string]$Message) {
    [Console]::Error.WriteLine($Message)
}

function Test-Administrator {
    $principal = New-Object Security.Principal.WindowsPrincipal([Security.Principal.WindowsIdentity]::GetCurrent())
    $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

function Invoke-Validation([string]$Exe) {
    $output = [IO.Path]::GetTempFileName()
    $errors = [IO.Path]::GetTempFileName()
    try {
        $process = Start-Process -FilePath $Exe -ArgumentList '--validate' -Wait -PassThru -NoNewWindow `
            -RedirectStandardOutput $output -RedirectStandardError $errors
        Get-Content -LiteralPath $output -Encoding UTF8 | ForEach-Object { Write-Host $_ }
        Get-Content -LiteralPath $errors -Encoding UTF8 | ForEach-Object { Write-Failure $_ }
        $process.ExitCode
    }
    finally {
        Remove-Item -LiteralPath $output, $errors -Force -ErrorAction SilentlyContinue
    }
}

function Test-OwnedScheme([Microsoft.Win32.RegistryKey]$Classes, [string]$Scheme) {
    $key = $Classes.OpenSubKey($Scheme)
    if ($null -eq $key) { return $true }
    try { $null -ne $key.GetValue($Marker) } finally { $key.Close() }
}

# 64-bit
if ([Environment]::Is64BitOperatingSystem -and -not [Environment]::Is64BitProcess) {
    $native = Join-Path $env:WINDIR 'SysNative\WindowsPowerShell\v1.0\powershell.exe'
    $arguments = @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', "`"$PSCommandPath`"")
    if ($ConfigPath) { $arguments += @('-ConfigPath', "`"$ConfigPath`"") }
    $process = Start-Process -FilePath $native -ArgumentList $arguments -Wait -PassThru -NoNewWindow
    exit $process.ExitCode
}

# Checks
if (-not (Test-Administrator)) {
    Write-Failure 'Administrator rights are required.'
    exit $ExitUsage
}
$sourceExe = Join-Path $PSScriptRoot $ExeName
$sourceUninstall = Join-Path $PSScriptRoot $UninstallName
foreach ($file in @($sourceExe, $sourceUninstall)) {
    if (-not (Test-Path -LiteralPath $file -PathType Leaf)) {
        Write-Failure "Missing file: $file"
        exit $ExitUsage
    }
}
if ($ConfigPath -and -not (Test-Path -LiteralPath $ConfigPath -PathType Leaf)) {
    Write-Failure "Configuration not found: $ConfigPath"
    exit $ExitUsage
}
$installDir = Join-Path $env:ProgramFiles $ProductName
$exe = Join-Path $installDir $ExeName
$config = Join-Path $installDir $ConfigName
if (-not $ConfigPath -and -not (Test-Path -LiteralPath $config -PathType Leaf)) {
    Write-Failure 'No configuration is installed yet. Pass one with -ConfigPath.'
    exit $ExitUsage
}

# Files
$previousConfig = $null
$previousExe = $null
try {
    New-Item -ItemType Directory -Path $installDir -Force | Out-Null
    if (Test-Path -LiteralPath $exe -PathType Leaf) {
        $previousExe = [IO.File]::ReadAllBytes($exe)
    }
    Copy-Item -LiteralPath $sourceExe -Destination $exe -Force
    Copy-Item -LiteralPath $sourceUninstall -Destination (Join-Path $installDir $UninstallName) -Force
    $sourceLanguages = Join-Path $PSScriptRoot $LanguageFolder
    if (Test-Path -LiteralPath $sourceLanguages -PathType Container) {
        $languages = Join-Path $installDir $LanguageFolder
        if (Test-Path -LiteralPath $languages) {
            Remove-Item -LiteralPath $languages -Recurse -Force
        }
        Copy-Item -LiteralPath $sourceLanguages -Destination $languages -Recurse
    }
    if ($ConfigPath) {
        if (Test-Path -LiteralPath $config -PathType Leaf) {
            $previousConfig = [IO.File]::ReadAllBytes($config)
        }
        Copy-Item -LiteralPath $ConfigPath -Destination $config -Force
    }
}
catch {
    Write-Failure "Copying failed: $($_.Exception.Message)"
    exit $ExitCopyFailed
}

# Validation
if ((Invoke-Validation $exe) -ne 0) {
    if ($null -ne $previousExe) {
        [IO.File]::WriteAllBytes($exe, $previousExe)
    }
    if ($ConfigPath -and $null -ne $previousConfig) {
        [IO.File]::WriteAllBytes($config, $previousConfig)
    }
    elseif ($ConfigPath) {
        Remove-Item -LiteralPath $config -Force
    }
    if ($null -ne $previousExe) {
        Write-Failure 'The configuration is invalid. The previous installation was restored.'
    }
    else {
        Write-Failure 'The configuration is invalid and was not installed.'
    }
    exit $ExitInvalidConfiguration
}

# Protocols
try {
    $settings = Get-Content -LiteralPath $config -Raw -Encoding UTF8 | ConvertFrom-Json
    $protocols = @($settings.protocols)
    foreach ($protocol in $protocols) {
        if ($protocol.scheme -cnotmatch $SchemePattern) {
            throw "Unexpected scheme: $($protocol.scheme)"
        }
    }
    $machine = [Microsoft.Win32.RegistryKey]::OpenBaseKey(
        [Microsoft.Win32.RegistryHive]::LocalMachine, [Microsoft.Win32.RegistryView]::Registry64)
    $classes = $machine.OpenSubKey($ClassesPath, $true)
    foreach ($protocol in $protocols) {
        if (-not (Test-OwnedScheme $classes $protocol.scheme)) {
            throw "The protocol '$($protocol.scheme)' is already registered by another program."
        }
    }
    $settingsKey = $machine.CreateSubKey($SettingsPath)
    $registered = @($settingsKey.GetValue('Schemes', [string[]]@()))
    $schemes = @($protocols | ForEach-Object { $_.scheme })
    foreach ($scheme in $registered) {
        if ($scheme -cmatch $SchemePattern -and $schemes -cnotcontains $scheme -and
            $null -ne $classes.OpenSubKey($scheme) -and (Test-OwnedScheme $classes $scheme)) {
            $classes.DeleteSubKeyTree($scheme)
            Write-Host "Removed protocol: $scheme"
        }
    }
    foreach ($protocol in $protocols) {
        $key = $classes.CreateSubKey($protocol.scheme)
        $key.SetValue('', "URL:$($protocol.name)")
        $key.SetValue('URL Protocol', '')
        $key.SetValue($Marker, '1')
        $command = $key.CreateSubKey('shell\open\command')
        $command.SetValue('', "`"$exe`" `"%1`"")
        $command.Close()
        $key.Close()
        Write-Host "Registered protocol: $($protocol.scheme)"
    }
    $settingsKey.SetValue('Schemes', [string[]]$schemes, [Microsoft.Win32.RegistryValueKind]::MultiString)
    $settingsKey.Close()
    $classes.Close()
}
catch {
    Write-Failure "Registration failed: $($_.Exception.Message)"
    exit $ExitRegistrationFailed
}

# Event log and uninstall entry
try {
    if (-not [Diagnostics.EventLog]::SourceExists($ProductName)) {
        New-EventLog -LogName Application -Source $ProductName
    }
    $version = (Get-Item -LiteralPath $exe).VersionInfo.ProductVersion
    $size = [int]((Get-ChildItem -LiteralPath $installDir -File | Measure-Object -Property Length -Sum).Sum / 1KB)
    $powershell = Join-Path $env:WINDIR 'System32\WindowsPowerShell\v1.0\powershell.exe'
    $uninstallCommand = "`"$powershell`" -NoProfile -ExecutionPolicy Bypass -File `"$(Join-Path $installDir $UninstallName)`""
    $uninstall = $machine.CreateSubKey($UninstallPath)
    $uninstall.SetValue('DisplayName', $ProductName)
    $uninstall.SetValue('DisplayVersion', $version)
    $uninstall.SetValue('Publisher', $Publisher)
    $uninstall.SetValue('InstallLocation', $installDir)
    $uninstall.SetValue('DisplayIcon', $exe)
    $uninstall.SetValue('UninstallString', $uninstallCommand)
    $uninstall.SetValue('QuietUninstallString', $uninstallCommand)
    $uninstall.SetValue('NoModify', 1, [Microsoft.Win32.RegistryValueKind]::DWord)
    $uninstall.SetValue('NoRepair', 1, [Microsoft.Win32.RegistryValueKind]::DWord)
    $uninstall.SetValue('EstimatedSize', $size, [Microsoft.Win32.RegistryValueKind]::DWord)
    $uninstall.Close()
    $machine.Close()
}
catch {
    Write-Failure "Registration failed: $($_.Exception.Message)"
    exit $ExitRegistrationFailed
}

Write-Host "$ProductName $version is installed in $installDir."
exit $ExitSuccess
