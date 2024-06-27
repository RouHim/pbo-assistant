use flate2::read::GzDecoder;
use std::io::{Cursor, Write};
use std::process::{Child, Command};
use std::thread;
use std::time::Duration;
use tar::Archive;

pub const ERROR_MESSAGE: &str = "Errors encountered.";
const PROCESS_PATH: &str = "/tmp/pbo-assistant/ycruncher/y-cruncher";

pub fn initialize() {
    let memory_file = include_bytes!("../assets/ycruncher/ycruncher.tar.gz");

    std::fs::create_dir_all("/tmp/pbo-assistant/ycruncher").expect("Failed to create directory");

    let cursor = Cursor::new(memory_file);
    let gz = GzDecoder::new(cursor);
    let mut archive = Archive::new(gz);

    let output_dir = "/tmp/pbo-assistant/";
    archive.unpack(output_dir).expect("Failed to unpack");

    Command::new("chmod")
        .arg("+x")
        .arg(PROCESS_PATH)
        .output()
        .expect("Failed to change permissions");
}

fn spawn_process(core_id: usize) -> Child {
    let logical_core_id = core_id * 2;

    let mut child_process = Command::new(PROCESS_PATH)
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
    stdin
        .write_all(format!("{}\n", logical_core_id).as_bytes())
        .unwrap();
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
