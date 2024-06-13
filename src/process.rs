use std::process::Command;

/// Set the affinity of a thread to a specific core by using the `taskset` command
pub fn set_thread_affinity(pid: u32, physical_core_id: usize) {
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

pub fn kill(pid: u32) {
    Command::new("kill")
        .arg("-9")
        .arg(pid.to_string())
        .output()
        .expect("Failed to kill the process");
}
