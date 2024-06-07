use std::process::Command;

pub fn run() -> u32 {
    let child_process = Command::new("openssl")
        .arg("speed")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("Failed to start the process");

    let proccess_id = child_process.id();
    
    println!("Started process with id: {}", proccess_id);
    
    proccess_id
}

pub fn kill(p0: u32) {
    Command::new("kill")
        .arg("-9")
        .arg(p0.to_string())
        .output()
        .expect("Failed to kill the process"); 
}