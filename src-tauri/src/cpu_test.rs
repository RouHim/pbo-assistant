use std::cmp::PartialEq;
use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::io::BufRead;
use std::ops::{Div, Mul};
use std::process::Child;
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::Serialize;
use strum_macros::{Display, EnumIter, EnumString};

use crate::{cpu_info, mprime, process, ycruncher};

#[derive(Debug, Clone)]
pub struct AppState {
    pub test_status: Arc<RwLock<HashMap<usize, CpuTestStatus>>>,
    pub terminated_by_user: Arc<RwLock<bool>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CpuTestStatus {
    pub core_id: usize,
    pub max_clock: u64,
    pub avg_clock: u64,
    pub min_clock: u64,
    pub verification_failed: bool,
    pub method_response: HashMap<CpuTestMethod, TestMethodResponse>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TestMethodResponse {
    pub method: CpuTestMethod,
    pub state: CpuTestMethodStatus,
    pub current_secs: u64,
    pub total_secs: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct CpuTestConfig {
    pub duration_per_core: String,
    pub cores_to_test: Vec<usize>,
    pub test_methods: Vec<CpuTestMethod>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, EnumIter, Display, EnumString, Serialize)]
pub enum CpuTestMethod {
    Prime95,
    YCruncher,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, EnumIter, Display, Serialize)]
pub enum CpuTestMethodStatus {
    Idle,
    Testing,
    Success,
    Failed,
}

pub fn run(app_state: AppState, config: &CpuTestConfig) {
    mprime::initialize();
    ycruncher::initialize();

    let duration = &config.duration_per_core;

    let time_to_test_per_core = parse_duration::parse(duration).unwrap();

    let cores_to_test = &config.cores_to_test;

    // Print total duration
    println!(
        "\nTotal duration will be: {}\n",
        pretty_print(time_to_test_per_core.mul(cores_to_test.len() as u32))
    );

    test_cores(
        &config.test_methods,
        cores_to_test.clone(),
        time_to_test_per_core,
        app_state,
    )
}

/// Initializes the test results with the given configuration
/// The test results will be stored in the app_state
pub fn initialize_response(
    core_status: &Arc<RwLock<HashMap<usize, CpuTestStatus>>>,
    config: &CpuTestConfig,
) {
    let time_to_test_per_core = &config.duration_per_core;

    // Remove all entries
    core_status.write().unwrap().clear();

    for core_id in config.cores_to_test.clone() {
        let mut test_result = CpuTestStatus {
            core_id,
            max_clock: u64::MIN,
            avg_clock: u64::MIN,
            min_clock: u64::MAX,
            verification_failed: false,
            method_response: HashMap::new(),
        };

        for cpu_test_method in &config.test_methods {
            let total_secs_per_method = parse_duration::parse(time_to_test_per_core)
                .unwrap()
                .as_secs()
                / config.test_methods.len() as u64;
            
            let method_response = TestMethodResponse {
                method: *cpu_test_method,
                state: CpuTestMethodStatus::Idle,
                current_secs: 0,
                total_secs: total_secs_per_method,
            };

            test_result
                .method_response
                .insert(*cpu_test_method, method_response);
        }

        // Initialize the test results with empty values
        core_status.write().unwrap().insert(core_id, test_result);
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
pub fn get_cores_to_test(cores_to_test: String, physical_core_count: usize) -> Vec<usize> {
    println!("Physical core count: {}", cpu_info::get_physical_cores());
    println!("Logical core count: {}", cpu_info::get_logical_cores());

    // Parse cores to test string into a vector of usize
    let mut cores_to_test: Vec<usize> = if cores_to_test.is_empty() {
        (0..physical_core_count).collect()
    } else {
        cores_to_test
            .split(',')
            .map(|core| core.trim().parse::<usize>().unwrap())
            .collect()
    };

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
    let cores_to_test = "0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 4, 3".to_string();
    let physical_core_count = 8;
    let cores_to_test = get_cores_to_test(cores_to_test, physical_core_count);
    assert_eq!(cores_to_test, vec![0, 7, 1, 6, 2, 5, 3, 4]);
}

fn test_cores(
    cpu_test_methods: &Vec<CpuTestMethod>,
    core_ids: Vec<usize>,
    time_to_test_per_core: Duration,
    app_state: AppState,
) {
    for core_id in core_ids {
        println!("===================================");
        println!(
            "Testing core {} for {} seconds",
            core_id,
            time_to_test_per_core.as_secs()
        );
        println!("===================================");

        for cpu_test_method in cpu_test_methods {
            let time_per_method = time_to_test_per_core.div(cpu_test_methods.len() as u32);

            println!(
                " --> Testing method {:?} for {:?}",
                cpu_test_method, time_per_method
            );

            // Test the core for the given method
            test_core_with_method(cpu_test_method, core_id, time_per_method, app_state.clone());

            // If cpu test result is failed, break earls the loop,
            // we do not need to test the other methods
            if should_interrupt(app_state.clone(), core_id) {
                println!(" ### Test interrupted by user");
                break;
            }

            // Wait 5 seconds to cool down the CPU
            thread::sleep(Duration::from_secs(5));
        }

        // Test if interrupted by user, if so cancel the whole test
        if *app_state.terminated_by_user.read().unwrap() {
            println!("Test interrupted by user");
            return;
        }
    }
}

/// Checks if the test should be interrupted
/// This could be due to any test failing or the user stopping the test
fn should_interrupt(app_state: AppState, core_id: usize) -> bool {
    let user_stopped = *app_state.terminated_by_user.read().unwrap();
    let verification_failed = app_state.test_status.read().unwrap()[&core_id].verification_failed;

    verification_failed || user_stopped
}

fn set_test_method_status(
    app_state: AppState,
    core_id: usize,
    cpu_test_method: &CpuTestMethod,
    status: CpuTestMethodStatus,
) {
    let mut test_status = app_state.test_status.write().unwrap();
    let test_result = test_status.get_mut(&core_id).unwrap();
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
    app_state: AppState,
) {
    // Set the state of the method to TESTING
    set_test_method_status(
        app_state.clone(),
        core_id,
        cpu_test_method,
        CpuTestMethodStatus::Testing,
    );

    // Define the shared variables
    let pid = Arc::new(RwLock::new(0));
    let time_up = Arc::new(RwLock::new(false));
    let test_program_process = Arc::new(RwLock::new(None));

    // Thread that starts the test program process
    let pid_for_core_test = pid.clone();
    let test_program_process_for_core_test = test_program_process.clone();
    let cpu_test_method = *cpu_test_method;
    let test_program_thread = thread::Builder::new()
        .name(format!("test_program_thread_{}", core_id))
        .spawn(move || {
            start_test_program_for_core(
                cpu_test_method,
                core_id,
                pid_for_core_test,
                test_program_process_for_core_test,
            );
        })
        .unwrap();

    // Wait a bit for the test program process to start
    thread::sleep(Duration::from_secs(3));

    // Define the start and end time for the test
    let start_time = Utc::now();
    let end_time = start_time + test_time;

    // Thread that monitors the CPU usage
    let app_state_for_monitor_cpu = app_state.clone();
    let time_up_for_monitor_cpu = time_up.clone();
    let monitor_cpu_thread = thread::Builder::new()
        .name(format!("monitor_cpu_thread_{}", core_id))
        .spawn(move || monitor_cpu(core_id, time_up_for_monitor_cpu, app_state_for_monitor_cpu))
        .unwrap();

    // Thread that monitors the prime95 process output for errors
    let time_up_for_monitor_process = time_up.clone();
    let app_state_for_monitor_process = app_state.clone();
    let mprime_process_for_monitor_process = test_program_process.clone();
    let monitor_process_thread = thread::Builder::new()
        .name(format!("monitor_process_thread_{}", core_id))
        .spawn(move || {
            monitor_process(
                core_id,
                time_up_for_monitor_process,
                app_state_for_monitor_process,
                mprime_process_for_monitor_process,
            );
        })
        .unwrap();

    // Thread that checks if the time to test per core has passed
    let time_up_for_time_tester = time_up.clone();
    let app_state_for_time_tester = app_state.clone();
    let core_test_timer_thread = thread::Builder::new()
        .name(format!("core_test_timer_thread_{}", core_id))
        .spawn(move || {
            check_time_left(
                core_id,
                &cpu_test_method,
                start_time,
                end_time,
                time_up_for_time_tester,
                app_state_for_time_tester,
            );
        })
        .unwrap();

    // Wait for all threads to finish
    test_program_thread.join().unwrap();
    monitor_cpu_thread.join().unwrap();
    monitor_process_thread.join().unwrap();
    core_test_timer_thread.join().unwrap();

    // Set the state of the method to SUCCESS if the verification did not fail
    if !app_state.test_status.read().unwrap()[&core_id].verification_failed {
        set_test_method_status(
            app_state.clone(),
            core_id,
            &cpu_test_method,
            CpuTestMethodStatus::Success,
        );
    } else {
        set_test_method_status(
            app_state.clone(),
            core_id,
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
    time_up: Arc<RwLock<bool>>,
    app_state: AppState,
) {
    loop {
        // Check if the time is up
        if Utc::now() > end_time {
            println!("Time is up");
            println!();
            *time_up.write().unwrap() = true;

            // Kill the test program processes
            process::kill();

            break;
        }

        // Check if the verification failed
        if should_interrupt(app_state.clone(), physical_core_id) {
            // Kill the rest program processes
            process::kill();

            break;
        }

        // Update current_secs
        let mut core_status = app_state.test_status.write().unwrap();
        let cpu_test_status = core_status.get_mut(&physical_core_id).unwrap();
        let method_response = cpu_test_status
            .method_response
            .get_mut(cpu_test_method)
            .unwrap();
        method_response.current_secs = (Utc::now() - start_time).num_seconds() as u64;

        // Important, drop the lock before sleeping
        drop(core_status);

        // Wait a second
        thread::sleep(Duration::from_secs(1));
    }
}

fn start_test_program_for_core(
    cpu_test_method: CpuTestMethod,
    core_id: usize,
    pid: Arc<RwLock<u32>>,
    test_program_process: Arc<RwLock<Option<Child>>>,
) {
    let child = match cpu_test_method {
        CpuTestMethod::Prime95 => mprime::start_verification(core_id),
        CpuTestMethod::YCruncher => ycruncher::start_verification(core_id),
    };

    // Set the pid of the child process
    *pid.write().unwrap() = child.id();

    // Store the child process
    *test_program_process.write().unwrap() = Some(child);
}

fn monitor_cpu(physical_core_id: usize, time_up: Arc<RwLock<bool>>, app_state: AppState) {
    loop {
        // Check if time is up or if the verification failed
        let should_interrupt = should_interrupt(app_state.clone(), physical_core_id);
        if *time_up.read().unwrap() || should_interrupt {
            break;
        }

        // Collect CPU metrics
        let current_freq: u64 = cpu_info::get_cpu_freq(physical_core_id) as u64;

        // Update clocks in the test results
        let mut core_status = app_state.test_status.write().unwrap();
        let cpu_test_status = core_status.get_mut(&physical_core_id).unwrap();
        cpu_test_status.max_clock = std::cmp::max(cpu_test_status.max_clock, current_freq);
        cpu_test_status.min_clock = std::cmp::min(cpu_test_status.min_clock, current_freq);
        cpu_test_status.avg_clock = (cpu_test_status.avg_clock + current_freq) / 2;

        // Important, drop the lock before sleeping
        drop(core_status);

        // Wait a second
        thread::sleep(Duration::from_secs(1));
    }
}

fn monitor_process(
    physical_core_id: usize,
    time_up: Arc<RwLock<bool>>,
    app_state: AppState,
    mprime_process: Arc<RwLock<Option<Child>>>,
) {
    if let Some(mprime_process) = &mut *mprime_process.write().unwrap() {
        let stdout = mprime_process.stdout.as_mut().unwrap();
        let reader = std::io::BufReader::new(stdout);
        let lines = reader.lines();

        for line in lines {
            // if time is up, break
            if *time_up.read().unwrap() {
                break;
            }

            let line = line.unwrap();
            //println!("{}", line);

            if line.contains(mprime::ERROR_MESSAGE) || line.contains(ycruncher::ERROR_MESSAGE) {
                println!("#############");
                println!("Verification failed for core {}", physical_core_id);
                println!("#############");

                // Set the verification failed flag
                let mut core_status = app_state.test_status.write().unwrap();
                let test_result = core_status.get_mut(&physical_core_id).unwrap();
                test_result.verification_failed = true;

                break;
            }
        }
    }
}

pub fn stop(cpu_test_status: AppState) {
    // Set interrupted by user flag
    *cpu_test_status.terminated_by_user.write().unwrap() = true;

    // Kill all processes
    process::kill();

    // Reset all test statuses
    let mut core_status = cpu_test_status.test_status.write().unwrap();
    for (_, test_result) in core_status.iter_mut() {
        for (_, method_response) in test_result.method_response.iter_mut() {
            if method_response.state == CpuTestMethodStatus::Testing {
                method_response.state = CpuTestMethodStatus::Idle;
            }
            method_response.current_secs = 0;
        }
    }
}
