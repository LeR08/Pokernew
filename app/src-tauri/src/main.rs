// Hide the console window on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let started = std::time::Instant::now();
    lps_app::run(started);
}
