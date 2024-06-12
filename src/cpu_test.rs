use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::process::Child;
use sysinfo::System;
use std::thread;
use std::time::Duration;
use chrono::{DateTime, Utc};
use std::io::BufRead;
use crate::mprime;

#[derive(Debug)]
pub struct CpuTestResult {
    pub id: usize,
    pub verification_failed: bool,
    pub max_clock: u64,
    pub avg_clock: u64,
    pub min_clock: u64,
}

pub fn run(duration: &str, cores_to_test: Vec<usize>) -> HashMap<usize, CpuTestResult> {
    mprime::initialize();

    // Define a list with physical cores to test, starting with 0
    let mut cores_to_test: Vec<usize> = cores_to_test;
    let time_to_test_per_core = parse_duration::parse(duration).unwrap();

    let physical_core_count = get_physical_cores();

    // If cores_to_test is empty fill with all physical cores
    if cores_to_test.is_empty() {
        cores_to_test = (0..physical_core_count).collect::<Vec<_>>();
    }

    test_cores(cores_to_test, time_to_test_per_core)
}


fn test_cores(core_ids: Vec<usize>, time_to_test_per_core: Duration) -> HashMap<usize, CpuTestResult> {
    let mut results = HashMap::new();

    for core_id in core_ids {
        let result = test_core(core_id, time_to_test_per_core);

        let verification_failed = result.0;
        let clocks = result.1;
        let max_clock = *clocks.iter().max().unwrap();
        let min_clock = *clocks.iter().min().unwrap();
        let avg_clock = clocks.iter().sum::<u64>() / clocks.len() as u64;

        results.insert(core_id, CpuTestResult {
            id: core_id,
            verification_failed,
            max_clock,
            min_clock,
            avg_clock,
        });
    }

    results
}

fn test_core(core_id: usize, time_to_test_per_core: Duration) -> (bool, Vec<u64>) {
    println!(
        "Testing core {} for {} seconds",
        core_id,
        time_to_test_per_core.as_secs()
    );

    // Define the shared variables
    let pid = Arc::new(Mutex::new(0));
    let verification_failed = Arc::new(Mutex::new(false));
    let time_up = Arc::new(Mutex::new(false));
    let mprime_process = Arc::new(Mutex::new(None));

    // Define the start and end time for the test
    let start_time = Utc::now();
    let end_time = start_time + time_to_test_per_core;

    // Thread that starts the prime95 process
    let pid_handle = pid.clone();
    let mprime_process_handle_1 = mprime_process.clone();
    let core_test_handle = thread::spawn(move || {
        start_mprime_for_core(core_id, pid_handle, mprime_process_handle_1);
    });

    // Wait a bit for the prime95 process to start
    thread::sleep(Duration::from_secs(3));

    // Thread that monitors the CPU usage
    let verification_failed_for_monitor_cpu = verification_failed.clone();
    let time_up_for_monitor_cpu = time_up.clone();
    let monitor_cpu_handle = thread::spawn(move || {
        monitor_cpu(
            core_id,
            time_up_for_monitor_cpu,
            verification_failed_for_monitor_cpu,
        )
    });

    // Thread that monitors the prime95 process output for errors
    let time_up_for_monitor_process = time_up.clone();
    let verification_failed_for_monitor_process = verification_failed.clone();
    let mprime_process_for_monitor_process = mprime_process.clone();
    let monitor_process_handle = thread::spawn(move || {
        monitor_process(
            core_id,
            time_up_for_monitor_process,
            verification_failed_for_monitor_process,
            mprime_process_for_monitor_process,
        );
    });

    // Thread that checks if the time to test per core has passed
    let time_up_for_time_tester = time_up.clone();
    let verification_failed_for_time_tester = verification_failed.clone();
    let core_test_timer_handle = thread::spawn(move || {
        check_time_left(
            pid,
            end_time,
            time_up_for_time_tester,
            verification_failed_for_time_tester,
        );
    });

    // Wait for all threads to finish
    core_test_handle.join().unwrap();
    let clocks = monitor_cpu_handle.join().unwrap();
    monitor_process_handle.join().unwrap();
    core_test_timer_handle.join().unwrap();

    // Check if the verification failed
    let verification_failed = *verification_failed.lock().unwrap();

    (verification_failed, clocks)
}

fn check_time_left(
    pid: Arc<Mutex<u32>>,
    end_time: DateTime<Utc>,
    time_up: Arc<Mutex<bool>>,
    verification_failed: Arc<Mutex<bool>>,
) {
    loop {
        // Check if the time is up
        if Utc::now() > end_time {
            println!("Time is up");
            *time_up.lock().unwrap() = true;

            // Kill the prime95 process
            let pid = *pid.lock().unwrap();
            mprime::kill(pid);

            break;
        }

        // Check if the verification failed
        if *verification_failed.lock().unwrap() {
            
            // Kill the prime95 process
            let pid = *pid.lock().unwrap();
            mprime::kill(pid);
            
            break;
        }
        
        // Wait a second
        thread::sleep(Duration::from_secs(1));
    }
}

fn start_mprime_for_core(core_id: usize, pid: Arc<Mutex<u32>>, mprime_process: Arc<Mutex<Option<Child>>>) {
    let child = start_mprime_verification(core_id);

    // Set the pid of the child process
    let mut pid_handle = pid.lock().unwrap();
    *pid_handle = child.id();

    // Store the child process in the arc mutex
    let mut mprime_process_handle = mprime_process.lock().unwrap();
    *mprime_process_handle = Some(child);
}

fn monitor_cpu(
    physical_core_id: usize,
    time_up: Arc<Mutex<bool>>,
    verification_failed: Arc<Mutex<bool>>,
) -> Vec<u64> {
    let mut clocks = vec![];
    let mut sys = System::new();
    loop {
        // Check if time is up or if the verification failed
        if *time_up.lock().unwrap() || *verification_failed.lock().unwrap() {
            break;
        }

        // Collect CPU metrics
        sys.refresh_cpu();
        let logical_core_id = physical_core_id * 2;
        let current_freq = sys.cpus()[logical_core_id].frequency();
        clocks.push(current_freq);

        // Wait a second
        thread::sleep(Duration::from_secs(1));
    }

    clocks
}

fn monitor_process(
    physical_core_id: usize,
    time_up: Arc<Mutex<bool>>,
    verification_failed: Arc<Mutex<bool>>,
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

            if line.contains("TORTURE TEST FAILED") {
                println!("#############");
                println!("Verification failed for core {}", physical_core_id);
                println!("#############");

                *verification_failed.lock().unwrap() = true;

                break;
            }
        }
    }
}

fn start_mprime_verification(core_id: usize) -> Child {
    let child = mprime::run();

    // Wait a second to make sure the process is started
    thread::sleep(Duration::from_secs(1));

    set_thread_affinity(child.id(), core_id);

    child
}

/// Set the affinity of a thread to a specific core by using the `taskset` command
fn set_thread_affinity(pid: u32, physical_core_id: usize) {
    let logical_core_id = physical_core_id * 2;
    println!(
        "Setting thread affinity for pid {} to logical core {}",
        pid, logical_core_id
    );
    std::process::Command::new("taskset")
        .arg("-a")
        .arg("-cp")
        .arg(logical_core_id.to_string())
        .arg(pid.to_string())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::inherit())
        .spawn()
        .expect("Failed to set thread affinity");
}

fn get_physical_cores() -> usize {
    let mut sys = System::new();
    sys.refresh_cpu();
    sys.physical_core_count().unwrap()
}