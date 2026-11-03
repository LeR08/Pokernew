//! Tauri shell of Local Poker Suite. This is the only crate that knows about
//! Tauri: it wires the core crates to commands, windows and events.

pub mod bus;
pub mod commands;
pub mod state;

use std::time::Instant;

use lps_config::paths::ResolveInput;
use tauri::{Emitter, Manager};

/// Builds and runs the app. `started` is taken first thing in `main`.
pub fn run(started: Instant) {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let state = match state::AppState::init(&ResolveInput::from_process(), started) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Local Poker Suite cannot start: {e}");
            std::process::exit(1);
        }
    };
    log::info!(
        "data folder {} ({:?}), backend ready in {} ms",
        state.data_dir.path.display(),
        state.data_dir.source,
        state.backend_ready_ms
    );

    tauri::Builder::default()
        .manage(state)
        .setup(|app| {
            // Forward the internal bus to every window.
            let handle = app.handle().clone();
            let mut rx = app.state::<state::AppState>().bus.subscribe();
            tauri::async_runtime::spawn(async move {
                loop {
                    match rx.recv().await {
                        Ok(ev) => {
                            let payload = match &ev {
                                bus::AppEvent::Config(c) => serde_json::to_value(c).unwrap_or_default(),
                            };
                            let _ = handle.emit(ev.channel(), payload);
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                            log::warn!("event bus lagged by {n} events");
                        }
                        Err(_) => break,
                    }
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_info,
            commands::config_get,
            commands::config_list,
            commands::config_raw,
            commands::config_validate,
            commands::config_save_text,
            commands::config_save_value,
            commands::config_delete,
            commands::config_issues,
            commands::schema_get,
            commands::frontend_ready,
            commands::open_module_window,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Local Poker Suite");
}
