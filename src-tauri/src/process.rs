use std::process::Command;

/// Set the affinity of a thread to a specific core by using the `taskset` command
pub fn set_thread_affinity(pid: u32, physical_core_id: usize) {
    let logical_core_id = physical_core_id * 2;
    println!(
        "Setting thread affinity for pid {} to logical core {}",
        pid, logical_core_id
    );
    Command::new("taskset")
        .arg("-a")
        .arg("-cp")
        .arg(logical_core_id.to_string())
        .arg(pid.to_string())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::inherit())
        .spawn()
        .expect("Failed to set thread affinity");
}

pub fn kill() {
    let output = Command::new("pgrep")
        .arg("-f")
        .arg("/tmp/pbo-assistant/")
        .output()
        .expect("Failed to list processes");

    let pids = String::from_utf8(output.stdout).expect("Failed to convert output to string");

    for pid in pids.lines() {
        Command::new("kill")
            .arg(pid)
            .output()
            .expect("Failed to kill process");
    }
}
