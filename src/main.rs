use std::process::id;
use std::sync::{Arc, Mutex};
use std::thread;

use core_affinity::CoreId;
use rand::Rng;
use sysinfo::System;

fn main() {
    let is_hyperthreading_enabled = is_cpu_hyperthreaded();

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

fn is_cpu_hyperthreaded() -> bool {
    let mut sys = System::new();
    sys.refresh_cpu();
    let physical_cpu_count = sys.physical_core_count().unwrap();
    let physical_logical_count = sys.cpus().len();
    let is_hyperthreading_enabled = physical_cpu_count < physical_logical_count;
    is_hyperthreading_enabled
}

fn prime() {
    let p_2_25 = 33554432;
    let p_2_26 = 67108864;
    let p_2_27 = 134217728;
    let p_2_28 = 268435456;
    let p_2_29 = 536870912;

    let primes1 = sieve_of_eratosthenes(p_2_27);
    let primes2 = sieve_of_eratosthenes(p_2_27);

    // Check if results are equal
    assert_eq!(primes1, primes2);

    // Verify the primes
    println!("Verifying primes...");
    primes1.iter().for_each(|&p| {
        assert!(is_prime(p as u64, 3));
    });
    println!("All primes verified!");
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

/// Führt den Miller-Rabin-Primzahltest für eine gegebene Zahl `n` mit `k` Iterationen durch.
/// Gibt `true` zurück, wenn `n` wahrscheinlich eine Primzahl ist, andernfalls `false`.
fn is_prime(n: u64, k: u32) -> bool {
    if n <= 1 {
        return false;
    }
    if n <= 3 {
        return true;
    }
    if n % 2 == 0 || n % 3 == 0 {
        return false;
    }

    let mut rng = rand::thread_rng();
    let mut s = n - 1;
    let mut r = 0;

    while s % 2 == 0 {
        s /= 2;
        r += 1;
    }

    for _ in 0..k {
        let a = rng.gen_range(2..n);
        let mut x = modpow(a, s, n);
        if x == 1 || x == n - 1 {
            continue;
        }

        let mut i = 1;
        while i < r {
            x = modpow(x, 2, n);
            if x == n - 1 {
                break;
            }
            i += 1;
        }

        if i == r {
            return false;
        }
    }

    true
}

/// Berechnet (a^b) % n mit Hilfe der binären Exponentation.
fn modpow(a: u64, b: u64, n: u64) -> u64 {
    let mut res = 1;
    let mut x = a % n;
    let mut y = b;

    while y != 0 {
        if y % 2 == 1 {
            res = (res * x) % n;
        }
        x = (x * x) % n;
        y /= 2;
    }

    res
}