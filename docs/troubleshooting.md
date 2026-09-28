# Troubleshooting Cybersouls Taskbar Quota HUD

## Taskbar error labels

Cybersouls Taskbar Quota HUD keeps authentication failures separate from transient service failures:

| Simplified Chinese | Other languages | Meaning | Recommended action |
|---|---|---|---|
| `!` | `!` | Enabled service credentials are missing or expired | Sign in with the relevant CLI/app, then refresh Cybersouls Taskbar Quota HUD. Claude Desktop login does not count as Claude Code CLI login. |
| `网络` | `NET` | Network or TLS connection failed | Check connectivity, VPN, proxy, and firewall settings |
| `限流` | `429` | Provider rate limit | Wait for the provider retry window; Cybersouls Taskbar Quota HUD retries with backoff |
| `服务` | `5XX` | Provider service failure | Wait and retry; check provider status if it persists |
| `错误` | `ERR` | Invalid or unsupported response | Enable diagnostics and inspect the log |

Authentication failures pause provider polling until the credential source changes, preventing repeated login notifications. Transient failures use exponential backoff up to the configured refresh interval.

## Diagnostic log

Run:

```powershell
cybersouls-taskbar-quota-hud.exe --diagnose
```

The log is written to `%TEMP%\cybersouls-taskbar-quota-hud.log`. It includes:

- application version and executable path
- direct or WinGet install channel
- provider failure category and retry delay
- window creation, taskbar placement, and relaunch events

The log does not include access tokens, refresh tokens, credential file contents, or API response bodies.

## Update failures

Direct installations and portable copies download only the exact `cybersouls-taskbar-quota-hud.exe` asset and verify it against `cybersouls-taskbar-quota-hud.exe.sha256` from the same GitHub Release. The updater keeps the previous EXE until the downloaded version has been installed and restarted successfully. If restart fails, the old EXE is restored.

Updates come from the fork repository configured in `Cargo.toml`. Before the first fork release is published, GitHub may return 404 for the latest-release query. There is no fallback to upstream.

No fork WinGet package has been published. A future `Cybersouls.TaskbarQuotaHUD` package will delegate its own upgrades to WinGet; `Ray.CodexUsage` is never an update source for this fork.

## Reset local position without deleting settings

Right-click the taskbar component and choose **Settings > Reset Position**. Settings are stored at `%APPDATA%\CodexUsage\settings.json`.

## Reinstall while preserving settings

Normal uninstall keeps the settings file. Reinstalling restores the saved language, refresh interval, provider selection, widget visibility, and taskbar position. Use `uninstall.ps1 -RemoveSettings` only when a full reset is intended.
