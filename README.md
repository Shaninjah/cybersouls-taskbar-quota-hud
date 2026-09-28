![Windows](https://img.shields.io/badge/platform-Windows-blue)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

**English** | [简体中文](README.zh-CN.md)

# Cybersouls Taskbar Quota HUD

<img src=".github/codex-usage-icon.png" alt="Cybersouls Taskbar Quota HUD icon" width="96" height="96">

![Screenshot](.github/animation.gif)

**AI quota monitoring directly in your Windows taskbar.**

A small native Windows application forked from [upstream-ray/codex-usage-monitor](https://github.com/upstream-ray/codex-usage-monitor), with optional Claude Code and Google Antigravity monitoring. This community fork is not an official product and is not affiliated with or endorsed by OpenAI, Anthropic, or Google.

## Dynamic quota colors

Each Codex row (5h and 7d) compares **remaining quota with the precise time until its own reset**, independently of the selected display mode. For the weekly cycle:

`quotaHoursRemaining = remainingPercent / 100 * 168`

`bufferHours = quotaHoursRemaining - secondsUntilReset / 3600`

| Color | Weekly buffer (hours) | Dark theme | Light theme |
|---|---|---|---|
| Green 1 | ≥ +36 | `#22C55E` | `#166534` |
| Green 2 | ≥ +24 and < +36 | `#4ACF65` | `#236A2B` |
| Green 3 | ≥ +12 and < +24 | `#72D56C` | `#356E25` |
| Green 4 | ≥ 0 and < +12 | `#9BDC72` | `#4B7221` |
| Yellow 1 | ≥ −6 and < 0 | `#C6DD6B` | `#65741D` |
| Yellow 2 | ≥ −12 and < −6 | `#DDE05B` | `#777019` |
| Yellow 3 | ≥ −18 and < −12 | `#EACD47` | `#8A6817` |
| Yellow 4 | ≥ −24 and < −18 | `#F2BC35` | `#9B5D14` |
| Orange 1 | ≥ −30 and < −24 | `#F7A72B` | `#A65216` |
| Orange 2 | ≥ −36 and < −30 | `#F99028` | `#AF471C` |
| Orange 3 | ≥ −42 and < −36 | `#F77B2D` | `#B63C22` |
| Orange 4 | ≥ −48 and < −42 | `#F36835` | `#BB3128` |
| Red 1 | ≥ −60 and < −48 | `#EF593E` | `#BA2B2D` |
| Red 2 | ≥ −72 and < −60 | `#EF5342` | `#B72530` |
| Red 3 | ≥ −84 and < −72 | `#EF4D47` | `#AC2030` |
| Red 4 | < −84 | `#EF474C` | `#991B2B` |

The palette has **16 discrete shades: four green, four yellow, four orange and four red**, with smaller changes between adjacent shades. Exactly on pace (`bufferHours = 0`) is green. For example, 50% remaining represents 84 hours of quota: with 48 hours until reset it is Green 1 (+36 hours), but with 120 hours until reset it is Orange 2 (−36 hours). 10% remaining with 120 hours until reset is Red 4 (−103.2 hours). An exact boundary uses the less severe shade; any actual negative buffer enters yellow.

The 5h row uses a five-hour cycle with proportionally scaled thresholds: weekly buffer thresholds are multiplied by `5h / 168h`, so +36 hours becomes about +1h04m17s. Each row uses its own reset timestamp. Calculations retain fractional hours and seconds without rounding to whole hours or days; a color changes when a band boundary is crossed. Colors repaint at least once a minute while a future Codex reset is known, without additional quota requests. Light-theme variants improve small-text contrast. Loading/error values and unknown or expired reset times use a neutral Codex color; no fixed-percentage fallback is used. Claude Code and Antigravity retain their existing colors.

This is a comparison with a theoretical linear budget, not a prediction based on measured consumption history. Low-quota alerts retain their existing percentage thresholds.

## Quota display

Right-click the widget or tray icon and choose **Quota display → Remaining quota / Used quota**. The radio selection applies to both Codex rows (5h and 7d), in every language:

- **Remaining quota** (default): the bar and number show `100 - used`. A full quota is a full bar, which empties as quota is used.
- **Used quota**: the bar and number show the percentage used, preserving the historical used-quota display.

For 20% used, Remaining shows **80%** with an 80% bar; Used shows **20%** with a 20% bar. Both use the same pace color based on remaining quota and reset time. For 90% used, the values are **10%** and **90%**, respectively, again with the same pace color. Switching modes updates cached display data immediately without fetching quotas. Reset counters and display rounding remain unchanged.

The setting `"quota_display_mode": "remaining"` or `"used"` is saved in the existing `%APPDATA%\CodexUsage\settings.json`. Missing or invalid values default safely to `remaining`, including older settings files, while retaining other preferences. Language chooses text/layout, not the Codex display mode; Simplified Chinese keeps its compact reset times with the appropriate remaining/used label. This setting affects Codex only; Claude Code and Antigravity keep their historical display behavior.

## Fork version and branding

The first Cybersouls release is **1.9.2**, with recommended tag `v1.9.2`. The existing updater compares numeric major/minor/patch values and ignores prerelease suffixes, so releases use ordinary increasing numeric versions. Both the updater and installer use [this fork's releases](https://github.com/Shaninjah/cybersouls-taskbar-quota-hud/releases), never upstream releases. Until the first release exists, update checks may report that no release is available. The original application icon is retained for this first version; a distinct icon can be added later.

Keep `origin` pointing to this fork and `upstream` pointing to the original repository. Future upstream changes can be fetched with `git fetch upstream` and reviewed/merged from `upstream/main`.

## Credential safety

Credentials remain in their local provider-managed stores. The existing application code reads the local credentials needed by enabled providers and sends authenticated usage requests to their endpoints. There is no intermediate Cybersouls backend. Never commit credentials, `auth.json`, local environment files, or private keys. The repository's ignore rules provide additional protection; they do not replace review before committing.

It sits in your taskbar and shows how much of your Codex usage window remains without opening the Codex app or account usage page.

## What You Get

- A **5h** bar for your current Codex usage window
- A **7d** bar for your current weekly window
- Simplified Chinese display with explicit remaining usage and reset countdowns
- Optional Claude Code usage alongside Codex
- Optional Antigravity model usage bars for Google's 5-hour and weekly Gemini quota windows
- A live countdown until each limit resets
- Optional low-quota alerts at 10%, 20%, or 30% remaining, deduplicated per reset window
- Independent display controls for the 5-hour and weekly rows
- A small native widget that lives directly in the Windows taskbar
- One system tray icon that matches the desktop app icon
- Left-click the tray icon to toggle the taskbar widget on or off
- Right-click options for refresh, monitored services, usage rows, quota alerts, update frequency, language, startup, widget visibility, and updates
- Multi-monitor taskbar placement, so the widget can live on the taskbar for the screen you prefer

## Who This Is For

This app is for Windows users who already have **Codex CLI or the Codex app installed and signed in**.

Codex is enabled by default. The app reads the same local credentials used by Codex.

Antigravity support is optional too. To show Antigravity usage, install and sign in to Google Antigravity, then enable the **Antigravity** service from the right-click **Monitored services** menu.

It works best if you want a simple "how close am I to the limit?" display that is always visible.

## Requirements

- Windows 10 or Windows 11
- Codex CLI or Codex app installed and authenticated
- Optional: Claude Code installed and authenticated
- Optional: Google Antigravity installed and authenticated, if you want Antigravity usage

If you use Claude Code through WSL, that is supported too. The monitor can read your Claude Code credentials from Windows or from your WSL environment.

## Install

The first Cybersouls release has not been published yet. Until it is, build the feature branch locally; the release downloads below become available after publication.

For a per-user installation, download `install.ps1` from the [latest release](https://github.com/Shaninjah/cybersouls-taskbar-quota-hud/releases/latest), then run:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\install.ps1
```

The installer verifies the release SHA256 and installs to `%LOCALAPPDATA%\Programs\CybersoulsTaskbarQuotaHUD` without administrator access. It adds a Start menu shortcut and an entry in Windows Installed Apps.

The executable is currently **unsigned**. Windows SmartScreen or antivirus software may show a warning. For a manual download, run `Get-FileHash .\cybersouls-taskbar-quota-hud.exe -Algorithm SHA256` and compare the result with `cybersouls-taskbar-quota-hud.exe.sha256` from the same release. SHA256 checks integrity; it is not a publisher signature or independent protection against a compromised release account. See [installation and update trust](docs/installation.md#download-and-update-trust).

For portable use, download `cybersouls-taskbar-quota-hud.exe` from the same release and run it from any user-writable directory. You can also build it locally:

```powershell
cargo build --release
```

For an executable you plan to share, use `./scripts/build-release.ps1` instead. It runs `cargo build --release --locked` with dynamic source-path remapping to keep your personal build paths out of the EXE. CI and release use the same helper.

Local builds create the executable at `target\release\cybersouls-taskbar-quota-hud.exe`.

## Uninstall

Uninstall **Cybersouls Taskbar Quota HUD** from Windows Settings > Apps > Installed apps, or run:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "$env:LOCALAPPDATA\Programs\CybersoulsTaskbarQuotaHUD\uninstall.ps1"
```

Uninstalling preserves `%APPDATA%\CodexUsage\settings.json`. Add `-RemoveSettings` to delete settings explicitly. See [Installation model](docs/installation.md) for upgrade, portable, startup, and WinGet behavior.

## Use

Run:

```powershell
cybersouls-taskbar-quota-hud
```

Once running, it will appear in your taskbar and as one tray icon in the notification area.

- Drag the left divider to move the taskbar widget
- On multi-monitor setups, drag the widget onto another Windows taskbar to move it to that screen
- Right-click the taskbar widget or tray icon for refresh, monitored services, usage rows, quota alerts, update frequency, Start with Windows, reset position, language, updates, and exit
- Left-click the tray icon to toggle the taskbar widget on or off
- Enable `Start with Windows` from the right-click menu if you want it to launch automatically when you sign in

### Monitored Services

Use the right-click **Monitored services** menu to choose which independent services the widget displays. The services are not mutually exclusive, so you can monitor more than one account at the same time:

- **Codex** is enabled by default
- **Claude Code** can be enabled alongside Codex or shown by itself when Claude Code CLI is installed and authenticated
- **Antigravity** can be enabled alongside the other providers or shown by itself as its own service column

When multiple services are shown, each service has its own usage bar and matching usage text color. Antigravity prefers Google's Gemini quota summary when available and falls back to model quota data when needed.

Claude Desktop and Claude Code CLI use separate local sessions. Signing in to Claude Desktop does not enable Claude Code monitoring. When no supported Claude Code CLI credentials are available, the menu shows **Claude Code (CLI login required)** as a disabled item and automatically keeps that service off.

### System Tray Icon

The app always shows one tray icon using the same embedded icon as the executable and desktop shortcut, regardless of how many services are enabled.

Hovering over the tray icon shows a compact summary for all enabled services. Left-clicking it toggles the taskbar widget; right-clicking it opens the settings menu.

### Usage Display And Alerts

Use the right-click **Usage display** menu to show both quota rows or only one. The app always keeps at least one row visible.

Use **Quota alerts** to choose a remaining-quota threshold of 10%, 20%, or 30%. Alerts are off by default. Each provider and quota window is notified only once until its reset time changes, including across app restarts.

In Simplified Chinese, the compact taskbar rows use `5h` / `7d`, one continuous progress bar, remaining percentage, and a concrete local reset value such as `18:30重置` or `07/17重置`.

## Diagnostics

If you need to troubleshoot startup or visibility issues, run:

```powershell
cybersouls-taskbar-quota-hud --diagnose
```

This writes a log file to:

```text
%TEMP%\cybersouls-taskbar-quota-hud.log
```

The log records the application version, install channel, executable path, polling failure category, and retry timing. It does not log access tokens or credential contents. See [Troubleshooting](docs/troubleshooting.md) for the taskbar error labels and recovery steps.

Settings are saved to:

```text
%APPDATA%\CodexUsage\settings.json
```

## Account Support

Codex usage is read from the account authenticated in the local Codex installation. Optional Claude Code monitoring works with the account types supported by Claude Code.

As of **March 19, 2026**, Anthropic's Claude Code setup documentation says:

- **Supported:** Pro, Max, Teams, Enterprise, and Console accounts
- **Not supported:** the free Claude.ai plan

If Anthropic changes Claude Code availability in the future, this app should follow whatever Claude Code supports, as long as the usage data remains exposed through the same authenticated endpoints.

## Privacy And Security

This project is **open source**, so you can inspect exactly what it does.

What the app reads:

- Your local Claude Code OAuth credentials from `~/.claude/.credentials.json`
- If `CLAUDE_CONFIG_DIR` is set, the Claude Code credentials file in that directory
- If needed, the same credentials file inside an installed WSL distro
- If Codex is enabled, your local Codex credentials from `$CODEX_HOME/auth.json` or `~/.codex/auth.json`
- If Antigravity is enabled, your local Antigravity OAuth token from Windows Credential Manager target `gemini:antigravity`

What the app sends over the network:

| Provider | Domains | Purpose and credentials |
|---|---|---|
| Codex | `chatgpt.com` | Read `/backend-api/wham/usage` using the local OAuth bearer token and, when present, account ID. |
| Claude Code | `api.anthropic.com` | Read `/api/oauth/usage`; `/v1/messages` is a minimal generation fallback for rate-limit headers. Both use the local OAuth bearer token. |
| Antigravity | `daily-cloudcode-pa.googleapis.com`, `daily-cloudcode-pa.sandbox.googleapis.com`, `cloudcode-pa.googleapis.com` | Query the existing Cloud Code quota/project/model endpoints using the local OAuth bearer token. |
| Updates | `api.github.com`, `github.com`, GitHub's release-download CDN | Manual and scheduled checks, plus downloads from this fork. No provider OAuth credential is attached. |

The monitor's HTTP agents require HTTPS and use normal operating-system certificate validation. `ALL_PROXY`, `HTTPS_PROXY`, `HTTP_PROXY` and their lowercase forms may route these requests through a configured proxy. A tunneling proxy normally relays encrypted HTTPS; a proxy that terminates TLS using a certificate trusted by your system can inspect authenticated traffic. Use a proxy you trust.

What the app stores locally:

- Widget position
- Selected taskbar / screen
- Widget visibility
- Polling frequency
- Language preference
- Last update check time
- Visible quota rows and low-quota alert threshold
- Quota-window notification keys used to prevent duplicate alerts
- Enabled providers and Codex Remaining/Used display mode
- Optional diagnostic logs (`--diagnose`) and temporary updater files

What it does **not** do:

- The reviewed application code contains no Cybersouls telemetry or backend endpoint
- The monitor's own quota-request bodies do not include your project files
- OAuth credentials are not serialized into the monitor's settings or written to its diagnostic logs
- It does not directly edit your Codex credentials file
- It does not read or reuse Claude Desktop authentication data

Notes:

- Local Claude credentials may also be probed to determine CLI availability. Credentials are read into temporary in-memory strings; the monitor does not create a new persistent credential store.
- If your Claude or Codex token expires, existing refresh routines may launch the local CLI with a minimal `.` prompt. These CLIs follow their own settings and may make generation requests. The Claude Messages fallback can also consume a small amount of quota. Ordinary Codex quota GET requests and local color repainting do not submit a generation prompt. Credential updates are handled by the provider CLI, not by writing tokens into the monitor's settings.
- If your Antigravity token is expired, open Antigravity and sign in again. The monitor does not write Windows Credential Manager entries itself.
- Portable installs can update themselves by downloading the latest release from this repository
- Diagnostics can contain local paths, distro names, timestamps and quota values. Review/redact them before attaching an issue; never attach credential files. Report problems through [GitHub Issues](https://github.com/Shaninjah/cybersouls-taskbar-quota-hud/issues).

## How It Works

The monitor:

1. Finds your enabled model login credentials
2. Reads your current usage from Anthropic, ChatGPT, and/or Google's Antigravity endpoints
3. Shows the result directly in the Windows taskbar
4. Keeps the widget aligned with the selected taskbar and tray area
5. Refreshes periodically in the background

If the newer usage endpoint is unavailable, it can fall back to reading the rate-limit headers returned by Claude's Messages API.

## Open Source

This project is licensed under the MIT License. The original [LICENSE](LICENSE) and copyright notice are preserved.

This fork retains the work and attribution of [upstream-ray/codex-usage-monitor](https://github.com/upstream-ray/codex-usage-monitor), itself a maintained derivative of [CodeZeno/Claude-Code-Usage-Monitor](https://github.com/CodeZeno/Claude-Code-Usage-Monitor). Thanks to Craig Constable and the upstream contributors for the original project. Changes in this repository are not affiliated with or endorsed by the upstream maintainers, OpenAI, Anthropic, or Google.

If you want to inspect the behavior or audit the code, everything is in this repository.
