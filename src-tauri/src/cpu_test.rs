use std::cmp::PartialEq;
use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::io::BufRead;
use std::ops::{Div, Mul};
use std::process::Child;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use chrono::{DateTime, Utc};
use strum_macros::{Display, EnumIter, EnumString};
use sysinfo::System;

use crate::{AppState, mprime, process, ycruncher};

#[derive(Debug, Clone)]
pub struct CpuTestStatus {
    pub core_id: usize,
    pub max_clock: u64,
    pub avg_clock: u64,
    pub min_clock: u64,
    pub verification_failed: bool,
    pub method_response: HashMap<CpuTestMethod, TestMethodResponse>,
}

#[derive(Debug, Clone)]
pub struct TestMethodResponse {
    pub method: CpuTestMethod,
    pub state: CpuTestMethodStatus,
    pub current_secs: u64,
    pub total_secs: u64,
}

#[derive(Debug, Clone)]
pub struct CpuTestConfig {
    pub duration_per_core: String,
    pub cores_to_test: Vec<usize>,
    pub test_methods: Vec<CpuTestMethod>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, EnumIter, Display, EnumString)]
pub enum CpuTestMethod {
    Prime95,
    YCruncher,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, EnumIter, Display)]
pub enum CpuTestMethodStatus {
    Idle,
    Testing,
    Success,
    Failed,
}

pub fn run(app_state: Arc<Mutex<AppState>>, config: CpuTestConfig) {
    mprime::initialize();
    ycruncher::initialize();
    
    let duration = &config.duration_per_core;

    let time_to_test_per_core = parse_duration::parse(duration).unwrap();

    let cores_to_test = config.cores_to_test;

    // Print total duration
    println!(
        "\nTotal duration will be: {}\n",
        pretty_print(time_to_test_per_core.mul(cores_to_test.len() as u32))
    );

    test_cores(
        config.test_methods,
        &cores_to_test,
        time_to_test_per_core,
        app_state,
    )
}

/// Initializes the test results with the given configuration
/// The test results will be stored in the app_state
pub fn initialize_response(
    app_state: &Arc<Mutex<AppState>>,
) {
    let config = app_state.lock().unwrap().test_config.clone();
    let time_to_test_per_core = &config.duration_per_core;

    for core_id in &config.cores_to_test {
        let mut test_result = CpuTestStatus {
            core_id: *core_id,
            max_clock: u64::MIN,
            avg_clock: u64::MIN,
            min_clock: u64::MAX,
            verification_failed: false,
            method_response: HashMap::new(),
        };

        for cpu_test_method in &config.test_methods {
            let method_response = TestMethodResponse {
                method: *cpu_test_method,
                state: CpuTestMethodStatus::Idle,
                current_secs: 0,
                total_secs: parse_duration::parse(time_to_test_per_core)
                    .unwrap()
                    .as_secs(),
            };

            test_result
                .method_response
                .insert(*cpu_test_method, method_response);
        }

        app_state
            .lock()
            .unwrap()
            .test_results
            .insert(*core_id, test_result);
    }
}

/// Pretty prints the duration in a human-readable format
fn pretty_print(duration: Duration) -> String {
    let seconds = duration.as_secs();
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let seconds = seconds % 60;

    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
}

/// Gets the actual cores to test
/// If cores_to_test is empty, all physical cores will be tested
/// If cores_to_test is not empty, it will be filtered to only include physical cores
/// Also filters non-existing cores
/// Also removes duplicates and alternates the cores to test
pub fn get_cores_to_test(mut cores_to_test: Vec<usize>, physical_core_count: usize) -> Vec<usize> {
    // If cores_to_test is empty fill with all physical cores
    if cores_to_test.is_empty() {
        cores_to_test = (0..physical_core_count).collect::<Vec<_>>();
    }

    // Remove duplicates
    dedup(&mut cores_to_test);

    // Remove cores that are not available
    cores_to_test.retain(|&core| core < physical_core_count);

    // Alternate the cores to test
    // This is done to avoid testing cores that are next to each other
    cores_to_test = alternate_cores(cores_to_test);

    cores_to_test
}

// test for get_cores_to_test
#[test]
fn test_get_cores_to_test() {
    let cores_to_test = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 4, 3];
    let physical_core_count = 8;
    let cores_to_test = get_cores_to_test(cores_to_test, physical_core_count);
    assert_eq!(cores_to_test, vec![0, 7, 1, 6, 2, 5, 3, 4]);
}

fn test_cores(
    cpu_test_methods: Vec<CpuTestMethod>,
    core_ids: &Vec<usize>,
    time_to_test_per_core: Duration,
    app_state: Arc<Mutex<AppState>>,
) {
    for core_id in core_ids {
        let core_id = *core_id;

        println!("===================================");
        println!(
            "Testing core {} for {} seconds",
            core_id,
            time_to_test_per_core.as_secs()
        );
        println!("===================================");

        for cpu_test_method in &cpu_test_methods {
            let time_per_method = time_to_test_per_core.div(cpu_test_methods.len() as u32);

            println!(
                " --> Testing method {:?} for {:?}",
                cpu_test_method, time_per_method
            );

            // Test the core for the given method
            test_core_with_method(cpu_test_method, core_id, time_per_method, app_state.clone());

            // If cpu test result is failed, break earls the loop,
            // we do not need to test the other methods
            if app_state.lock().unwrap().test_results[&core_id].verification_failed {
                break;
            }

            // Wait 5 seconds to cool down the CPU
            thread::sleep(Duration::from_secs(5));
        }
    }
}

fn set_test_method_status(
    app_state: Arc<Mutex<AppState>>,
    core_id: &usize,
    cpu_test_method: &CpuTestMethod,
    status: CpuTestMethodStatus,
) {
    let mut app_state = app_state.lock().unwrap();
    let test_result = app_state.test_results.get_mut(core_id).unwrap();
    let method_response = test_result
        .method_response
        .get_mut(cpu_test_method)
        .unwrap();
    method_response.state = status;
}

fn test_core_with_method(
    cpu_test_method: &CpuTestMethod,
    core_id: usize,
    test_time: Duration,
    app_state: Arc<Mutex<AppState>>,
) {
    // Set the state of the method to TESTING
    set_test_method_status(
        app_state.clone(),
        &core_id,
        cpu_test_method,
        CpuTestMethodStatus::Testing,
    );

    // Define the shared variables
    let pid = Arc::new(Mutex::new(0));
    let time_up = Arc::new(Mutex::new(false));
    let test_program_process = Arc::new(Mutex::new(None));

    // Thread that starts the test program process
    let pid_for_core_test = pid.clone();
    let test_program_process_for_core_test = test_program_process.clone();
    let cpu_test_method = *cpu_test_method;
    let test_program_thread = thread::spawn(move || {
        start_test_program_for_core(
            cpu_test_method,
            core_id,
            pid_for_core_test,
            test_program_process_for_core_test,
        );
    });

    // Wait a bit for the test program process to start
    thread::sleep(Duration::from_secs(3));

    // Define the start and end time for the test
    let start_time = Utc::now();
    let end_time = start_time + test_time;

    // Thread that monitors the CPU usage
    let app_state_for_monitor_cpu = app_state.clone();
    let time_up_for_monitor_cpu = time_up.clone();
    let monitor_cpu_thread = thread::spawn(move || {
        monitor_cpu(core_id, time_up_for_monitor_cpu, app_state_for_monitor_cpu)
    });

    // Thread that monitors the prime95 process output for errors
    let time_up_for_monitor_process = time_up.clone();
    let app_state_for_monitor_process = app_state.clone();
    let mprime_process_for_monitor_process = test_program_process.clone();
    let monitor_process_thread = thread::spawn(move || {
        monitor_process(
            core_id,
            time_up_for_monitor_process,
            app_state_for_monitor_process,
            mprime_process_for_monitor_process,
        );
    });

    // Thread that checks if the time to test per core has passed
    let time_up_for_time_tester = time_up.clone();
    let app_state_for_time_tester = app_state.clone();
    let core_test_timer_thread = thread::spawn(move || {
        check_time_left(
            core_id,
            &cpu_test_method,
            start_time,
            end_time,
            time_up_for_time_tester,
            app_state_for_time_tester,
        );
    });

    // Wait for all threads to finish
    test_program_thread.join().unwrap();
    monitor_cpu_thread.join().unwrap();
    monitor_process_thread.join().unwrap();
    core_test_timer_thread.join().unwrap();

    // Set the state of the method to SUCCESS if the verification did not fail
    if !app_state.lock().unwrap().test_results[&core_id].verification_failed {
        set_test_method_status(
            app_state.clone(),
            &core_id,
            &cpu_test_method,
            CpuTestMethodStatus::Success,
        );
    } else {
        set_test_method_status(
            app_state.clone(),
            &core_id,
            &cpu_test_method,
            CpuTestMethodStatus::Failed,
        );
    }
}

fn alternate_cores(mut cores: Vec<usize>) -> Vec<usize> {
    cores.sort();

    let mut alt_cores: Vec<usize> = vec![];

    let mut take_first = true;
    while !cores.is_empty() {
        if take_first {
            let c = cores.first().unwrap();
            alt_cores.push(*c);
            cores.remove(0);
        } else {
            let c = cores.last().unwrap();
            alt_cores.push(*c);
            cores.remove(cores.len() - 1);
        }

        take_first = !take_first;
    }
    alt_cores
}

fn dedup<T: Eq + Hash + Copy>(v: &mut Vec<T>) {
    let mut uniques = HashSet::new();
    v.retain(|e| uniques.insert(*e));
}

fn check_time_left(
    physical_core_id: usize,
    cpu_test_method: &CpuTestMethod,
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
    time_up: Arc<Mutex<bool>>,
    app_state: Arc<Mutex<AppState>>,
) {
    loop {
        // Check if the time is up
        if Utc::now() > end_time {
            println!("Time is up");
            println!();
            *time_up.lock().unwrap() = true;

            // Kill the test program processes
            process::kill();

            break;
        }

        // Check if the verification failed
        if app_state.lock().unwrap().test_results[&physical_core_id].verification_failed {
            // Kill the rest program processes
            process::kill();

            break;
        }

        // Update current_secs
        let mut app_state = app_state.lock().unwrap();
        let test_result = app_state.test_results.get_mut(&physical_core_id).unwrap();
        let method_response = test_result
            .method_response
            .get_mut(cpu_test_method)
            .unwrap();
        method_response.current_secs = (Utc::now() - start_time).num_seconds() as u64;

        // Wait a second
        thread::sleep(Duration::from_secs(1));
    }
}

fn start_test_program_for_core(
    cpu_test_method: CpuTestMethod,
    core_id: usize,
    pid: Arc<Mutex<u32>>,
    test_program_process: Arc<Mutex<Option<Child>>>,
) {
    let child = match cpu_test_method {
        CpuTestMethod::Prime95 => mprime::start_verification(core_id),
        CpuTestMethod::YCruncher => ycruncher::start_verification(core_id),
    };

    // Set the pid of the child process
    let mut pid_handle = pid.lock().unwrap();
    *pid_handle = child.id();

    // Store the child process in the arc mutex
    let mut test_program_process_handle = test_program_process.lock().unwrap();
    *test_program_process_handle = Some(child);
}

fn monitor_cpu(
    physical_core_id: usize,
    time_up: Arc<Mutex<bool>>,
    app_state: Arc<Mutex<AppState>>,
) {
    let mut sys = System::new();
    loop {
        // Check if time is up or if the verification failed
        let verification_failed =
            app_state.lock().unwrap().test_results[&physical_core_id].verification_failed;
        if *time_up.lock().unwrap() || verification_failed {
            break;
        }

        // Collect CPU metrics
        sys.refresh_cpu();
        let logical_core_id = physical_core_id * 2;
        let current_freq = sys.cpus()[logical_core_id].frequency();

        // Update clocks in the test results
        let mut app_state = app_state.lock().unwrap();
        let test_result = app_state.test_results.get_mut(&physical_core_id).unwrap();
        test_result.max_clock = std::cmp::max(test_result.max_clock, current_freq);
        test_result.min_clock = std::cmp::min(test_result.min_clock, current_freq);
        test_result.avg_clock = (test_result.avg_clock + current_freq) / 2;

        // Wait a second
        thread::sleep(Duration::from_secs(1));
    }
}

fn monitor_process(
    physical_core_id: usize,
    time_up: Arc<Mutex<bool>>,
    app_state: Arc<Mutex<AppState>>,
    mprime_process: Arc<Mutex<Option<Child>>>,
) {
    if let Some(mprime_process) = &mut *mprime_process.lock().unwrap() {
        let stdout = mprime_process.stdout.as_mut().unwrap();
        let reader = std::io::BufReader::new(stdout);
        let lines = reader.lines();

        for line in lines {
            // if time is up, break
            if *time_up.lock().unwrap() {
                break;
            }

            let line = line.unwrap();
            //println!("{}", line);

            if line.contains(mprime::ERROR_MESSAGE) || line.contains(ycruncher::ERROR_MESSAGE) {
                println!("#############");
                println!("Verification failed for core {}", physical_core_id);
                println!("#############");

                // Set the verification failed flag
                let mut app_state = app_state.lock().unwrap();
                let test_result = app_state.test_results.get_mut(&physical_core_id).unwrap();
                test_result.verification_failed = true;

                break;
            }
        }
    }
}

pub fn get_physical_cores() -> usize {
    let mut sys = System::new();
    sys.refresh_cpu();
    sys.physical_core_count().unwrap()
}
