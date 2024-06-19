// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::cpu_test::CpuTestResponse;

mod cpu_test;
mod mprime;
mod process;
mod ycruncher;

pub struct AppState {
    pub test_results: Arc<Mutex<HashMap<usize, CpuTestResponse>>>,
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(AppState {
            test_results: Arc::new(Mutex::new(HashMap::new())),
        })
        .invoke_handler(tauri::generate_handler![start_test])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
async fn start_test() -> Result<(), String> {
    let core_to_test = "";

    let cores_to_test = cpu_test::get_cores_to_test(core_to_test, cpu_test::get_physical_cores());

    let config = cpu_test::CpuTestConfig {
        duration_per_core: "20s".to_string(),
        cores_to_test,
        test_methods: vec![
            cpu_test::CpuTestMethod::Prime95,
            cpu_test::CpuTestMethod::YCruncher,
        ],
    };

    // Holds the global state of the test results
    let test_results: Arc<Mutex<HashMap<usize, CpuTestResponse>>> =
        Arc::new(Mutex::new(HashMap::new()));

    cpu_test::initialize_response(&config, &test_results, &config.duration_per_core);

    cpu_test::run(config, test_results.clone());

    let rest_results = test_results.lock().unwrap();
    let mut values: Vec<&CpuTestResponse> = rest_results.values().collect();
    values.sort_by(|a, b| a.core_id.cmp(&b.core_id));
    println!();
    println!();
    println!();
    for cpu_result in values {
        println!("{:?}", cpu_result);
    }

    Ok(())
}
