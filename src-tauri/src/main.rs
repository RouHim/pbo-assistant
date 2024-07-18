// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::collections::HashMap;
use std::ops::Deref;
use std::str::FromStr;
use std::sync::{Arc, RwLock};
use std::thread;
use strum::IntoEnumIterator;

use cpu_test::AppState;

use crate::cpu_test::CpuTestStatus;

mod cpu_info;
mod cpu_test;
mod mprime;
mod process;
mod ycruncher;

fn main() {
    tauri::Builder::default()
        .manage(AppState {
            test_status: Arc::new(RwLock::new(HashMap::new())),
            terminated_by_user: Arc::new(RwLock::new(false)),
        })
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            get_test_methods,
            start_test,
            stop_test,
            get_test_status,
            get_physical_cores,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
async fn get_physical_cores() -> Result<usize, ()> {
    Ok(cpu_info::get_physical_cores())
}

#[tauri::command]
async fn stop_test(app_state: tauri::State<'_, AppState>) -> Result<(), ()> {
    let state = app_state.deref();
    cpu_test::stop(state.clone());
    Ok(())
}

#[tauri::command]
async fn get_test_status(app_state: tauri::State<'_, AppState>) -> Result<String, ()> {
    // Read values from state
    let state = app_state.deref();
    let test_status = state.test_status.read().unwrap();
    let mut test_status_values: Vec<&CpuTestStatus> = test_status.deref().values().collect();

    // Sort by core id
    test_status_values.sort_by(|a, b| a.core_id.cmp(&b.core_id));

    Ok(serde_json::to_string(&test_status_values).unwrap())
}

#[tauri::command]
async fn get_test_methods() -> Result<String, String> {
    let cpu_test_methods = cpu_test::CpuTestMethod::iter()
        .map(|method| method.to_string())
        .collect::<Vec<String>>();
    Ok(serde_json::to_string(&cpu_test_methods).unwrap())
}

#[tauri::command]
async fn start_test(
    test_methods: Vec<String>,
    duration_per_core: String,
    cores_to_test: String,
    app_state: tauri::State<'_, AppState>,
) -> Result<(), ()> {
    let test_methods = test_methods
        .iter()
        .map(|method| cpu_test::CpuTestMethod::from_str(method).unwrap())
        .collect::<Vec<cpu_test::CpuTestMethod>>();

    // Cleanup and validate cores to test
    let cores_to_test = cpu_test::get_cores_to_test(cores_to_test, cpu_info::get_physical_cores());

    let config = cpu_test::CpuTestConfig {
        test_methods,
        duration_per_core: duration_per_core.parse().unwrap(),
        cores_to_test,
    };

    let app_state = app_state.deref();

    // Reset terminated by user flag
    *app_state.terminated_by_user.write().unwrap() = false;

    cpu_test::initialize_response(&app_state.test_status, &config);

    let core_status = app_state.clone();
    thread::spawn(move || {
        cpu_test::run(core_status, &config);
    });

    Ok(())
}
