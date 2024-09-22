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
    let proc_cpuinfo_string = match fs::read_to_string("/proc/cpuinfo") {
        Ok(content) => content,
        Err(_) => return Err("Failed to read /proc/cpuinfo".to_string()),
    };

    let mut cpus: Vec<CpuInfo> = proc_cpuinfo_string
        .split("processor")
        .map(parse_cpu_info)
        .filter(|cpu_info| !cpu_info.name.is_empty())
        .collect();

    normalize_physical_core_ids(&mut cpus);

    let (physical_cores, logical_cores) = get_cores_count(&proc_cpuinfo_string);

    Ok(CpusInfo {
        cpus,
        physical_cores,
        logical_cores,
    })
}

fn normalize_physical_core_ids(cpus: &mut Vec<CpuInfo>) {
    // Normalize physical_id to a range between 0 and :physical_cores -1
    cpus.sort_by(|a, b| a.physical_id.cmp(&b.physical_id));
    let cpu_chunks = cpus.clone().into_iter().chunk_by(|cpu| cpu.physical_id);
    let cpu_chunks = cpu_chunks.into_iter().enumerate();

    for (index, (_key, chunk)) in cpu_chunks {
        let logical_cores_per_cpu: Vec<CpuInfo> = chunk.collect();

        for logical_core in logical_cores_per_cpu {
            cpus.iter_mut()
                .find(|cpu| cpu.logical_id == logical_core.logical_id)
                .unwrap()
                .physical_id = index;
        }
    }
    cpus.sort_by(|a, b| a.logical_id.cmp(&b.logical_id));
}

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

/// Parses the cpuinfo string and returns a CpuInfo struct
/// Detects logical_id, physical_id, name and mhz
fn parse_cpu_info(cpu_string: &str) -> CpuInfo {
    let cpu_lines = cpu_string.trim().lines().enumerate();

    let mut id = 0;
    let mut physical_id = 0;
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
    
    // TODO: determine logical_id
    let mut logical_id = 0;

    CpuInfo {
        id,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_cpu_info_by_physical_id() {
        let mut cpus = vec![
            CpuInfo {
                logical_id: 0,
                physical_id: 100,
                name: "Intel Core i7".to_string(),
                mhz: 3000.0,
            },
            CpuInfo {
                logical_id: 1,
                physical_id: 200,
                name: "Intel Core i7".to_string(),
                mhz: 3000.0,
            },
            CpuInfo {
                logical_id: 2,
                physical_id: 100,
                name: "Intel Core i7".to_string(),
                mhz: 3000.0,
            },
            CpuInfo {
                logical_id: 3,
                physical_id: 200,
                name: "Intel Core i7".to_string(),
                mhz: 3000.0,
            },
        ];

        normalize_physical_core_ids(&mut cpus);

        // Check if the CPU info is correct
        assert_eq!(cpus[0].physical_id, 0);
        assert_eq!(cpus[0].logical_id, 0);
        assert_eq!(cpus[1].physical_id, 1);
        assert_eq!(cpus[1].logical_id, 1);
        assert_eq!(cpus[2].physical_id, 0);
        assert_eq!(cpus[2].logical_id, 2);
        assert_eq!(cpus[3].physical_id, 1);
        assert_eq!(cpus[3].logical_id, 3);
    }

    #[test]
    fn test_chunk_cpu_info_by_physical_id_2() {
        let mut cpus = vec![
            CpuInfo {
                logical_id: 0,
                physical_id: 100,
                name: "Intel Core i7".to_string(),
                mhz: 3000.0,
            },
            CpuInfo {
                logical_id: 1,
                physical_id: 200,
                name: "Intel Core i7".to_string(),
                mhz: 3000.0,
            },
        ];

        normalize_physical_core_ids(&mut cpus);

        // Check if the CPU info is correct
        assert_eq!(cpus[0].physical_id, 0);
        assert_eq!(cpus[0].logical_id, 0);
        assert_eq!(cpus[1].physical_id, 1);
        assert_eq!(cpus[1].logical_id, 1);
    }

    #[test]
    fn test_parse_amd_cpu() {
        // Parse the CPU info
        let cpu_info = get().unwrap();

        // print the CPU info to console
        println!("{:?}", cpu_info);

        // Check if the number of physical and logical cores is correct
        assert_eq!(cpu_info.physical_cores, 12);
        assert_eq!(cpu_info.logical_cores, 24);

        // Check if the CPU info is correct
        assert_eq!(cpu_info.cpus[0].logical_id, 0);
        assert_eq!(cpu_info.cpus[0].physical_id, 0);

        // Check if the CPU info is correct
        assert_eq!(cpu_info.cpus[23].physical_id, 11);
        assert_eq!(cpu_info.cpus[23].logical_id, 23);
    }
}
