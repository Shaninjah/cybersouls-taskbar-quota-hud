[CmdletBinding()]
param(
    [switch]$RemoveSettings,
    [switch]$Quiet
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$InstallDirectory = Join-Path $env:LOCALAPPDATA 'Programs\CybersoulsTaskbarQuotaHUD'
$ExpectedInstallDirectory = [IO.Path]::GetFullPath($InstallDirectory).TrimEnd('\')
$TargetPath = Join-Path $ExpectedInstallDirectory 'cybersouls-taskbar-quota-hud.exe'
$ShortcutPath = Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs\Cybersouls Taskbar Quota HUD.lnk'
$DesktopShortcutPath = Join-Path ([Environment]::GetFolderPath('Desktop')) 'Cybersouls Taskbar Quota HUD.lnk'
$UninstallKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\CybersoulsTaskbarQuotaHUD'
$RunKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run'
$SettingsDirectory = Join-Path $env:APPDATA 'CodexUsage'

$AllowedRoot = [IO.Path]::GetFullPath((Join-Path $env:LOCALAPPDATA 'Programs')).TrimEnd('\')
if (-not $ExpectedInstallDirectory.StartsWith($AllowedRoot + '\', [StringComparison]::OrdinalIgnoreCase)) {
    throw "Refusing to remove unexpected install directory: $ExpectedInstallDirectory"
}

Get-CimInstance Win32_Process -Filter "Name='cybersouls-taskbar-quota-hud.exe'" -ErrorAction SilentlyContinue |
    Where-Object { $_.ExecutablePath -eq $TargetPath } |
    ForEach-Object { Stop-Process -Id $_.ProcessId -Force }

if (Test-Path -LiteralPath $RunKey) {
    # A portable/upstream copy may have taken ownership of the shared startup key.
    $StartupTarget = (Get-ItemProperty -Path $RunKey -ErrorAction SilentlyContinue).PSObject.Properties['CodexUsage']
    if ($StartupTarget -and $StartupTarget.Value.Trim('"') -eq $TargetPath) {
        Remove-ItemProperty -Path $RunKey -Name 'CodexUsage' -ErrorAction SilentlyContinue
    }
}

Remove-Item -LiteralPath $ShortcutPath -Force -ErrorAction SilentlyContinue
Remove-Item -LiteralPath $DesktopShortcutPath -Force -ErrorAction SilentlyContinue
Remove-Item -LiteralPath $UninstallKey -Recurse -Force -ErrorAction SilentlyContinue

if ($RemoveSettings) {
    $ExpectedSettingsDirectory = [IO.Path]::GetFullPath((Join-Path $env:APPDATA 'CodexUsage')).TrimEnd('\')
    $ResolvedSettingsDirectory = [IO.Path]::GetFullPath($SettingsDirectory).TrimEnd('\')
    if ($ResolvedSettingsDirectory -eq $ExpectedSettingsDirectory) {
        Remove-Item -LiteralPath $ResolvedSettingsDirectory -Recurse -Force -ErrorAction SilentlyContinue
    }
}

if (Test-Path -LiteralPath $ExpectedInstallDirectory -PathType Container) {
    Remove-Item -LiteralPath $ExpectedInstallDirectory -Recurse -Force
}

if (-not $Quiet) {
    Write-Output 'Cybersouls Taskbar Quota HUD was uninstalled.'
    if (-not $RemoveSettings) {
        Write-Output "Settings were preserved at $SettingsDirectory"
    }
}
