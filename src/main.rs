use chrono::{DateTime, Utc};
use std::io::BufRead;
use std::process::Child;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use sysinfo::System;

mod mprime;

fn main() {
    mprime::initialize();

    // Define a list with physical cores to test, starting with 0
    let mut cores_to_test: Vec<usize> = vec![];
    let time_to_test_per_core = parse_duration::parse("10s").unwrap();

    let physical_core_count = get_physical_cores();

    // If cores_to_test is empty fill with all physical cores
    if cores_to_test.is_empty() {
        cores_to_test = (0..physical_core_count).collect::<Vec<_>>();
    }

    test_cores(cores_to_test, time_to_test_per_core);
}

fn test_cores(core_ids: Vec<usize>, time_to_test_per_core: Duration) {
    for core_id in core_ids {
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
            test_core(core_id, pid_handle, mprime_process_handle_1);
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
            );
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
        monitor_cpu_handle.join().unwrap();
        monitor_process_handle.join().unwrap();
        core_test_timer_handle.join().unwrap();

        // Check if the verification failed
        if *verification_failed.lock().unwrap() {
            println!("Verification failed for core {}", core_id);
        }
    }
}

fn check_time_left(
    pid: Arc<Mutex<u32>>,
    end_time: DateTime<Utc>,
    time_up: Arc<Mutex<bool>>,
    verification_failed: Arc<Mutex<bool>>,
) {
    // Check if the time to test per core has passed or if the verification failed
    while Utc::now() < end_time || *verification_failed.lock().unwrap() {
        thread::sleep(Duration::from_secs(1));
    }

    // Kill the prime95 process, if the time is up
    mprime::kill(*pid.lock().unwrap());

    // Set the time_up flag to true
    *time_up.lock().unwrap() = true;
}

fn test_core(core_id: usize, pid: Arc<Mutex<u32>>, mprime_process: Arc<Mutex<Option<Child>>>) {
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
) {
    let mut sys = System::new();
    loop {
        // Check if time is up or if the verification failed
        if *time_up.lock().unwrap() || *verification_failed.lock().unwrap() {
            break;
        }

        // Collect CPU metrics
        sys.refresh_cpu();
        let logical_core_id = physical_core_id * 2;
        let freq = sys.cpus()[logical_core_id].frequency();
        println!("Frequency: {:?}", freq);

        // Wait a second
        thread::sleep(Duration::from_secs(1));
    }
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
            // if time is up or verification failed, break
            if *time_up.lock().unwrap() || *verification_failed.lock().unwrap() {
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
        "Setting thread affinity for pid {} to core {}",
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
