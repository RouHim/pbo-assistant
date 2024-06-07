use std::io::{stderr, Write};
use std::process::{Child, Command};
use std::sync::{Arc, Mutex};

pub fn initialize() {
    let mprime = include_bytes!("../mprime/mprime");
    let prime_txt = include_bytes!("../mprime/prime.txt");

    std::fs::create_dir_all("/tmp/pbo-assistant").expect("Failed to create directory");

    std::fs::write("/tmp/pbo-assistant/mprime", mprime).expect("Failed to write file");
    std::fs::write("/tmp/pbo-assistant/prime.txt", prime_txt).expect("Failed to write file");

    Command::new("chmod")
        .arg("+x")
        .arg("/tmp/pbo-assistant/run.sh")
        .output()
        .expect("Failed to change permissions");

    Command::new("chmod")
        .arg("+x")
        .arg("/tmp/pbo-assistant/mprime")
        .output()
        .expect("Failed to change permissions");
}

pub fn run() -> Child {
    let mut child_process = Command::new("/tmp/pbo-assistant/mprime")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to start the process");

    let mut stdin = child_process.stdin.take().unwrap();
    // Start torque test 
    stdin.write_all(b"16\n").unwrap();
    // Core count
    stdin.write_all(b"1\n").unwrap();
    // Use hyperthreading
    stdin.write_all(b"N\n").unwrap();
    // Smallest FFTs
    stdin.write_all(b"2\n").unwrap();
    stdin.write_all(b"N\n").unwrap();
    stdin.write_all(b"N\n").unwrap();
    stdin.write_all(b"Y\n").unwrap();

    let proccess_id = child_process.id();

    println!("Started process with id: {}", proccess_id);

    child_process
}

pub fn kill(p0: u32) {
    Command::new("kill")
        .arg("-9")
        .arg(p0.to_string())
        .output()
        .expect("Failed to kill the process");
}