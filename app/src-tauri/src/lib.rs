//! Tauri shell of Local Poker Suite. This is the only crate that knows about
//! Tauri: it wires the core crates to commands, windows and events.

pub mod bus;
pub mod commands;
pub mod import_commands;
pub mod import_service;
pub mod state;
pub mod tracker_commands;

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
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(state)
        .setup(|app| {
            // Forward the internal bus to every window.
            let handle = app.handle().clone();
            let mut rx = app.state::<state::AppState>().bus.subscribe();
            let st = app.state::<state::AppState>();
            let import = st.import.clone();
            let store = st.store.clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    match rx.recv().await {
                        Ok(ev) => {
                            // Room folders changed: restart the watcher on the new list.
                            if let bus::AppEvent::Config(lps_config::ConfigEvent::Changed { family, .. }
                            | lps_config::ConfigEvent::Removed { family, .. }) = &ev
                            {
                                if family == "rooms" || family == "settings" {
                                    if let Some(svc) = import.clone() {
                                        let store = store.clone();
                                        tauri::async_runtime::spawn_blocking(move || {
                                            if let Err(e) = svc.restart_watcher(&store) {
                                                log::warn!("watcher restart failed: {e}");
                                            }
                                        });
                                    }
                                }
                            }
                            let payload = match &ev {
                                bus::AppEvent::Config(c) => serde_json::to_value(c).unwrap_or_default(),
                                bus::AppEvent::HandsImported(w) => serde_json::to_value(w).unwrap_or_default(),
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
            import_commands::db_stats,
            import_commands::import_status,
            import_commands::import_paths,
            import_commands::import_errors,
            import_commands::import_retry_errors,
            import_commands::import_errors_export,
            import_commands::detect_rooms,
            import_commands::backup_list,
            import_commands::backup_now,
            import_commands::backup_restore,
            import_commands::app_restart,
            tracker_commands::tracker_report,
            tracker_commands::tracker_graph,
            tracker_commands::tracker_tournaments,
            tracker_commands::tracker_hands,
            tracker_commands::tracker_accounts,
            tracker_commands::hand_get,
            tracker_commands::stat_preview,
            tracker_commands::tracker_sessions,
            tracker_commands::session_review,
            tracker_commands::tracker_leaks,
            tracker_commands::tracker_grid,
            tracker_commands::notes_get,
            tracker_commands::note_add,
            tracker_commands::note_delete,
            tracker_commands::player_colour,
            tracker_commands::bankroll_get,
            tracker_commands::bankroll_tx_list,
            tracker_commands::bankroll_tx_add,
            tracker_commands::bankroll_tx_delete,
            tracker_commands::export_csv,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Local Poker Suite");
}
