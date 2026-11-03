//! Tauri shell of Local Poker Suite. This is the only crate that knows about
//! Tauri: it wires the core crates to commands, windows and events.

pub mod bus;
pub mod commands;
pub mod equity_commands;
pub mod hud;
pub mod hud_commands;
pub mod icm_commands;
pub mod import_commands;
pub mod import_service;
pub mod solver_commands;
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
        .manage(equity_commands::EquityJobs::default())
        .manage(solver_commands::SolverState::default())
        .setup(|app| {
            // HUD: window tracking, overlays, play mode, global hotkeys.
            let hud = hud::HudService::start(app.handle().clone());
            app.manage(hud.clone());
            {
                use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};
                let st = app.state::<state::AppState>();
                let toggle: Option<Shortcut> =
                    hud::shortcut_string(&hud::hotkey(&st, "toggle_hud", "Ctrl+Alt+H"))
                        .parse()
                        .ok();
                let edit: Option<Shortcut> =
                    hud::shortcut_string(&hud::hotkey(&st, "hud_edit", "Ctrl+Alt+E"))
                        .parse()
                        .ok();
                let (t2, e2, h2) = (toggle, edit, hud.clone());
                app.handle().plugin(
                    tauri_plugin_global_shortcut::Builder::new()
                        .with_handler(move |app, sc, ev| {
                            if ev.state() != ShortcutState::Pressed {
                                return;
                            }
                            if Some(sc) == t2.as_ref() {
                                h2.toggle_visible(app);
                            } else if Some(sc) == e2.as_ref() {
                                h2.toggle_edit(app);
                            }
                        })
                        .build(),
                )?;
                for sc in [toggle, edit].into_iter().flatten() {
                    if let Err(e) = app.global_shortcut().register(sc) {
                        log::warn!("global hotkey unavailable: {e}");
                    }
                }
            }
            // Forward the internal bus to every window.
            let handle = app.handle().clone();
            let mut rx = app.state::<state::AppState>().bus.subscribe();
            let st = app.state::<state::AppState>();
            let import = st.import.clone();
            let store = st.store.clone();
            let app_handle = app.handle().clone();
            let hud_handle = app.handle().clone();
            let last_autonotes = std::sync::Arc::new(std::sync::Mutex::new(
                std::time::Instant::now() - std::time::Duration::from_secs(3600),
            ));
            tauri::async_runtime::spawn(async move {
                loop {
                    match rx.recv().await {
                        Ok(ev) => {
                            // Room folders changed: restart the watcher on the new list.
                            if let bus::AppEvent::Config(
                                lps_config::ConfigEvent::Changed { family, .. }
                                | lps_config::ConfigEvent::Removed { family, .. },
                            ) = &ev
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
                            // New hands: refresh the auto-notes, at most once a minute.
                            if let Some(h) =
                                hud_handle.try_state::<std::sync::Arc<hud::HudService>>()
                            {
                                match &ev {
                                    bus::AppEvent::HandsImported(_) => h.send(hud::Msg::Data),
                                    bus::AppEvent::Config(_) => h.send(hud::Msg::Config),
                                }
                            }
                            if let bus::AppEvent::HandsImported(w) = &ev {
                                let due = last_autonotes
                                    .lock()
                                    .map(|mut t| {
                                        let due = t.elapsed() >= std::time::Duration::from_secs(60);
                                        if due {
                                            *t = std::time::Instant::now();
                                        }
                                        due
                                    })
                                    .unwrap_or(false);
                                if w.report.hands_inserted > 0 && due {
                                    let h = app_handle.clone();
                                    tauri::async_runtime::spawn_blocking(move || {
                                        let st = h.state::<state::AppState>();
                                        match tracker_commands::run_autonotes(&st) {
                                            Ok(n) => log::info!("auto-notes: {n}"),
                                            Err(e) => log::warn!("auto-notes failed: {e}"),
                                        }
                                    });
                                }
                            }
                            let payload = match &ev {
                                bus::AppEvent::Config(c) => {
                                    serde_json::to_value(c).unwrap_or_default()
                                }
                                bus::AppEvent::HandsImported(w) => {
                                    serde_json::to_value(w).unwrap_or_default()
                                }
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
            tracker_commands::tracker_opponents,
            tracker_commands::autonotes_run,
            hud_commands::hud_status,
            hud_commands::play_mode_get,
            hud_commands::hud_set_visible,
            hud_commands::hud_set_edit,
            hud_commands::hud_set_edit_table,
            hud_commands::hud_set_table_hidden,
            hud_commands::hud_overlay,
            hud_commands::hud_save_seat,
            equity_commands::equity_calc,
            equity_commands::equity_cancel,
            equity_commands::range_breakdown,
            equity_commands::range_filter,
            equity_commands::flop_texture,
            equity_commands::ranges_list,
            equity_commands::range_load,
            equity_commands::range_save,
            equity_commands::range_delete,
            icm_commands::icm_calc,
            icm_commands::pushfold_solve,
            icm_commands::icm_review_hand,
            icm_commands::icm_review_tournament,
            solver_commands::solver_estimate,
            solver_commands::solver_start,
            solver_commands::solver_progress,
            solver_commands::solver_control,
            solver_commands::solver_finish,
            solver_commands::solver_cache_list,
            solver_commands::solver_cache_load,
            solver_commands::solver_cache_delete,
            solver_commands::solver_cache_rename,
            solver_commands::solver_view,
            solver_commands::solver_breakdown,
            solver_commands::solver_export,
            solver_commands::solver_loaded_spot,
            solver_commands::solver_flops,
            solver_commands::solver_batch_start,
            solver_commands::solver_batch_progress,
            solver_commands::solver_batch_cancel,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Local Poker Suite");
}
