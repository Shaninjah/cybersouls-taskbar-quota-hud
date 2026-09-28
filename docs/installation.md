# Cybersouls Taskbar Quota HUD installation model

Cybersouls Taskbar Quota HUD remains a single native Windows executable. Installation only places the executable and an uninstall helper in a stable per-user directory; it does not add a runtime, service, driver, telemetry component, or machine-wide dependency.

## Direct installation

- Install directory: `%LOCALAPPDATA%\Programs\CybersoulsTaskbarQuotaHUD`
- Executable: `%LOCALAPPDATA%\Programs\CybersoulsTaskbarQuotaHUD\cybersouls-taskbar-quota-hud.exe`
- Uninstall helper: `%LOCALAPPDATA%\Programs\CybersoulsTaskbarQuotaHUD\uninstall.ps1`
- Start menu shortcut: `%APPDATA%\Microsoft\Windows\Start Menu\Programs\Cybersouls Taskbar Quota HUD.lnk`
- Add/Remove Programs key: `HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\CybersoulsTaskbarQuotaHUD`

The installer is per-user and does not request elevation. It verifies the release SHA256 before replacing an existing executable. Replacement uses a temporary file and keeps the previous executable until the new file has been placed successfully.

## Download and update trust

Download URLs must be HTTPS GitHub release paths under `Shaninjah/cybersouls-taskbar-quota-hud`, with the expected asset filename. Checksum files must identify exactly `cybersouls-taskbar-quota-hud.exe`; the installer and self-updater reject an unexpected file or malformed checksum. GitHub may redirect the asset request to its release CDN. No provider OAuth token is used for updates.

The executable and checksum come from the same release. SHA256 detects download corruption or a mismatch; it is not an independent publisher signature and cannot protect against a compromised maintainer account or release workflow. The current EXE is not Authenticode-signed, so SmartScreen or antivirus warnings are possible. No self-signed certificate is provided as a public signing substitute.

The installer retains the previous EXE until a normal launch succeeds and remains active for its short startup check. With `-NoLaunch`, the `.old` backup is retained. Binary rollback does not make every registry/shortcut operation transactional, and startup checks cannot guarantee long-term runtime health. The self-updater also rolls back a replacement if the new process immediately fails to launch; full release download/update is still to be exercised after the first release.

Install/uninstall helpers reject symbolic links or junctions in application paths. Before recursive uninstall, the entire application tree (and the settings tree only with `-RemoveSettings`) is checked for reparse points. Settings are preserved by default. The shared startup key is removed only when it targets this installed EXE.

## Portable mode

`cybersouls-taskbar-quota-hud.exe` can be run from any user-writable directory without installation. Portable mode uses the same `%APPDATA%\CodexUsage\settings.json` settings as a direct or WinGet installation.

## Settings and startup behavior

- Upgrades preserve `%APPDATA%\CodexUsage\settings.json`.
- The global Codex 5h/7d display mode is stored as `quota_display_mode: "remaining"` or `"used"` in that same file. Missing/invalid values default to `remaining`; other preferences are retained. Select it from the widget/tray **Quota display** submenu. This setting does not affect Claude Code or Antigravity.
- Normal uninstall preserves settings so a later reinstall restores preferences.
- `uninstall.ps1 -RemoveSettings` explicitly deletes the settings directory.
- The installer does not enable startup automatically. If startup was already enabled, installation preserves that choice and updates the registry value to the stable installed executable. Users otherwise control startup from the application's settings menu.
- Uninstall removes the `CodexUsage` startup entry because its executable no longer exists.

## Migration from upstream

The shared settings directory `%APPDATA%\CodexUsage` and startup value `CodexUsage` intentionally remain compatible. Installation points the existing startup value to the quoted new executable path when startup was enabled. The custom application is installed separately under `CybersoulsTaskbarQuotaHUD`.

Test the release build before running the installer. For an existing direct upstream installation, the installer creates `codex-usage.original.exe` in the old installation directory without overwriting an existing backup, then stops only the matching monitor process. After the fork is installed, it archives the old EXE and uninstaller and removes the old registration and matching shortcuts. It never deletes the original backup or shared settings. Do not enable a second startup entry. A portable test does not repoint the existing `CodexUsage` startup value.

## WinGet

The historical manifests in `packaging/winget/1.6.0` describe the original `Ray.CodexUsage` package and are retained as upstream history, not as packaging for this fork. No Cybersouls WinGet package is published yet. Use the PowerShell installer or portable build for this fork. The reserved fork identifier is `Cybersouls.TaskbarQuotaHUD`; delegation to WinGet applies only to that specific package directory. A fork binary never delegates an upgrade to `Ray.CodexUsage`.

The PowerShell installer is not used as a WinGet installer because the public WinGet community repository does not accept script-based installers.
