#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod core;

use core::{scan_and_build, types::ScanRequest};
use std::path::PathBuf;
use tauri::path::BaseDirectory;
use tauri::Manager;

fn resolve_ioc_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let resource_path = app
        .path()
        .resolve("Multiling IOC 15.1_d.xlsx", BaseDirectory::Resource)
        .map_err(|err| err.to_string())?;
    if resource_path.exists() {
        return Ok(resource_path);
    }

    let cwd = std::env::current_dir().map_err(|err| err.to_string())?;
    let candidates = [
        cwd.join("Multiling IOC 15.1_d.xlsx"),
        cwd.join("..").join("Multiling IOC 15.1_d.xlsx"),
    ];
    for candidate in candidates {
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    Err("IOC resource not found. Ensure it is bundled with the app.".to_string())
}

fn resolve_cache_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let cache_dir = app
        .path()
        .app_cache_dir()
        .unwrap_or_else(|_| std::env::temp_dir());
    Ok(cache_dir.join("scan-cache.json"))
}

fn clear_cache_file(app: &tauri::AppHandle) {
    if let Ok(path) = resolve_cache_path(app) {
        let _ = std::fs::remove_file(path);
    }
}

#[tauri::command]
fn scan(app: tauri::AppHandle, request: ScanRequest) -> Result<core::types::ScanResponse, String> {
    let ioc_path = resolve_ioc_path(&app)?;
    let cache_path = resolve_cache_path(&app)?;
    scan_and_build(request, &ioc_path, &cache_path).map_err(|err| err.to_string())
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
    let app = tauri::Builder::default()
        .setup(|app| {
            clear_cache_file(app.handle());
            Ok(())
        })
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![scan, reveal, open_file])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|app_handle, event| {
        if let tauri::RunEvent::ExitRequested { .. } = event {
            clear_cache_file(app_handle);
        }
    });
}
