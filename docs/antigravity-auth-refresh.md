# Antigravity authentication recovery

The widget previously read only the access token from the existing
`gemini:antigravity` Windows Credential Manager entry. Once that access token
expired, quota requests failed even when Antigravity itself remained signed in.

This is a targeted backport of the original project's MIT-licensed fixes:

- [Refresh expired OAuth tokens and retry after an authentication rejection](https://github.com/CodeZeno/Claude-Code-Usage-Monitor/commit/0ae56de7897509e2f1aea0c943b2688fde52de62)
- [Keep temporary OAuth failures retryable](https://github.com/CodeZeno/Claude-Code-Usage-Monitor/commit/12291218f4641c231ad9234e83db3f607f36e89e)

The fork's direct parent, `upstream-ray/codex-usage-monitor`, still lacks this
renewal logic at commit `26f47f422c70ab99e142ea5f567d1c12506db7c3`
(checked October 31, 2026).

## Behavior

The monitor reads the optional `refresh_token` and `expiry` fields alongside
`access_token`. It renews an expired or nearly expired token (60-second margin)
before requesting quotas. When expiry is missing or unrecognized, it first tries
the existing token; authentication rejection permits one renewal and one retry.
Another rejection ends that poll without a renewal loop.

The refresh token is sent only to `https://oauth2.googleapis.com/token` in an
OAuth form request. Redirects are disabled on that request. Quota requests keep
their existing provider endpoints. Refreshed access tokens stay in memory for
the current poll; the monitor does not change the provider's stored credentials.
Legacy credentials containing only an access token remain supported.

As in the original fix, OAuth client metadata is obtained from the installed
Antigravity or agy executable rather than embedded in this repository. The
supported locations are:

- `%LOCALAPPDATA%\Programs\Antigravity\resources\bin\language_server.exe`
- `%LOCALAPPDATA%\agy\bin\agy.exe`
- `%ProgramFiles%\Antigravity\resources\bin\language_server.exe`

A rejected OAuth client pairing may fall through to another pairing found in
that installation. Network, rate-limit, server, and malformed-response failures
stop the current exchange and remain retryable instead of pausing for sign-in.
Diagnostic messages contain failure categories, without tokens, client metadata,
or raw OAuth response bodies.

## Validation

The regression tests use synthetic credentials, binary metadata and HTTP
responses. They cover expiry and its 60-second boundary, fractional timestamp
formats, refresh-only credentials, successful recovery, bounded retries, missing
refresh tokens, rejected client pairings, temporary failures, and legacy JSON.

Run them in [Docker](docker-development.md):

```powershell
docker compose exec dev cargo test --locked
```

These tests do not authenticate against Google. A native Windows smoke test is
still required to verify renewal and quota display with a real account.

For a one-shot native check, run the built executable with `--check-antigravity`.
It reads the Antigravity session and may renew it directly with Google, then
prints a JSON report containing only quota percentages and reset availability.
It exits with code 0 on success or 1 with an error category, without loading
widget settings, polling other providers, or opening the taskbar UI.

## Selecting the correct quota pool

Since local version 1.9.4, quota requests use the CLI-style User-Agent from
[CodeZeno PR #173](https://github.com/CodeZeno/Claude-Code-Usage-Monitor/pull/173).
The generic User-Agent can produce a summary containing only Claude/GPT quotas;
the old fallback then incorrectly displayed that pool as Gemini, including a
misleading 0% used and a fresh weekly countdown.

The headline Antigravity column now selects the main Gemini group explicitly.
Claude/GPT and Gemini 4 Argon are distinct pools and cannot replace it. Model
fallbacks are likewise restricted to main Gemini models. A missing main Gemini
pool is not treated as a full unused quota. Separate columns for the other pools
are outside this change.

Since local version 1.9.5, Antigravity follows the existing Remaining/Used quota
setting alongside Codex and Claude Code. With Remaining selected, 75% remaining
in Antigravity appears as 75% with a 75%-filled bar in the widget. With Used
selected, the same quota appears as 25% with a 25%-filled bar. Text, bar lengths
and tray tooltips update from cached data when the mode changes. The preference
applies in every language; provider colors, reset times and alerts are unchanged.

Adding `--diagnose` to `--check-antigravity` compares the configured endpoints and
writes only quota group names, bucket identifiers, fractions, and reset dates to
the diagnostic log. It does not include credentials or raw authentication bodies.
