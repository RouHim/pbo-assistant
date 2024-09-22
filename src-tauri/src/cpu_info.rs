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
    pub id: usize,
    pub physical_id: usize,
    pub thread_count: usize,
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
    
    // TODO: find first logical core id for physical core id
    cpu.id
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
    let proc_cpuinfo_string = match fs::read_to_string("/proc/cpuinfo") {
        Ok(content) => content,
        Err(_) => return Err("Failed to read /proc/cpuinfo".to_string()),
    };

    let mut cpus: Vec<CpuInfo> = proc_cpuinfo_string
        .split("processor")
        .map(parse_cpu_info)
        .filter(|cpu_info| !cpu_info.name.is_empty())
        .collect();

    // normalize_physical_core_ids(&mut cpus);

    let (physical_cores, logical_cores) = get_cores_count(&proc_cpuinfo_string);

    Ok(CpusInfo {
        cpus,
        physical_cores,
        logical_cores,
    })
}

// fn normalize_physical_core_ids(cpus: &mut Vec<CpuInfo>) {
//     // Normalize physical_id to a range between 0 and :physical_cores -1
//     cpus.sort_by(|a, b| a.physical_id.cmp(&b.physical_id));
//     let cpu_chunks = cpus.clone().into_iter().chunk_by(|cpu| cpu.physical_id);
//     let cpu_chunks = cpu_chunks.into_iter().enumerate();
// 
//     for (index, (_key, chunk)) in cpu_chunks {
//         let logical_cores_per_cpu: Vec<CpuInfo> = chunk.collect();
// 
//         for logical_core in logical_cores_per_cpu {
//             cpus.iter_mut()
//                 .find(|cpu| cpu.logical_id == logical_core.logical_id)
//                 .unwrap()
//                 .physical_id = index;
//         }
//     }
//     cpus.sort_by(|a, b| a.logical_id.cmp(&b.logical_id));
// }

fn get_cores_count(proc_cpuinfo_string: &str) -> (usize, usize) {
    let mut lines = proc_cpuinfo_string.lines();

    let physical_cores = lines
        .find(|line| line.starts_with("cpu cores"))
        .map(|line| line.split(':').last().unwrap().trim().parse().unwrap())
        .unwrap_or(0);

    let logical_cores = lines
        .find(|line| line.starts_with("siblings"))
        .map(|line| line.split(':').last().unwrap().trim().parse().unwrap())
        .unwrap_or(0);

    (physical_cores, logical_cores)
}

fn get_first_proc_cpuinfo_property(proc_cpu_info: &str, property: &str) -> String {
    proc_cpu_info
        .lines()
        .find_map(|line| {
            if line.starts_with(property) {
                line.split(':').nth(1).map(|value| value.trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_default()
}

fn get_all_proc_cpuinfo_property(proc_cpu_info: &str, property: &str) -> Vec<String> {
    proc_cpu_info
        .lines()
        .filter_map(|line| {
            if line.starts_with(property) {
                line.split(':').nth(1).map(|value| value.trim().to_string())
            } else {
                None
            }
        })
        .collect()
}

/// Parses the cpuinfo string and returns a CpuInfo struct
/// Detects logical_id, physical_id, name and mhz
fn parse_cpu_info(cpu_string: &str) -> CpuInfo {
    let cpu_lines = cpu_string.trim().lines().enumerate();

    let mut id = 0;
    let mut physical_id = 0;
    let mut thread_count = 0;
    let mut name = "".to_string();
    let mut mhz = 0.0;

    for cpu_line in cpu_lines.clone() {
        let line_index = cpu_line.0;
        let line_data = cpu_line.1;

        if line_index == 0 {
            id = line_data.replace(':', "").trim().parse().unwrap();
        } else if line_data.starts_with("model name") {
            name = line_data.split(':').last().unwrap().trim().to_string();
        } else if line_data.starts_with("cpu MHz") {
            mhz = line_data.split(':').last().unwrap().trim().parse().unwrap();
        } else if line_data.starts_with("core id") {
            physical_id = line_data.split(':').last().unwrap().trim().parse().unwrap();
        }
    }

    // TODO Count threads
    // If needed calculate logical id dynamically based on core id and thread count per core
    thread_count = 1;

    CpuInfo {
        id,
        physical_id,
        thread_count,
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

#[cfg(test)]
mod tests {
    use super::*;

    const INTEL_HYPERTHREADING: &str = include_str!("../test_proc_cpuinfo/intel_hyperthreading");

    #[test]
    fn test_get_proc_cpuinfo_property() {
        let cpuinfo = INTEL_HYPERTHREADING;
        let property = "siblings";
        let expected = "8";
        let result = get_first_proc_cpuinfo_property(cpuinfo, property);
        assert_eq!(result, expected);
    }
    
    #[test]
    fn test_get_all_proc_cpuinfo_property() {
        let cpuinfo = INTEL_HYPERTHREADING;
        let property = "core id";
        let expected = vec!["0", "1", "2", "3", "0", "1", "2", "3"];
        let result = get_all_proc_cpuinfo_property(cpuinfo, property);
        assert_eq!(result, expected);
    }
}
