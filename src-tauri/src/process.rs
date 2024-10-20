use crate::cpu_info;
use std::collections::HashSet;
use std::io::BufRead;
use std::process::Command;

/// Set the affinity of a thread to a specific core by using the `taskset` command
pub fn set_thread_affinity(pid: u32, physical_core_id: usize) {
    let logical_core_id = cpu_info::get_first_logical_core_id_for(physical_core_id);
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

pub fn kill_pbo_app() {
    let mut all_pids: HashSet<u32> = HashSet::new();

    let pids_of_pbo_assistant = get_all_pids_for_command_path("/tmp/pbo-assistant/");
    all_pids.extend(pids_of_pbo_assistant.clone());

    for pid in pids_of_pbo_assistant {
        all_pids.extend(get_all_child_pids(pid));
    }

    for pid in all_pids {
        let pid = nix::unistd::Pid::from_raw(pid as i32);
        nix::sys::signal::kill(pid, nix::sys::signal::Signal::SIGKILL).unwrap();
    }
}

/// Get all PIDs for the specified command path
fn get_all_pids_for_command_path(process_path: &str) -> Vec<u32> {
    let mut pids = Vec::new();

    for processes in procfs::process::all_processes().unwrap() {
        let process = processes.unwrap();
        if let Ok(status) = process.stat() {
            if let Ok(exe) = process.exe() {
                let exec_path = exe.to_str().unwrap();
                if exec_path.starts_with(process_path) {
                    pids.push(status.pid as u32);
                }
            }
        }
    }

    pids
}

/// Get all child processes of the specified parent process id
fn get_all_child_pids(parent_pid: u32) -> Vec<u32> {
    let mut child_pids = Vec::new();

    for processes in procfs::process::all_processes().unwrap() {
        let process = processes.unwrap();
        if let Ok(status) = process.stat() {
            if status.ppid as u32 == parent_pid {
                child_pids.push(status.pid as u32);
            }
        }
    }

    child_pids
}

pub fn pause(test_app_pid: u32) {
    let mut all_pids: HashSet<u32> = HashSet::new();
    all_pids.extend(get_all_child_pids(test_app_pid));
    all_pids.insert(test_app_pid);

    for pid in all_pids {
        let pid = nix::unistd::Pid::from_raw(pid as i32);
        nix::sys::signal::kill(pid, nix::sys::signal::Signal::SIGSTOP).unwrap();
    }
}

pub fn resume(test_app_pid: u32) {
    let mut all_pids: HashSet<u32> = HashSet::new();
    all_pids.extend(get_all_child_pids(test_app_pid));
    all_pids.insert(test_app_pid);

    for pid in all_pids {
        let pid = nix::unistd::Pid::from_raw(pid as i32);
        nix::sys::signal::kill(pid, nix::sys::signal::Signal::SIGCONT).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assertor::{assert_that, VecAssertion};
    use std::thread::sleep;
    use std::time::Duration;

    #[test]
    fn test_get_all_pids_for_command_path() {
        // Start a simple process (e.g., sleep for 10 seconds)
        let mut cmd = Command::new("sleep")
            .arg("10")
            .spawn()
            .expect("Failed to start sleep process");

        // Give the process some time to start
        sleep(Duration::from_secs(1));

        // Get the command path of the sleep process
        let process_path = "/usr/bin/sleep";

        // Call the function to get all PIDs for the command path
        let pids = get_all_pids_for_command_path(process_path);

        assert_that!(pids).is_not_empty();

        // Kill the process
        cmd.kill().expect("Failed to kill sleep process");
    }
}
