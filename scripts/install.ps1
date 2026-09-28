[CmdletBinding()]
param(
    [string]$SourcePath,
    [string]$ExpectedSha256,
    [string]$Version,
    [switch]$NoLaunch
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

function Assert-NoReparsePoint {
    param([Parameter(Mandatory = $true)][string]$Path)
    $current = [IO.Path]::GetFullPath($Path)
    while ($current) {
        $item = Get-Item -LiteralPath $current -Force -ErrorAction SilentlyContinue
        if ($item) {
            if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) {
                throw 'Refusing to modify a path containing a symbolic link or junction.'
            }
        }
        $parent = Split-Path -Parent $current
        if ($parent -eq $current) { break }
        $current = $parent
    }
}

foreach ($root in @($env:LOCALAPPDATA, $env:APPDATA)) {
    if ([string]::IsNullOrWhiteSpace($root) -or -not [IO.Path]::IsPathRooted($root)) {
        throw 'A valid absolute per-user application data directory is required.'
    }
}
if ($Version -and $Version -notmatch '^v?\d+\.\d+\.\d+$') {
    throw 'Version must be a numeric major.minor.patch release.'
}
if ($ExpectedSha256 -and $ExpectedSha256 -notmatch '^[0-9a-fA-F]{64}$') {
    throw 'ExpectedSha256 must be exactly 64 hexadecimal characters.'
}

$Repository = 'Shaninjah/cybersouls-taskbar-quota-hud'
$InstallDirectory = Join-Path $env:LOCALAPPDATA 'Programs\CybersoulsTaskbarQuotaHUD'
$TargetPath = Join-Path $InstallDirectory 'cybersouls-taskbar-quota-hud.exe'
$InstalledUninstaller = Join-Path $InstallDirectory 'uninstall.ps1'
$ShortcutPath = Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs\Cybersouls Taskbar Quota HUD.lnk'
$DesktopShortcutPath = Join-Path ([Environment]::GetFolderPath('Desktop')) 'Cybersouls Taskbar Quota HUD.lnk'
$UninstallKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\CybersoulsTaskbarQuotaHUD'
$RunKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run'
$TempDirectory = Join-Path ([IO.Path]::GetTempPath()) ('cybersouls-taskbar-quota-hud-install-' + [Guid]::NewGuid().ToString('N'))
$LegacyDirectory = Join-Path $env:LOCALAPPDATA 'Programs\CodexUsage'
$LegacyExecutable = Join-Path $LegacyDirectory 'codex-usage.exe'
$OriginalBackup = Join-Path $LegacyDirectory 'codex-usage.original.exe'
foreach ($path in @($TargetPath, "$TargetPath.new", "$TargetPath.old", $InstalledUninstaller, $LegacyExecutable, $OriginalBackup, $TempDirectory)) {
    Assert-NoReparsePoint -Path $path
}

function Assert-ReleaseAssetUrl {
    param([Parameter(Mandatory = $true)][string]$Url, [Parameter(Mandatory = $true)][string]$Name)
    $prefix = "https://github.com/$Repository/releases/download/"
    if (-not $Url.StartsWith($prefix, [StringComparison]::Ordinal)) {
        throw 'Release assets must use this fork''s HTTPS GitHub download path.'
    }
    $parts = $Url.Substring($prefix.Length).Split('/')
    if ($parts.Length -ne 2 -or $parts[0] -in @('.', '..') -or $parts[0] -notmatch '^[A-Za-z0-9._-]+$' -or $parts[1] -cne $Name) {
        throw 'Release asset URL has an unexpected tag or filename.'
    }
}

function Read-ReleaseChecksum {
    param([Parameter(Mandatory = $true)][string]$Text)
    if ($Text -notmatch '\A([0-9a-fA-F]{64})[ \t]+cybersouls-taskbar-quota-hud\.exe(?:\r?\n)?\z') {
        throw 'Checksum must contain one SHA256 and the expected release filename.'
    }
    return $Matches[1]
}

$StartupWasEnabled = $false
$ExistingStartup = $null
if (Test-Path -LiteralPath $RunKey) {
    $ExistingStartup = try {
        Get-ItemPropertyValue -Path $RunKey -Name 'CodexUsage' -ErrorAction Stop
    }
    catch {
        $null
    }
    $StartupWasEnabled = -not [string]::IsNullOrWhiteSpace($ExistingStartup)
}

function Get-ReleaseAsset {
    param(
        [Parameter(Mandatory = $true)]$Release,
        [Parameter(Mandatory = $true)][string]$Name
    )

    $asset = $Release.assets | Where-Object { $_.name -eq $Name } | Select-Object -First 1
    if (-not $asset) {
        throw "Release asset '$Name' was not found."
    }
    Assert-ReleaseAssetUrl -Url $asset.browser_download_url -Name $Name
    return $asset.browser_download_url
}

function Invoke-ReleaseDownload {
    param(
        [Parameter(Mandatory = $true)][string]$Url,
        [Parameter(Mandatory = $true)][string]$Destination
    )

    $response = Invoke-WebRequest -UseBasicParsing -TimeoutSec 120 -Headers @{ 'User-Agent' = 'CybersoulsTaskbarQuotaHUD-Installer' } -Uri $Url -OutFile $Destination -PassThru
    $finalUri = if ($response.BaseResponse.PSObject.Properties['ResponseUri']) {
        $response.BaseResponse.ResponseUri
    } else {
        $response.BaseResponse.RequestMessage.RequestUri
    }
    if ($finalUri.Scheme -ne 'https') { throw 'Release download redirected outside HTTPS.' }
}

New-Item -ItemType Directory -Force -Path $TempDirectory | Out-Null

try {
    $StagedExecutable = Join-Path $TempDirectory 'cybersouls-taskbar-quota-hud.exe'
    $StagedUninstaller = Join-Path $TempDirectory 'uninstall.ps1'

    if ($SourcePath) {
        $ResolvedSource = (Resolve-Path -LiteralPath $SourcePath).Path
        Copy-Item -LiteralPath $ResolvedSource -Destination $StagedExecutable -Force

        $LocalUninstaller = Join-Path $PSScriptRoot 'uninstall.ps1'
        if (-not (Test-Path -LiteralPath $LocalUninstaller -PathType Leaf)) {
            throw "Local uninstall helper was not found at $LocalUninstaller"
        }
        Copy-Item -LiteralPath $LocalUninstaller -Destination $StagedUninstaller -Force
    }
    else {
        $ApiUrl = if ($Version) {
            "https://api.github.com/repos/$Repository/releases/tags/v$($Version.TrimStart('v'))"
        }
        else {
            "https://api.github.com/repos/$Repository/releases/latest"
        }

        $Release = Invoke-RestMethod -UseBasicParsing -TimeoutSec 30 -Headers @{ 'User-Agent' = 'CybersoulsTaskbarQuotaHUD-Installer' } -Uri $ApiUrl
        $ExecutableUrl = Get-ReleaseAsset -Release $Release -Name 'cybersouls-taskbar-quota-hud.exe'
        $ChecksumUrl = Get-ReleaseAsset -Release $Release -Name 'cybersouls-taskbar-quota-hud.exe.sha256'
        $UninstallerUrl = Get-ReleaseAsset -Release $Release -Name 'uninstall.ps1'
        $ChecksumPath = Join-Path $TempDirectory 'cybersouls-taskbar-quota-hud.exe.sha256'

        Invoke-ReleaseDownload -Url $ExecutableUrl -Destination $StagedExecutable
        Invoke-ReleaseDownload -Url $ChecksumUrl -Destination $ChecksumPath
        Invoke-ReleaseDownload -Url $UninstallerUrl -Destination $StagedUninstaller

        $ChecksumText = Get-Content -Raw -LiteralPath $ChecksumPath
        $ExpectedSha256 = Read-ReleaseChecksum -Text $ChecksumText
    }

    $ActualSha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $StagedExecutable).Hash
    if ($ExpectedSha256 -and $ActualSha256 -ne $ExpectedSha256.Trim().ToUpperInvariant()) {
        throw "SHA256 mismatch. Expected $ExpectedSha256 but downloaded $ActualSha256."
    }

    New-Item -ItemType Directory -Force -Path $InstallDirectory | Out-Null

    # Preserve the original installation before stopping or migrating it.
    # Never overwrite or remove this permanent backup on later installations.
    if (Test-Path -LiteralPath $LegacyExecutable -PathType Leaf) {
        if (-not (Test-Path -LiteralPath $OriginalBackup -PathType Leaf)) {
            Copy-Item -LiteralPath $LegacyExecutable -Destination $OriginalBackup
        }
        Get-CimInstance Win32_Process -Filter "Name='codex-usage.exe'" -ErrorAction SilentlyContinue |
            Where-Object { $_.ExecutablePath -eq $LegacyExecutable } |
            ForEach-Object { Stop-Process -Id $_.ProcessId -Force }
    }

    Get-CimInstance Win32_Process -Filter "Name='cybersouls-taskbar-quota-hud.exe'" -ErrorAction SilentlyContinue |
        Where-Object { $_.ExecutablePath -eq $TargetPath } |
        ForEach-Object { Stop-Process -Id $_.ProcessId -Force }

    $NewPath = "$TargetPath.new"
    $BackupPath = "$TargetPath.old"
    Copy-Item -LiteralPath $StagedExecutable -Destination $NewPath -Force
    Remove-Item -LiteralPath $BackupPath -Force -ErrorAction SilentlyContinue

    $HadPreviousVersion = Test-Path -LiteralPath $TargetPath -PathType Leaf
    if ($HadPreviousVersion) {
        Move-Item -LiteralPath $TargetPath -Destination $BackupPath -Force
    }

    try {
        Move-Item -LiteralPath $NewPath -Destination $TargetPath -Force
    }
    catch {
        Remove-Item -LiteralPath $NewPath -Force -ErrorAction SilentlyContinue
        if ($HadPreviousVersion -and (Test-Path -LiteralPath $BackupPath -PathType Leaf)) {
            Move-Item -LiteralPath $BackupPath -Destination $TargetPath -Force
        }
        throw
    }

    try {
        Copy-Item -LiteralPath $StagedUninstaller -Destination $InstalledUninstaller -Force

        $InstalledVersion = (Get-Item -LiteralPath $TargetPath).VersionInfo.ProductVersion
        if (-not $InstalledVersion) {
            throw 'The installed executable does not contain a product version.'
        }

        New-Item -Path $UninstallKey -Force | Out-Null
        $UninstallCommand = "powershell.exe -NoProfile -ExecutionPolicy Bypass -File `"$InstalledUninstaller`""
        Set-ItemProperty -Path $UninstallKey -Name DisplayName -Value 'Cybersouls Taskbar Quota HUD'
        Set-ItemProperty -Path $UninstallKey -Name DisplayVersion -Value $InstalledVersion
        Set-ItemProperty -Path $UninstallKey -Name Publisher -Value 'Cybersouls'
        Set-ItemProperty -Path $UninstallKey -Name DisplayIcon -Value $TargetPath
        Set-ItemProperty -Path $UninstallKey -Name InstallLocation -Value $InstallDirectory
        Set-ItemProperty -Path $UninstallKey -Name URLInfoAbout -Value "https://github.com/$Repository"
        Set-ItemProperty -Path $UninstallKey -Name UninstallString -Value $UninstallCommand
        Set-ItemProperty -Path $UninstallKey -Name QuietUninstallString -Value "$UninstallCommand -Quiet"
        Set-ItemProperty -Path $UninstallKey -Name NoModify -Type DWord -Value 1
        Set-ItemProperty -Path $UninstallKey -Name NoRepair -Type DWord -Value 1

        if ($StartupWasEnabled) {
            Set-ItemProperty -Path $RunKey -Name 'CodexUsage' -Value ('"' + $TargetPath + '"')
        }

        $Shell = New-Object -ComObject WScript.Shell
        foreach ($LinkPath in @($ShortcutPath, $DesktopShortcutPath)) {
            $ShortcutDirectory = Split-Path -Parent $LinkPath
            New-Item -ItemType Directory -Force -Path $ShortcutDirectory | Out-Null
            $Shortcut = $Shell.CreateShortcut($LinkPath)
            $Shortcut.TargetPath = $TargetPath
            $Shortcut.WorkingDirectory = $InstallDirectory
            $Shortcut.IconLocation = "$TargetPath,0"
            $Shortcut.Description = 'Cybersouls Taskbar Quota HUD'
            $Shortcut.Save()
        }

        # Ask Explorer to refresh shortcut icons after an in-place EXE upgrade.
        # This avoids a stale cached icon while preserving the standard arrow overlay.
        $IconRefreshTool = Join-Path $env:SystemRoot 'System32\ie4uinit.exe'
        if (Test-Path -LiteralPath $IconRefreshTool -PathType Leaf) {
            Start-Process -FilePath $IconRefreshTool -ArgumentList '-show' -WindowStyle Hidden -Wait
        }
        if (-not $NoLaunch) {
            $StartedApp = Start-Process -FilePath $TargetPath -WorkingDirectory $InstallDirectory -WindowStyle Hidden -PassThru
            Start-Sleep -Seconds 2
            if ($StartedApp.HasExited) { throw 'The installed application exited immediately.' }
        }
    }
    catch {
        if ($HadPreviousVersion -and (Test-Path -LiteralPath $BackupPath -PathType Leaf)) {
            Remove-Item -LiteralPath $TargetPath -Force -ErrorAction SilentlyContinue
            Move-Item -LiteralPath $BackupPath -Destination $TargetPath -Force
        }
        if ($StartupWasEnabled -and $ExistingStartup) {
            Set-ItemProperty -Path $RunKey -Name 'CodexUsage' -Value $ExistingStartup
        }
        if ($HadPreviousVersion -and -not $NoLaunch -and (Test-Path -LiteralPath $TargetPath -PathType Leaf)) {
            Start-Process -FilePath $TargetPath -WorkingDirectory $InstallDirectory -WindowStyle Hidden
        }
        throw
    }

    if (-not $NoLaunch) {
        Remove-Item -LiteralPath $BackupPath -Force -ErrorAction SilentlyContinue
    }

    # Retire only the known upstream installation after the fork is installed.
    # Keep the original EXE backup and all shared settings for rollback.
    if (Test-Path -LiteralPath $LegacyExecutable -PathType Leaf) {
        $RetiredExecutable = Join-Path $LegacyDirectory ('codex-usage.retired-' + [Guid]::NewGuid().ToString('N') + '.exe')
        Move-Item -LiteralPath $LegacyExecutable -Destination $RetiredExecutable
        $LegacyUninstaller = Join-Path $LegacyDirectory 'uninstall.ps1'
        if (Test-Path -LiteralPath $LegacyUninstaller -PathType Leaf) {
            Move-Item -LiteralPath $LegacyUninstaller -Destination (Join-Path $LegacyDirectory ('uninstall.original-' + [Guid]::NewGuid().ToString('N') + '.ps1'))
        }
        $LegacyUninstallKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\CodexUsage'
        if (Test-Path -LiteralPath $LegacyUninstallKey) {
            $LegacyLocation = Get-ItemPropertyValue -Path $LegacyUninstallKey -Name InstallLocation -ErrorAction SilentlyContinue
            if ($LegacyLocation -eq $LegacyDirectory) {
                Remove-Item -LiteralPath $LegacyUninstallKey -Recurse -Force
            }
        }
        foreach ($LegacyLink in @(
            (Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs\Codex Usage.lnk'),
            (Join-Path ([Environment]::GetFolderPath('Desktop')) 'Codex Usage.lnk')
        )) {
            if (Test-Path -LiteralPath $LegacyLink -PathType Leaf) {
                $OldShortcut = $Shell.CreateShortcut($LegacyLink)
                if ($OldShortcut.TargetPath -eq $LegacyExecutable) {
                    Remove-Item -LiteralPath $LegacyLink -Force
                }
            }
        }
    }

    Write-Output "Cybersouls Taskbar Quota HUD $InstalledVersion installed to $InstallDirectory"
    Write-Output "SHA256: $ActualSha256"
}
finally {
    $ExpectedTempParent = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd('\')
    $ResolvedTemp = [IO.Path]::GetFullPath($TempDirectory).TrimEnd('\')
    if ($ResolvedTemp.StartsWith($ExpectedTempParent + '\', [StringComparison]::OrdinalIgnoreCase)) {
        Assert-NoReparsePoint -Path $ResolvedTemp
        Remove-Item -LiteralPath $ResolvedTemp -Recurse -Force -ErrorAction SilentlyContinue
    }
}
