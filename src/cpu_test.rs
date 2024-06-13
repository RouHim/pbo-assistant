use crate::mprime;
use chrono::{DateTime, Utc};
use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::io::BufRead;
use std::process::Child;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use sysinfo::System;

#[derive(Debug)]
pub struct CpuTestResult {
    pub id: usize,
    pub verification_failed: bool,
    pub max_clock: u64,
    pub avg_clock: u64,
    pub min_clock: u64,
}

#[derive(Debug)]
pub struct CpuTestConfig {
    pub duration_per_core: String,
    pub cores_to_test: Vec<usize>,
    pub test_methods: Vec<CpuTestMethod>,
}

#[derive(Debug)]
pub enum CpuTestMethod {
    Prime95,
    YCruncher,
}

pub fn run(config: CpuTestConfig) -> HashMap<usize, CpuTestResult> {
    mprime::initialize();

    let cores_to_test = get_cores_to_test(&config.cores_to_test, get_physical_cores());

    let duration = config.duration_per_core;

    let time_to_test_per_core = parse_duration::parse(&duration).unwrap();
    test_cores(cores_to_test, time_to_test_per_core)
}

fn get_cores_to_test(cores_to_test: &[usize], physical_core_count: usize) -> Vec<usize> {
    let mut cores_to_test: Vec<usize> = cores_to_test.to_vec();

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
    let cores_to_test = get_cores_to_test(&cores_to_test, physical_core_count);
    assert_eq!(cores_to_test, vec![0, 7, 1, 6, 2, 5, 3, 4]);
}

fn test_cores(
    core_ids: Vec<usize>,
    time_to_test_per_core: Duration,
) -> HashMap<usize, CpuTestResult> {
    let mut results = HashMap::new();

    for core_id in core_ids {
        let result = test_core(core_id, time_to_test_per_core);

        let verification_failed = result.0;
        let clocks = result.1;
        let max_clock = *clocks.iter().max().unwrap();
        let min_clock = *clocks.iter().min().unwrap();
        let avg_clock = clocks.iter().sum::<u64>() / clocks.len() as u64;

        results.insert(
            core_id,
            CpuTestResult {
                id: core_id,
                verification_failed,
                max_clock,
                min_clock,
                avg_clock,
            },
        );
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
    let test_program_process = Arc::new(Mutex::new(None));

    // Define the start and end time for the test
    let start_time = Utc::now();
    let end_time = start_time + time_to_test_per_core;

    // Thread that starts the test program process
    let pid_for_core_test = pid.clone();
    let test_program_process_for_core_test = test_program_process.clone();
    let test_program_thread = thread::spawn(move || {
        start_mprime_for_core(
            core_id,
            pid_for_core_test,
            test_program_process_for_core_test,
        );
    });

    // Wait a bit for the test program process to start
    thread::sleep(Duration::from_secs(3));

    // Thread that monitors the CPU usage
    let verification_failed_for_monitor_cpu = verification_failed.clone();
    let time_up_for_monitor_cpu = time_up.clone();
    let monitor_cpu_thread = thread::spawn(move || {
        monitor_cpu(
            core_id,
            time_up_for_monitor_cpu,
            verification_failed_for_monitor_cpu,
        )
    });

    // Thread that monitors the prime95 process output for errors
    let time_up_for_monitor_process = time_up.clone();
    let verification_failed_for_monitor_process = verification_failed.clone();
    let mprime_process_for_monitor_process = test_program_process.clone();
    let monitor_process_thread = thread::spawn(move || {
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
    let core_test_timer_thread = thread::spawn(move || {
        check_time_left(
            pid,
            end_time,
            time_up_for_time_tester,
            verification_failed_for_time_tester,
        );
    });

    // Wait for all threads to finish
    test_program_thread.join().unwrap();
    let clocks = monitor_cpu_thread.join().unwrap();
    monitor_process_thread.join().unwrap();
    core_test_timer_thread.join().unwrap();

    // Check if the verification failed
    let verification_failed = *verification_failed.lock().unwrap();

    (verification_failed, clocks)
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

fn start_mprime_for_core(
    core_id: usize,
    pid: Arc<Mutex<u32>>,
    mprime_process: Arc<Mutex<Option<Child>>>,
) {
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

            if line.contains(mprime::ERROR_MESSAGE) {
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
