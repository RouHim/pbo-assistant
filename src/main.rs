mod prime95;

use std::sync::{Arc, Mutex};
use std::thread;
use chrono::{Duration, TimeDelta};

use sysinfo::{Pid, System};

fn main() {
    // Define a list with physical cores to test, starting with 0
    let mut cores_to_test: Vec<usize> = vec![];
    let time_to_test_per_core = Duration::seconds(10);

    let physical_core_count = get_physical_cores();

    // If cores_to_test is empty fill with all physical cores
    if cores_to_test.is_empty() {
        cores_to_test = (0..physical_core_count).collect::<Vec<_>>();
    }

    test_cores(cores_to_test, time_to_test_per_core);
}

fn test_cores(core_ids: Vec<usize>, time_to_test_per_core: TimeDelta) {
    for core_id in core_ids {
        println!("Testing core {} for {} seconds", core_id, time_to_test_per_core.num_seconds());

        let pid = Arc::new(Mutex::new(0));
        let verification_failed = Arc::new(Mutex::new(false));
        let time_up = Arc::new(Mutex::new(false));

        let start_time = chrono::Utc::now();
        let end_time = start_time + time_to_test_per_core;

        let pid_handle = pid.clone();
        let verification_failed_handle = verification_failed.clone();
        let time_up_handle = time_up.clone();
        let core_test_handle = thread::spawn(move || {
            test_core(core_id, pid_handle, verification_failed_handle, time_up_handle);
        });

        let time_up_handle2 = time_up.clone();
        let verification_failed_handle = verification_failed.clone();
        let core_test_monitor_handle = thread::spawn(move || {
            // Check if the time to test per core has passed or if the verification failed

            while chrono::Utc::now() < end_time || *verification_failed_handle.lock().unwrap() {
                println!("Time left: {:?}", end_time - chrono::Utc::now());
                thread::sleep(std::time::Duration::from_secs(1));
            }

            // Kill the prime95 process
            prime95::kill(*pid.lock().unwrap());

            // Set the time_up flag to true
            *time_up_handle2.lock().unwrap() = true;
        });


        core_test_monitor_handle.join().unwrap();
        core_test_handle.join().unwrap();

        // Check if the verification failed
        // TODO: Handle this
        if *verification_failed.lock().unwrap() {
            println!("Verification failed for core {}", core_id);
        }
    }
}

fn test_core(core_id: usize, pid_handle: Arc<Mutex<u32>>, verification_failed: Arc<Mutex<bool>>, time_up_handle: Arc<Mutex<bool>>) {
    let prime_handle = thread::spawn(move || {
        let pid = prime_verification(core_id);
        let mut pid_handle = pid_handle.lock().unwrap();
        *pid_handle = pid;
    });

    let verification_failed_handle = verification_failed.clone();
    let monitor_handle = thread::spawn(move || {
        monitor_cpu(core_id, time_up_handle, verification_failed_handle);
    });

    prime_handle.join().unwrap();
    monitor_handle.join().unwrap();
}

fn monitor_cpu(physical_core_id: usize, time_up: Arc<Mutex<bool>>, verification_failed: Arc<Mutex<bool>>) {
    let mut sys = System::new();
    loop {
        // Check if time is up or if the verification failed
        if *time_up.lock().unwrap() || *verification_failed.lock().unwrap() {
            break;
        }

        sys.refresh_cpu();
        let logical_core_id = physical_core_id * 2;
        let freq = sys.cpus()[logical_core_id].frequency();

        println!("Frequency: {:?}", freq);

        // Wait a second
        thread::sleep(std::time::Duration::from_secs(1));
    }
}

fn prime_verification(core_id: usize) -> u32 {
    let pid = prime95::run();
    set_thread_affinity(pid, core_id);
    pid
}

/// Set the affinity of a thread to a specific core by using the `taskset` command
fn set_thread_affinity(pid: u32, physical_core_id: usize) {
    let logical_core_id = physical_core_id * 2;
    let output = std::process::Command::new("taskset")
        .arg("-a")
        .arg("-cp")
        .arg(logical_core_id.to_string())
        .arg(pid.to_string())
        .output()
        .expect("Failed to set thread affinity");

    println!("Set thread affinity: {:?}", output);
}

fn get_physical_cores() -> usize {
    let mut sys = System::new();
    sys.refresh_cpu();
    sys.physical_core_count().unwrap()
}