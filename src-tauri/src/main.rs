// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use strum::IntoEnumIterator;

use crate::cpu_test::CpuTestStatus;

mod cpu_test;
mod mprime;
mod process;
mod ycruncher;


#[derive(Debug)]
pub struct AppState {
    pub core_status: Arc<Mutex<HashMap<usize, CpuTestStatus>>>,
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![get_test_methods])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
async fn get_test_methods() -> Result<String, String> {
    let cpu_test_methods = cpu_test::CpuTestMethod::iter()
        .map(|method| method.to_string())
        .collect::<Vec<String>>();
    Ok(serde_json::to_string(&cpu_test_methods).unwrap())
}

#[tauri::command]
fn start_test(
    test_methods: Vec<String>,
    duration_per_core: String,
    cores_to_test: String,
    app_state: tauri::State<AppState>,
) -> Result<(), ()> {

    let test_methods = test_methods
        .iter()
        .map(|method| cpu_test::CpuTestMethod::from_str(method).unwrap())
        .collect::<Vec<cpu_test::CpuTestMethod>>();

    let config = cpu_test::CpuTestConfig {
        test_methods,
        duration_per_core: duration_per_core.parse().unwrap(),
        cores_to_test: cores_to_test.parse().unwrap(),
    };
    app_state.config = config;
    
    cpu_test::run();

    Ok(())
}