use crate::process;
use std::io::Write;
use std::process::{Child, Command};
use std::thread;
use std::time::Duration;

pub const ERROR_MESSAGE: &str = "TORTURE TEST FAILED";

pub fn initialize() {
    let ycruncher = include_bytes!("../ycruncher/ycruncher");

    std::fs::create_dir_all("/tmp/pbo-assistant").expect("Failed to create directory");

    std::fs::write("/tmp/pbo-assistant/ycruncher", ycruncher).expect("Failed to write file");

    Command::new("chmod")
        .arg("+x")
        .arg("/tmp/pbo-assistant/run.sh")
        .output()
        .expect("Failed to change permissions");

    Command::new("chmod")
        .arg("+x")
        .arg("/tmp/pbo-assistant/ycruncher")
        .output()
        .expect("Failed to change permissions");
}

fn spawn_process(core_id: usize) -> Child {
    let mut child_process = Command::new("/tmp/pbo-assistant/ycruncher")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to start the process");

    let mut stdin = child_process.stdin.take().unwrap();
    
    // Apply this inputs:
    // 1   Component Stress Tester
    // 1   Modify Cores
    // d   Disable all Cores
    // #   Number of logical core id
    // \n  confirm core dialog
    // 2   Modify memory settins
    // \n  confirm core dialog
    // 5   Run Forever (we are managing the time)
    // 0   Start Stress test
    
    stdin.write_all(b"1\n").unwrap();
    stdin.write_all(b"1\n").unwrap();
    stdin.write_all(b"d\n").unwrap();
    stdin.write_all(format!("{}\n", core_id).as_bytes()).unwrap();
    stdin.write_all(b"\n").unwrap();
    stdin.write_all(b"2\n").unwrap();
    stdin.write_all(b"\n").unwrap();
    stdin.write_all(b"5\n").unwrap();
    stdin.write_all(b"0\n").unwrap();

    let proccess_id = child_process.id();

    println!("Started process with id: {}", proccess_id);

    child_process
}

pub fn start_verification(core_id: usize) -> Child {
    let child = spawn_process(core_id);

    // Wait a second to make sure the process is started
    thread::sleep(Duration::from_secs(1));

    child
}
