use crate::cpu_test::CpuTestResponse;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

mod cpu_test;
mod mprime;
mod process;
mod ycruncher;

fn main() {
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
