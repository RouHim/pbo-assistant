use crate::process;
use std::process::{Child, Command};
use std::thread;
use std::time::Duration;

pub const ERROR_MESSAGE: &str = "TORTURE TEST FAILED";
const PROCESS_PATH: &str = "/tmp/pbo-assistant/mprime/mprime";
const CONFIG_PATH: &str = "/tmp/pbo-assistant/mprime/prime.txt";

pub fn initialize() {
    // Kill all processes
    process::kill_pbo_app();

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

/// Start the process only with mprime -t prime.txt  
fn spawn_process() -> Child {
    let child_process = Command::new(PROCESS_PATH)
        .arg("-t")
        .arg(CONFIG_PATH)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to start the process");

    let process_id = child_process.id();

    println!("Started process with id: {}", process_id);

    child_process
}

pub fn start_verification(core_id: usize) -> Child {
    let child = spawn_process();

    // Wait a second to make sure the process is started
    thread::sleep(Duration::from_secs(1));

    process::set_thread_affinity(child.id(), core_id);

    child
}
