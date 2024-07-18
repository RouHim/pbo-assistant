use itertools::Itertools;
use std::fs;

#[derive(Debug)]
pub struct CpusInfo {
    pub cpus: Vec<CpuInfo>,
    pub physical_cores: usize,
    pub logical_cores: usize,
}

#[derive(Debug, Clone)]
pub struct CpuInfo {
    pub logical_id: usize,
    pub physical_id: usize,
    pub name: String,
    pub mhz: f64,
}

pub fn get_first_logical_core_id_for(physical_core_id: usize) -> usize {
    let cpu_info = get().unwrap();
    let cpu = cpu_info
        .cpus
        .iter()
        .find(|cpu| cpu.physical_id == physical_core_id)
        .unwrap();
    cpu.logical_id
}

pub fn get_cpu_freq(physical_core_id: usize) -> f64 {
    let cpu_info = get().unwrap();
    let cpu = cpu_info
        .cpus
        .iter()
        .find(|cpu| cpu.physical_id == physical_core_id)
        .unwrap();
    cpu.mhz
}

pub fn get() -> Result<CpusInfo, String> {
    let content = match fs::read_to_string("/proc/cpuinfo") {
        Ok(content) => content,
        Err(_) => return Err("Failed to read /proc/cpuinfo".to_string()),
    };

    let mut cpus: Vec<CpuInfo> = content
        .split("processor")
        .map(parse_cpu_info)
        .filter(|cpu_info| !cpu_info.name.is_empty())
        .collect();

    let physical_cores = cpus.iter().chunk_by(|x| x.physical_id).into_iter().count();
    let logical_cores = cpus.len();

    // Normalize physical_id to a range between 0 and :physical_cores
    cpus.sort_by(|a, b| a.logical_id.cmp(&b.logical_id));
    cpus.iter_mut()
        .chunk_by(|x| x.physical_id)
        .into_iter()
        .enumerate()
        .for_each(|(group_enumeration, (_, cpus))| {
            for cpu in cpus {
                cpu.physical_id = group_enumeration;
            }
        });

    Ok(CpusInfo {
        cpus,
        physical_cores,
        logical_cores,
    })
}

fn parse_cpu_info(cpu_string: &str) -> CpuInfo {
    let cpu_lines = cpu_string.trim().lines().enumerate();

    let mut logical_id = 0;
    let mut physical_id = 0;
    let mut name = "".to_string();
    let mut mhz = 0.0;

    for cpu_line in cpu_lines.clone() {
        let line_index = cpu_line.0;
        let line_data = cpu_line.1;

        if line_index == 0 {
            logical_id = line_data.replace(':', "").trim().parse().unwrap();
        } else if line_data.starts_with("model name") {
            name = line_data.split(':').last().unwrap().trim().to_string();
        } else if line_data.starts_with("cpu MHz") {
            mhz = line_data.split(':').last().unwrap().trim().parse().unwrap();
        } else if line_data.starts_with("core id") {
            physical_id = line_data.split(':').last().unwrap().trim().parse().unwrap();
        }
    }

    CpuInfo {
        logical_id,
        physical_id,
        name,
        mhz,
    }
}

pub fn get_physical_cores() -> usize {
    get().unwrap().physical_cores
}

pub fn get_logical_cores() -> usize {
    get().unwrap().logical_cores
}
