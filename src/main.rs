use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use gtk::glib;
use strum::IntoEnumIterator;

use crate::cpu_test::CpuTestResponse;

mod cpu_test;
mod mprime;
mod process;
mod ui;
mod ycruncher;

#[derive(Debug)]
pub struct AppState {
    pub test_results: HashMap<usize, CpuTestResponse>,
    pub test_config: cpu_test::CpuTestConfig,
    // FIXME: gtk widget ist nicht transportsicher, muss evtl geboxt werden
    pub ui_elements: HashMap<String, gtk::Widget>,
}

fn main() -> glib::ExitCode {
    // Build App State
    // TODO: Load and save config
    let config = cpu_test::CpuTestConfig {
        duration_per_core: "10m".to_string(),
        cores_to_test: vec![],
        test_methods: cpu_test::CpuTestMethod::iter().collect(),
    };
    let app_state = AppState {
        test_results: HashMap::new(),
        test_config: config,
        ui_elements: HashMap::new(),
    };
    let app_state = Arc::new(Mutex::new(app_state));

    // Starts the UI and holds the main thread
    ui::start_ui_application(app_state)
}
