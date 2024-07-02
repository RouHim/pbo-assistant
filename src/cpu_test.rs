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
use strum_macros::{Display, EnumIter};
use sysinfo::System;

use crate::{mprime, process, ycruncher};

#[derive(Debug, Clone)]
pub struct CpuTestResponse {
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

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, EnumIter, Display)]
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

pub fn run(config: CpuTestConfig, test_results: Arc<Mutex<HashMap<usize, CpuTestResponse>>>) {
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
        test_results,
    )
}

pub fn initialize_response(
    config: &CpuTestConfig,
    test_results: &Arc<Mutex<HashMap<usize, CpuTestResponse>>>,
    time_to_test_per_core: &str,
) {
    for core_id in &config.cores_to_test {
        let mut test_result = CpuTestResponse {
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

        test_results.lock().unwrap().insert(*core_id, test_result);
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

pub fn get_cores_to_test(cores_to_test: &str, physical_core_count: usize) -> Vec<usize> {
    let mut cores_to_test: Vec<usize> = cpu_core_string_to_vec(cores_to_test);

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

fn cpu_core_string_to_vec(cores_to_test: &str) -> Vec<usize> {
    // if string is empty or contains only whitespace, return an empty vector
    if cores_to_test.trim().is_empty() {
        return vec![];
    }

    cores_to_test
        .split(',')
        .map(|s| s.trim().parse::<usize>().unwrap())
        .collect()
}

// test for get_cores_to_test
#[test]
fn test_get_cores_to_test() {
    let cores_to_test = "0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 4, 3";
    let physical_core_count = 8;
    let cores_to_test = get_cores_to_test(cores_to_test, physical_core_count);
    assert_eq!(cores_to_test, vec![0, 7, 1, 6, 2, 5, 3, 4]);
}

fn test_cores(
    cpu_test_methods: Vec<CpuTestMethod>,
    core_ids: &Vec<usize>,
    time_to_test_per_core: Duration,
    test_results: Arc<Mutex<HashMap<usize, CpuTestResponse>>>,
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
            test_core_with_method(
                cpu_test_method,
                core_id,
                time_per_method,
                test_results.clone(),
            );

            // If cpu test result is failed, break earls the loop,
            // we do not need to test the other methods
            if test_results.lock().unwrap()[&core_id].verification_failed {
                break;
            }

            // Wait 5 seconds to cool down the CPU
            thread::sleep(Duration::from_secs(5));
        }
    }
}

fn set_test_method_status(
    test_results: Arc<Mutex<HashMap<usize, CpuTestResponse>>>,
    core_id: &usize,
    cpu_test_method: &CpuTestMethod,
    status: CpuTestMethodStatus,
) {
    let mut test_results = test_results.lock().unwrap();
    let test_result = test_results.get_mut(core_id).unwrap();
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
    test_results: Arc<Mutex<HashMap<usize, CpuTestResponse>>>,
) {
    // Set the state of the method to TESTING
    set_test_method_status(
        test_results.clone(),
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
    let test_results_for_monitor_cpu = test_results.clone();
    let time_up_for_monitor_cpu = time_up.clone();
    let monitor_cpu_thread = thread::spawn(move || {
        monitor_cpu(
            core_id,
            time_up_for_monitor_cpu,
            test_results_for_monitor_cpu,
        )
    });

    // Thread that monitors the prime95 process output for errors
    let time_up_for_monitor_process = time_up.clone();
    let test_results_for_monitor_process = test_results.clone();
    let mprime_process_for_monitor_process = test_program_process.clone();
    let monitor_process_thread = thread::spawn(move || {
        monitor_process(
            core_id,
            time_up_for_monitor_process,
            test_results_for_monitor_process,
            mprime_process_for_monitor_process,
        );
    });

    // Thread that checks if the time to test per core has passed
    let time_up_for_time_tester = time_up.clone();
    let test_results_for_time_tester = test_results.clone();
    let core_test_timer_thread = thread::spawn(move || {
        check_time_left(
            core_id,
            &cpu_test_method,
            start_time,
            end_time,
            time_up_for_time_tester,
            test_results_for_time_tester,
        );
    });

    // Wait for all threads to finish
    test_program_thread.join().unwrap();
    monitor_cpu_thread.join().unwrap();
    monitor_process_thread.join().unwrap();
    core_test_timer_thread.join().unwrap();

    // Set the state of the method to SUCCESS if the verification did not fail
    if !test_results.lock().unwrap()[&core_id].verification_failed {
        set_test_method_status(
            test_results.clone(),
            &core_id,
            &cpu_test_method,
            CpuTestMethodStatus::Success,
        );
    } else {
        set_test_method_status(
            test_results.clone(),
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
    test_results: Arc<Mutex<HashMap<usize, CpuTestResponse>>>,
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
        if test_results.lock().unwrap()[&physical_core_id].verification_failed {
            // Kill the rest program processes
            process::kill();

            break;
        }

        // Update current_secs
        let mut test_results = test_results.lock().unwrap();
        let test_result = test_results.get_mut(&physical_core_id).unwrap();
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
    test_results: Arc<Mutex<HashMap<usize, CpuTestResponse>>>,
) {
    let mut sys = System::new();
    loop {
        // Check if time is up or if the verification failed
        let verification_failed =
            test_results.lock().unwrap()[&physical_core_id].verification_failed;
        if *time_up.lock().unwrap() || verification_failed {
            break;
        }

        // Collect CPU metrics
        sys.refresh_cpu();
        let logical_core_id = physical_core_id * 2;
        let current_freq = sys.cpus()[logical_core_id].frequency();

        // Update clocks in the test results
        let mut test_results = test_results.lock().unwrap();
        let test_result = test_results.get_mut(&physical_core_id).unwrap();
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
    test_results: Arc<Mutex<HashMap<usize, CpuTestResponse>>>,
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
                let mut test_results = test_results.lock().unwrap();
                let test_result = test_results.get_mut(&physical_core_id).unwrap();
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
