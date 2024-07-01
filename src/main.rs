use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use gtk::{glib, prelude::*};
use crate::cpu_test::CpuTestResponse;

mod cpu_test;
mod mprime;
mod process;
mod ycruncher;
mod ui;

#[derive(Debug)]
pub struct AppState {
    pub test_results: HashMap<usize, CpuTestResponse>,
}

fn main() -> glib::ExitCode {
    // Build App State
    let app_state = AppState {
        test_results: HashMap::new(),
    };
    let app_state = Arc::new(Mutex::new(app_state));
    
    // Starts the UI and holds the main thread
    ui::start_ui_application(app_state)
}
