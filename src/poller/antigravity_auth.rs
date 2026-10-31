//! Antigravity OAuth recovery, adapted from CodeZeno/Claude-Code-Usage-Monitor
//! commits 0ae56de7897509e2f1aea0c943b2688fde52de62 and
//! 12291218f4641c231ad9234e83db3f607f36e89e (MIT).
//! Provider credentials and refreshed tokens stay in memory.

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use serde::Deserialize;

use super::{
    classify_http_status, classify_ureq_error, fetch_antigravity_usage, parse_iso8601,
    read_windows_generic_credential, PollError, ANTIGRAVITY_CREDENTIAL_TARGET,
};
use crate::diagnose;
use crate::models::UsageData;

const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const EXPIRY_SKEW: Duration = Duration::from_secs(60);

#[derive(Deserialize)]
struct AntigravityAuthFile {
    token: AntigravityTokenData,
}

#[derive(Deserialize)]
struct AntigravityTokenData {
    #[serde(default)]
    access_token: String,
    refresh_token: Option<String>,
    expiry: Option<String>,
}

#[derive(Deserialize)]
struct RefreshResponse {
    access_token: String,
}

pub(super) fn poll() -> Result<UsageData, PollError> {
    let creds = read_credentials().ok_or_else(|| {
        diagnose::log("Antigravity usage poll failed: no supported credentials found");
        PollError::NoCredentials
    })?;
    poll_with_refresh(
        &creds,
        SystemTime::now(),
        fetch_antigravity_usage,
        refresh_antigravity_token,
    )
}

fn parse_credentials(content: &str) -> Option<AntigravityTokenData> {
    let auth: AntigravityAuthFile = serde_json::from_str(content).ok()?;
    (!auth.token.access_token.trim().is_empty()
        || auth
            .token
            .refresh_token
            .as_deref()
            .is_some_and(|token| !token.trim().is_empty()))
    .then_some(auth.token)
}

fn read_credentials() -> Option<AntigravityTokenData> {
    let content = read_windows_generic_credential(ANTIGRAVITY_CREDENTIAL_TARGET)?;
    parse_credentials(&content)
}

fn poll_with_refresh(
    creds: &AntigravityTokenData,
    now: SystemTime,
    mut fetch: impl FnMut(&str) -> Result<UsageData, PollError>,
    mut refresh: impl FnMut(&str) -> Result<String, PollError>,
) -> Result<UsageData, PollError> {
    let refresh_token = creds
        .refresh_token
        .as_deref()
        .filter(|value| !value.trim().is_empty());
    let expiring = creds.access_token.trim().is_empty()
        || parse_iso8601(creds.expiry.as_deref())
            .is_some_and(|expiry| expiry <= now.checked_add(EXPIRY_SKEW).unwrap_or(now));
    let mut token = creds.access_token.clone();
    let mut refreshed = false;
    if expiring {
        if let Some(secret) = refresh_token {
            token = refresh(secret)?;
            refreshed = true;
        }
    }
    if token.trim().is_empty() {
        return Err(PollError::AuthRequired);
    }
    match fetch(&token) {
        Err(PollError::AuthRequired) if !refreshed => {
            let Some(refresh_token) = refresh_token else {
                return Err(PollError::AuthRequired);
            };
            let token = refresh(refresh_token)?;
            fetch(&token)
        }
        result => result,
    }
}

fn installed_oauth_clients() -> Vec<(String, String)> {
    let mut paths = Vec::new();
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        let local = PathBuf::from(local);
        paths.push(local.join("Programs/Antigravity/resources/bin/language_server.exe"));
        paths.push(local.join("agy/bin/agy.exe"));
    }
    if let Some(program_files) = std::env::var_os("ProgramFiles") {
        paths.push(
            PathBuf::from(program_files).join("Antigravity/resources/bin/language_server.exe"),
        );
    }
    for path in paths {
        if let Ok(bytes) = fs::read(path) {
            let clients = oauth_clients_from_binary(&bytes);
            if !clients.is_empty() {
                return clients;
            }
        }
    }
    Vec::new()
}

fn oauth_clients_from_binary(bytes: &[u8]) -> Vec<(String, String)> {
    const CLIENT_ID_SUFFIX: &str = ".apps.googleusercontent.com";
    const CLIENT_SECRET_PREFIX: &str = "GOCSPX-";
    let mut ids = Vec::new();
    let mut secrets = Vec::new();
    for run in
        bytes.split(|byte| !byte.is_ascii_alphanumeric() && !matches!(*byte, b'.' | b'_' | b'-'))
    {
        for (suffix_at, _) in run
            .windows(CLIENT_ID_SUFFIX.len())
            .enumerate()
            .filter(|(_, part)| *part == CLIENT_ID_SUFFIX.as_bytes())
        {
            for (hyphen, byte) in run[..suffix_at].iter().enumerate() {
                if *byte != b'-' {
                    continue;
                }
                let mut start = hyphen;
                while start > 0 && run[start - 1].is_ascii_digit() {
                    start -= 1;
                }
                let client_hash = &run[hyphen + 1..suffix_at];
                if hyphen - start < 10
                    || !(20..=80).contains(&client_hash.len())
                    || !client_hash
                        .iter()
                        .all(|byte| byte.is_ascii_alphanumeric() || matches!(*byte, b'_' | b'-'))
                {
                    continue;
                }
                if let Ok(client_id) =
                    std::str::from_utf8(&run[start..suffix_at + CLIENT_ID_SUFFIX.len()])
                {
                    if !ids.iter().any(|existing| existing == client_id) {
                        ids.push(client_id.to_owned());
                    }
                }
            }
        }
        for (start, _) in run
            .windows(CLIENT_SECRET_PREFIX.len())
            .enumerate()
            .filter(|(_, part)| *part == CLIENT_SECRET_PREFIX.as_bytes())
        {
            let Some(candidate) = run.get(start..start + 35) else {
                continue;
            };
            if candidate
                .iter()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(*byte, b'_' | b'-'))
            {
                if let Ok(secret) = std::str::from_utf8(candidate) {
                    if !secrets.iter().any(|existing| existing == secret) {
                        secrets.push(secret.to_owned());
                    }
                }
            }
        }
    }
    ids.into_iter()
        .flat_map(|id| {
            secrets
                .iter()
                .cloned()
                .map(move |secret| (id.clone(), secret))
        })
        .collect()
}

fn build_refresh_agent() -> Result<ureq::Agent, PollError> {
    let tls = native_tls::TlsConnector::new().map_err(|_| PollError::RequestFailed)?;
    Ok(ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(30))
        .https_only(true)
        // OAuth form fields must never be forwarded to a redirect destination.
        .redirects(0)
        .tls_connector(Arc::new(tls))
        .build())
}

fn refresh_antigravity_token(refresh_token: &str) -> Result<String, PollError> {
    let agent = build_refresh_agent()?;
    refresh_from_clients(installed_oauth_clients(), |client_id, client_secret| {
        parse_refresh_response(agent.post(GOOGLE_TOKEN_URL).send_form(&[
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("refresh_token", refresh_token),
            ("grant_type", "refresh_token"),
        ]))
    })
}

fn refresh_from_clients(
    clients: Vec<(String, String)>,
    mut exchange: impl FnMut(&str, &str) -> Result<String, PollError>,
) -> Result<String, PollError> {
    for (client_id, client_secret) in clients {
        match exchange(&client_id, &client_secret) {
            // Another installed client pairing may have issued this token.
            Err(PollError::AuthRequired) => continue,
            // Keep network, rate-limit and server failures retryable.
            result => return result,
        }
    }
    diagnose::log("Antigravity OAuth refresh failed: auth_required");
    Err(PollError::AuthRequired)
}

fn parse_refresh_response(
    response: Result<ureq::Response, ureq::Error>,
) -> Result<String, PollError> {
    let response = match response {
        Ok(response) => response,
        Err(ureq::Error::Status(400, response)) => {
            #[derive(Deserialize)]
            struct OAuthError {
                error: String,
            }
            let error: OAuthError = response.into_json().map_err(|_| PollError::RequestFailed)?;
            return Err(match error.error.as_str() {
                "invalid_grant" | "invalid_client" | "unauthorized_client" => {
                    PollError::AuthRequired
                }
                _ => PollError::RequestFailed,
            });
        }
        // Log neither raw HTTP errors nor OAuth response bodies.
        Err(error) => return Err(classify_ureq_error(&error)),
    };
    if !(200..300).contains(&response.status()) {
        return Err(classify_http_status(response.status()));
    }
    let token: RefreshResponse = response.into_json().map_err(|_| PollError::RequestFailed)?;
    if token.access_token.trim().is_empty() {
        return Err(PollError::RequestFailed);
    }
    Ok(token.access_token)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    fn now() -> SystemTime {
        parse_iso8601(Some("2030-01-01T00:00:00Z")).unwrap()
    }

    fn credentials(expiry: Option<&str>) -> AntigravityTokenData {
        AntigravityTokenData {
            access_token: "old".into(),
            refresh_token: Some("refresh".into()),
            expiry: expiry.map(str::to_owned),
        }
    }

    fn response(status: u16, body: &str) -> Result<ureq::Response, ureq::Error> {
        let response = ureq::Response::new(status, "test", body).unwrap();
        if status >= 400 {
            Err(ureq::Error::Status(status, response))
        } else {
            Ok(response)
        }
    }

    #[test]
    fn existing_and_refresh_only_credential_formats_remain_supported() {
        let old = parse_credentials(r#"{"token":{"access_token":"old"}}"#).unwrap();
        assert_eq!(old.access_token, "old");
        assert!(old.refresh_token.is_none());
        let current = parse_credentials(
            r#"{"token":{"access_token":"old","refresh_token":"refresh","expiry":"2020-01-01T00:00:00Z"}}"#,
        ).unwrap();
        assert_eq!(current.refresh_token.as_deref(), Some("refresh"));
        assert!(parse_credentials(r#"{"token":{"refresh_token":"refresh"}}"#).is_some());
        for invalid in [
            "not JSON",
            "{}",
            r#"{"token":{}}"#,
            r#"{"token":{"access_token":" "}}"#,
        ] {
            assert!(parse_credentials(invalid).is_none());
        }
    }

    #[test]
    fn valid_or_unknown_expiry_uses_existing_token_without_refresh() {
        for expiry in [None, Some("not a date"), Some("2030-01-01T00:01:01Z")] {
            let result = poll_with_refresh(
                &credentials(expiry),
                now(),
                |token| {
                    assert_eq!(token, "old");
                    Ok(UsageData::default())
                },
                |_| panic!("valid or unknown expiry must first try the stored token"),
            );
            assert!(result.is_ok());
        }
    }

    #[test]
    fn expired_and_nearly_expired_tokens_refresh_before_usage_request() {
        for expiry in [
            "2020-01-01T00:00:00Z",
            "2020-01-01T00:00:00.1234567+00:00",
            "2030-01-01T00:01:00Z",
        ] {
            let refreshes = Cell::new(0);
            let result = poll_with_refresh(
                &credentials(Some(expiry)),
                now(),
                |token| {
                    assert_eq!(token, "new");
                    Ok(UsageData::default())
                },
                |secret| {
                    assert_eq!(secret, "refresh");
                    refreshes.set(refreshes.get() + 1);
                    Ok("new".into())
                },
            );
            assert!(result.is_ok());
            assert_eq!(refreshes.get(), 1);
        }
    }

    #[test]
    fn empty_access_token_can_recover_using_refresh_token() {
        let mut creds = credentials(None);
        creds.access_token.clear();
        assert!(poll_with_refresh(
            &creds,
            now(),
            |token| {
                assert_eq!(token, "new");
                Ok(UsageData::default())
            },
            |_| Ok("new".into()),
        )
        .is_ok());
    }

    #[test]
    fn unauthorized_refreshes_and_retries_only_once() {
        let calls = Cell::new(0);
        let refreshes = Cell::new(0);
        let result = poll_with_refresh(
            &credentials(None),
            now(),
            |_| {
                calls.set(calls.get() + 1);
                Err(PollError::AuthRequired)
            },
            |_| {
                refreshes.set(refreshes.get() + 1);
                Ok("new".into())
            },
        );
        assert_eq!(result.unwrap_err(), PollError::AuthRequired);
        assert_eq!(calls.get(), 2);
        assert_eq!(refreshes.get(), 1);
    }

    #[test]
    fn unauthorized_usage_recovers_with_the_replacement_token() {
        let calls = Cell::new(0);
        let result = poll_with_refresh(
            &credentials(None),
            now(),
            |token| {
                calls.set(calls.get() + 1);
                if token == "old" {
                    Err(PollError::AuthRequired)
                } else {
                    assert_eq!(token, "new");
                    let mut data = UsageData::default();
                    data.session.percentage = 42.0;
                    Ok(data)
                }
            },
            |_| Ok("new".into()),
        )
        .unwrap();
        assert_eq!(result.session.percentage, 42.0);
        assert_eq!(calls.get(), 2);
    }

    #[test]
    fn proactively_refreshed_token_is_not_refreshed_twice() {
        let calls = Cell::new(0);
        let refreshes = Cell::new(0);
        let result = poll_with_refresh(
            &credentials(Some("2020-01-01T00:00:00Z")),
            now(),
            |_| {
                calls.set(calls.get() + 1);
                Err(PollError::AuthRequired)
            },
            |_| {
                refreshes.set(refreshes.get() + 1);
                Ok("new".into())
            },
        );
        assert_eq!(result.unwrap_err(), PollError::AuthRequired);
        assert_eq!(calls.get(), 1);
        assert_eq!(refreshes.get(), 1);
    }

    #[test]
    fn missing_refresh_token_preserves_auth_failure() {
        let mut creds = credentials(None);
        creds.refresh_token = None;
        let result = poll_with_refresh(
            &creds,
            now(),
            |_| Err(PollError::AuthRequired),
            |_| panic!("there is no refresh token"),
        );
        assert_eq!(result.unwrap_err(), PollError::AuthRequired);
    }

    #[test]
    fn transient_usage_failures_do_not_trigger_oauth_refresh() {
        for error in [
            PollError::NetworkUnavailable,
            PollError::RateLimited,
            PollError::ServerError,
        ] {
            let result = poll_with_refresh(
                &credentials(None),
                now(),
                |_| Err(error),
                |_| panic!("usage failure is unrelated to authentication"),
            );
            assert_eq!(result.unwrap_err(), error);
        }
    }

    #[test]
    fn transient_refresh_failures_stop_client_attempts_and_remain_retryable() {
        for error in [
            PollError::NetworkUnavailable,
            PollError::RateLimited,
            PollError::ServerError,
            PollError::RequestFailed,
        ] {
            let attempts = Cell::new(0);
            let result = poll_with_refresh(
                &credentials(Some("2020-01-01T00:00:00Z")),
                now(),
                |_| panic!("usage must wait for successful refresh"),
                |_| {
                    refresh_from_clients(
                        vec![
                            ("first".into(), "secret".into()),
                            ("second".into(), "secret".into()),
                        ],
                        |_, _| {
                            attempts.set(attempts.get() + 1);
                            Err(error)
                        },
                    )
                },
            );
            assert_eq!(result.unwrap_err(), error);
            assert_eq!(attempts.get(), 1);
        }
    }

    #[test]
    fn rejected_client_pair_falls_through_and_missing_clients_require_sign_in() {
        assert_eq!(
            refresh_from_clients(
                vec![
                    ("first".into(), "secret".into()),
                    ("second".into(), "secret".into())
                ],
                |id, _| if id == "first" {
                    Err(PollError::AuthRequired)
                } else {
                    Ok("new".into())
                },
            ),
            Ok("new".into())
        );
        assert_eq!(
            refresh_from_clients(Vec::new(), |_, _| panic!("no clients available")),
            Err(PollError::AuthRequired)
        );
    }

    #[test]
    fn refresh_responses_keep_retryable_failures_separate_from_rejected_credentials() {
        assert_eq!(
            parse_refresh_response(response(200, r#"{"access_token":"new"}"#)),
            Ok("new".into())
        );
        for status in [401, 403] {
            assert_eq!(
                parse_refresh_response(response(status, "")),
                Err(PollError::AuthRequired)
            );
        }
        for error in ["invalid_grant", "invalid_client", "unauthorized_client"] {
            let body = serde_json::json!({"error":error}).to_string();
            assert_eq!(
                parse_refresh_response(response(400, &body)),
                Err(PollError::AuthRequired)
            );
        }
        for (status, error) in [
            (429, PollError::RateLimited),
            (500, PollError::ServerError),
            (503, PollError::ServerError),
        ] {
            assert_eq!(parse_refresh_response(response(status, "")), Err(error));
        }
        for body in [
            "not JSON",
            "{}",
            r#"{"access_token":""}"#,
            r#"{"access_token":" "}"#,
        ] {
            assert_eq!(
                parse_refresh_response(response(200, body)),
                Err(PollError::RequestFailed)
            );
        }
        assert_eq!(
            parse_refresh_response(response(400, r#"{"error":"temporarily_unavailable"}"#)),
            Err(PollError::RequestFailed)
        );
        assert_eq!(
            parse_refresh_response(response(302, "")),
            Err(PollError::RequestFailed)
        );
    }

    #[test]
    fn oauth_client_extraction_uses_only_synthetic_binary_metadata() {
        let mut bytes =
            b"123456789012-hash_12345678901234567890.apps.googleusercontent.com".to_vec();
        bytes.extend_from_slice(b"GOCSPX-");
        bytes.resize(bytes.len() + 28, b'1');
        assert_eq!(oauth_clients_from_binary(&bytes).len(), 1);
        let duplicate = bytes.clone();
        bytes.push(0);
        bytes.extend_from_slice(&duplicate);
        assert_eq!(oauth_clients_from_binary(&bytes).len(), 1);
        for invalid in [b"unrelated binary data".as_slice(), b"GOCSPX-short"] {
            assert!(oauth_clients_from_binary(invalid).is_empty());
        }
    }
}
