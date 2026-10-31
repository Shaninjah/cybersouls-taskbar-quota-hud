#![windows_subsystem = "windows"]

mod diagnose;
mod localization;
mod models;
mod native_interop;
mod poller;
mod quota_colors;
mod quota_display;
mod theme;
mod tray_icon;
mod updater;
mod window;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|arg| arg == "--check-antigravity") {
        if args.iter().any(|arg| arg == "--diagnose") {
            let _ = diagnose::init();
        }
        // One provider only, without loading settings or starting the taskbar UI.
        // The report deliberately contains no credentials or raw HTTP errors.
        let (code, report) = match poller::poll(false, false, true) {
            Ok(data) => match data.antigravity {
                Some(usage) => (
                    0,
                    serde_json::json!({
                        "status": "ok",
                        "session_used_percent": usage.session.percentage,
                        "weekly_used_percent": usage.weekly.percentage,
                        "session_reset_known": usage.session.resets_at.is_some(),
                        "weekly_reset_known": usage.weekly.resets_at.is_some(),
                    }),
                ),
                None => (
                    1,
                    serde_json::json!({"status":"error", "category":"invalid_response"}),
                ),
            },
            Err(error) => (
                1,
                serde_json::json!({"status":"error", "category":error.category()}),
            ),
        };
        println!("{report}");
        std::process::exit(code);
    }
    let diagnose_enabled = args.iter().any(|arg| arg == "--diagnose");
    if diagnose_enabled {
        match diagnose::init() {
            Ok(_) => {
                // Arbitrary CLI arguments can contain user-provided sensitive data.
                diagnose::log("startup");
                diagnose::log(format!(
                    "version={} install_channel={:?} executable={}",
                    env!("CARGO_PKG_VERSION"),
                    updater::current_install_channel(),
                    std::env::current_exe()
                        .map(|value| value.display().to_string())
                        .unwrap_or_else(|error| format!("unavailable:{error}"))
                ));
            }
            Err(error) => {
                // Logging may not be available yet, but keep startup behavior unchanged.
                let _ = error;
            }
        }
    }

    if let Some(exit_code) = updater::handle_cli_mode(&args) {
        if diagnose_enabled {
            diagnose::log(format!("cli mode exited with code {exit_code}"));
        }
        std::process::exit(exit_code);
    }

    if diagnose_enabled {
        diagnose::log("entering window::run");
    }
    window::run();
}
