param (
    [string]$ShortcutPath,
    [string]$TargetPath,
    [string]$RequiresElevation = "false",
    [string]$CurrentDirectory = $PSScriptRoot
)

if ($RequiresElevation -ne "false" -and -not ([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole] "Administrator")) {
    Write-Host "Elevation required. Restarting script with elevated privileges..."
    Start-Process powershell -ArgumentList "-ExecutionPolicy Bypass -File `"$PSCommandPath`" -ShortcutPath `"$ShortcutPath`" -TargetPath `"$TargetPath`" -RequiresElevation $RequiresElevation -CurrentDirectory `"$CurrentDirectory`"" -Verb RunAs -Wait
    exit
}

Set-Location -Path $CurrentDirectory

$WScriptShell = New-Object -ComObject WScript.Shell
$Shortcut = $WScriptShell.CreateShortcut($ShortcutPath)
$Shortcut.TargetPath = $TargetPath
$Shortcut.Save()
