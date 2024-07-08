use crate::process;
use std::io::Write;
use std::process::{Child, Command};
use std::thread;
use std::time::Duration;

pub const ERROR_MESSAGE: &str = "TORTURE TEST FAILED";
const PROCESS_PATH: &str = "/tmp/pbo-assistant/mprime/mprime";

pub fn initialize() {
    let mprime = include_bytes!("../../assets/mprime/mprime");
    let prime_txt = include_bytes!("../../assets/mprime/prime.txt");

    std::fs::create_dir_all("/tmp/pbo-assistant/mprime").expect("Failed to create directory");

    std::fs::write(PROCESS_PATH, mprime).expect("Failed to write file");
    std::fs::write("/tmp/pbo-assistant/mprime/prime.txt", prime_txt).expect("Failed to write file");

    Command::new("chmod")
        .arg("+x")
        .arg(PROCESS_PATH)
        .output()
        .expect("Failed to change permissions");
}

fn spawn_process() -> Child {
    let mut child_process = Command::new(PROCESS_PATH)
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

pub fn start_verification(core_id: usize) -> Child {
    let child = spawn_process();

    // Wait a second to make sure the process is started
    thread::sleep(Duration::from_secs(1));

    process::set_thread_affinity(child.id(), core_id);

    child
}
