mod mprime;

use std::io::BufRead;
use std::process::Child;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use sysinfo::{Pid, System};

fn main() {
    mprime::initialize();

    // Define a list with physical cores to test, starting with 0
    let mut cores_to_test: Vec<usize> = vec![];
    let time_to_test_per_core = parse_duration::parse("10m").unwrap();

    let physical_core_count = get_physical_cores();

    // If cores_to_test is empty fill with all physical cores
    if cores_to_test.is_empty() {
        cores_to_test = (0..physical_core_count).collect::<Vec<_>>();
    }

    test_cores(cores_to_test, time_to_test_per_core);
}

fn test_cores(core_ids: Vec<usize>, time_to_test_per_core: Duration) {
    for core_id in core_ids {
        println!("Testing core {} for {} seconds", core_id, time_to_test_per_core.as_secs());

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
                thread::sleep(Duration::from_secs(1));
            }

            // Kill the prime95 process
            mprime::kill(*pid.lock().unwrap());

            // Set the time_up flag to true
            *time_up_handle2.lock().unwrap() = true;
        });


        // TODO: This hangs on verification failed
        core_test_monitor_handle.join().unwrap();
        core_test_handle.join().unwrap();

        // Check if the verification failed
        // TODO: Handle this
        // TODO: We are stuck after this message gets printed
        if *verification_failed.lock().unwrap() {
            println!("Verification failed for core {}", core_id);
        }
    }
}

fn test_core(core_id: usize, pid_handle: Arc<Mutex<u32>>, verification_failed: Arc<Mutex<bool>>, time_up_handle: Arc<Mutex<bool>>) {
    // arc mutex for mutable child
    let mprime_process_handle = Arc::new(Mutex::new(None));

    let mprime_process_handle2 = mprime_process_handle.clone();
    let prime_handle = thread::spawn(move || {
        let child = start_mprime_verification(core_id);

        // Set the pid of the child process
        let mut pid_handle = pid_handle.lock().unwrap();
        *pid_handle = child.id();

        // Store the child process in the arc mutex
        let mut mprime_process_handle = mprime_process_handle2.lock().unwrap();
        *mprime_process_handle = Some(child);
    });

    let verification_failed_handle = verification_failed.clone();
    let time_up_handle_2 = time_up_handle.clone();
    let monitor_handle = thread::spawn(move || {
        monitor(core_id, time_up_handle_2, verification_failed_handle, mprime_process_handle);
    });

    prime_handle.join().unwrap();
    monitor_handle.join().unwrap();
}

fn monitor(physical_core_id: usize, time_up: Arc<Mutex<bool>>, verification_failed: Arc<Mutex<bool>>, mprime_process_handle: Arc<Mutex<Option<Child>>>) {
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

        // Check mprime press output for "TORTURE TEST FAILED"
        let mut mprime_process = mprime_process_handle.lock().unwrap();
        if let Some(child) = &mut *mprime_process {
            let stdout = child.stdout.as_mut().take().unwrap();
            let reader = std::io::BufReader::new(stdout);
            // TODO: this blocks until there is new output thus we are stuck here
            // Find a way to read all output in t he buffer and continue
            for line in reader.lines() {
                let line = line.unwrap();
                println!("{}", line);
                if line.contains("TORTURE TEST FAILED") {
                    println!("#############");
                    println!("Verification failed for core {}", physical_core_id);
                    println!("#############");
                    *verification_failed.lock().unwrap() = true;
                    break;
                }
            }
        }

        // Wait a second
        thread::sleep(std::time::Duration::from_secs(1));
    }
}

fn start_mprime_verification(core_id: usize) -> Child {
    let child = mprime::run();

    // Wait a second to make sure the process is started
    thread::sleep(std::time::Duration::from_secs(1));

    set_thread_affinity(child.id(), core_id);

    child
}

/// Set the affinity of a thread to a specific core by using the `taskset` command
fn set_thread_affinity(pid: u32, physical_core_id: usize) {
    let logical_core_id = physical_core_id * 2;
    println!("Setting thread affinity for pid {} to core {}", pid, logical_core_id);
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