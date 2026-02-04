#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod core;

use core::{scan_and_build, types::ScanRequest};

#[tauri::command]
fn scan(request: ScanRequest) -> Result<core::types::ScanResponse, String> {
    scan_and_build(request).map_err(|err| err.to_string())
}

#[tauri::command]
fn reveal(path: String) -> Result<(), String> {
    core::locator::reveal_in_file_manager(path).map_err(|err| err.to_string())
}

#[tauri::command]
fn open_file(path: String) -> Result<(), String> {
    core::locator::open_file(path).map_err(|err| err.to_string())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![scan, reveal, open_file])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
