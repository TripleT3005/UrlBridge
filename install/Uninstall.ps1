[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# Constants
$ProductName = 'UrlBridge'
$Marker = 'UrlBridge'
$ClassesPath = 'SOFTWARE\Classes'
$SettingsPath = 'SOFTWARE\UrlBridge'
$UninstallPath = 'SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\UrlBridge'
$ExitSuccess = 0
$ExitUsage = 1
$ExitCopyFailed = 3
$ExitRegistrationFailed = 4

# Helpers
function Write-Failure([string]$Message) {
    [Console]::Error.WriteLine($Message)
}

# 64-bit
if ([Environment]::Is64BitOperatingSystem -and -not [Environment]::Is64BitProcess) {
    $native = Join-Path $env:WINDIR 'SysNative\WindowsPowerShell\v1.0\powershell.exe'
    $arguments = @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', "`"$PSCommandPath`"")
    $process = Start-Process -FilePath $native -ArgumentList $arguments -Wait -PassThru -NoNewWindow
    exit $process.ExitCode
}

# Checks
$principal = New-Object Security.Principal.WindowsPrincipal([Security.Principal.WindowsIdentity]::GetCurrent())
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    Write-Failure 'Administrator rights are required.'
    exit $ExitUsage
}

# Registry
try {
    $machine = [Microsoft.Win32.RegistryKey]::OpenBaseKey(
        [Microsoft.Win32.RegistryHive]::LocalMachine, [Microsoft.Win32.RegistryView]::Registry64)
    $classes = $machine.OpenSubKey($ClassesPath, $true)
    $settingsKey = $machine.OpenSubKey($SettingsPath)
    if ($null -ne $settingsKey) {
        foreach ($scheme in @($settingsKey.GetValue('Schemes', [string[]]@()))) {
            $key = $classes.OpenSubKey($scheme)
            if ($null -eq $key) { continue }
            $owned = $null -ne $key.GetValue($Marker)
            $key.Close()
            if ($owned) {
                $classes.DeleteSubKeyTree($scheme)
                Write-Host "Removed protocol: $scheme"
            }
        }
        $settingsKey.Close()
        $machine.DeleteSubKeyTree($SettingsPath)
    }
    $classes.Close()
    $machine.DeleteSubKeyTree($UninstallPath, $false)
    $machine.Close()
    if ([Diagnostics.EventLog]::SourceExists($ProductName)) {
        Remove-EventLog -Source $ProductName
    }
}
catch {
    Write-Failure "Removing the registration failed: $($_.Exception.Message)"
    exit $ExitRegistrationFailed
}

# Files
try {
    $installDir = Join-Path $env:ProgramFiles $ProductName
    Set-Location -LiteralPath $env:WINDIR
    if (Test-Path -LiteralPath $installDir) {
        Remove-Item -LiteralPath $installDir -Recurse -Force
    }
}
catch {
    Write-Failure "Removing the files failed: $($_.Exception.Message)"
    exit $ExitCopyFailed
}

Write-Host "$ProductName was removed."
exit $ExitSuccess
