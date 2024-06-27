use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use gtk::{glib, prelude::*};

use crate::cpu_test::CpuTestResponse;

mod cpu_test;
mod mprime;
mod process;
mod ycruncher;

pub struct AppState {
    pub test_results: Arc<Mutex<HashMap<usize, CpuTestResponse>>>,
}

fn main() -> glib::ExitCode {
    let application = gtk::Application::builder()
        .application_id("rouhim.pbo-assistant")
        .build();
    application.connect_activate(build_ui);
    application.run()
}

fn build_ui(application: &gtk::Application) {
    let window = gtk::ApplicationWindow::new(application);

    window.set_title(Some("PBO-Assistant"));
    window.set_default_size(350, 70);

    let button = gtk::Button::with_label("Start test");
    button.connect_clicked(move |_| {
        start_test();
    });

    window.set_child(Some(&button));

    window.present();
}

fn start_test() {
    println!("Starting test");

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
}