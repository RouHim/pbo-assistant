use std::process::id;
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::current;
use std::time::Instant;
use core_affinity::CoreId;
use sysinfo::System;

fn main() {

    let mut sys = System::new();
    sys.refresh_cpu();
    let physical_cpu_count = sys.physical_core_count().unwrap();
    let physical_logical_count = sys.cpus().len();
    let is_hyperthreading_enabled = physical_cpu_count < physical_logical_count;

    // Retrieve the IDs of all cores on which the current
    // thread is allowed to run.
    // NOTE: If you want ALL the possible cores, you should
    // use num_cpus.
    let core_ids = core_affinity::get_core_ids().unwrap();

    // if hyperthreading is enabled, only use every second core
    let core_ids: Vec<CoreId> = if is_hyperthreading_enabled {
        core_ids.iter().step_by(2).copied().collect()
    } else {
        core_ids
    };

    // Now cycle through the cores and pin the current thread to each core
    for core_id in core_ids {
        let prime_done = Arc::new(Mutex::new(false));

        let prime_done_1 = prime_done.clone();
        let prime_handle = thread::spawn(move || {
            // Pin this thread to a single CPU core.
            let res = core_affinity::set_for_current(core_id);
            if res {
                println!("Thread {} pinned to core {}", id(), core_id.id);
                prime();
                *prime_done_1.lock().unwrap() = true;
            }
        });

        let prime_done_2 = prime_done.clone();
        let monitor_handle = thread::spawn(move || {
            let mut sys = System::new();
            loop {
                sys.refresh_cpu();
                let current_id = core_id.id;
                let freq = sys.cpus()[current_id].frequency();

                println!("Frequency: {:?}", freq);

                // Wait a second
                thread::sleep(std::time::Duration::from_secs(1));

                if *prime_done_2.lock().unwrap() {
                    break;
                }
            }
        });

        prime_handle.join().unwrap();
        monitor_handle.join().unwrap();
    }
}
fn prime() {
    sieve_of_eratosthenes(100000000);
}

fn sieve_of_eratosthenes(limit: usize) -> Vec<usize> {
    let mut primes = vec![true; limit + 1];
    primes[0] = false;
    primes[1] = false;

    let mut p = 2;
    while p * p <= limit {
        if primes[p] {
            for i in (p * p..=limit).step_by(p) {
                primes[i] = false;
            }
        }
        p += 1;
    }
    primes.iter().enumerate()
        .filter(|&(_i, &prime)| prime)
        .map(|(i, _)| i)
        .collect::<Vec<_>>()
}