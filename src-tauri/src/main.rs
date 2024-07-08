// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::collections::HashMap;
use crate::cpu_test::CpuTestResponse;

mod cpu_test;
mod mprime;
mod process;
mod ycruncher;


#[derive(Debug)]
pub struct AppState {
    pub test_results: HashMap<usize, CpuTestResponse>,
    pub test_config: cpu_test::CpuTestConfig,
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![greet])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
