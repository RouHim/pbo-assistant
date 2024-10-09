use itertools::Itertools;
use std::collections::HashMap;
use std::fs;

#[derive(Debug, Clone, PartialEq)]
pub struct CpusInfo {
    pub cpus: Vec<CpuInfo>,
    pub physical_cores: usize,
    pub logical_cores: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CpuInfo {
    pub id: usize,
    pub proc_cpu_id: usize,
    pub thread_count: usize,
    pub name: String,
    pub mhz: f64,
}

#[derive(Debug, PartialEq, Clone)]
struct ProcCpuInfo {
    processor: usize,
    core_id: usize,
    name: String,
    mhz: f64,
}

pub fn get_first_logical_core_id_for(physical_core_id: usize) -> usize {
    let cpu_info = get().unwrap();
    let mut logical_cpu_id = 0;

    // Find the first logical core id for the physical core id
    for cpu in cpu_info.cpus {
        if cpu.id == physical_core_id {
            break;
        }
        logical_cpu_id += cpu.thread_count;
    }

    logical_cpu_id
}

pub fn get_cpu_freq(physical_core_id: usize) -> f64 {
    let cpu_info = get().unwrap();

    for cpu in cpu_info.cpus {
        if cpu.id == physical_core_id {
            return cpu.mhz;
        }
    }

    0.0
}

pub fn get() -> Result<CpusInfo, String> {
    let proc_cpuinfo_string = match fs::read_to_string("/proc/cpuinfo") {
        Ok(content) => content,
        Err(_) => return Err("Failed to read /proc/cpuinfo".to_string()),
    };

    let cpus: Vec<CpuInfo> = parse_cpus_info(&proc_cpuinfo_string);

    let (physical_cores, logical_cores) = get_cores_count(&proc_cpuinfo_string);

    Ok(CpusInfo {
        cpus,
        physical_cores,
        logical_cores,
    })
}

fn parse_cpus_info(proc_cpuinfo: &str) -> Vec<CpuInfo> {
    let cpu_infos: Vec<ProcCpuInfo> = proc_cpuinfo
        .split("\n\n")
        .flat_map(parse_cpuinfo)
        .sorted_by(|a, b| a.processor.cmp(&b.processor))
        .collect();

    transform_to_cpu_info(cpu_infos)
}

/// Transforms the given proc_cpuinfos into an CpuInfo struct
fn transform_to_cpu_info(parsed_cpu_info: Vec<ProcCpuInfo>) -> Vec<CpuInfo> {
    let mut physical_cores: Vec<CpuInfo> = vec![];

    // Group by core.id
    let proc_cpu_info_grouped_by_core_id = group_by_core_id(parsed_cpu_info);

    for (iter_index, core_group) in proc_cpu_info_grouped_by_core_id
        .iter()
        .sorted_by(|a, b| a.0.cmp(b.0))
        .enumerate()
    {
        // Sort logical threads by processor
        let mut threads_per_core = core_group.1.to_owned();
        threads_per_core.sort_by(|a, b| a.processor.cmp(&b.processor));

        // Get first logical thread
        let first_thread = threads_per_core.first().unwrap();

        physical_cores.push(CpuInfo {
            id: iter_index,
            proc_cpu_id: first_thread.processor,
            thread_count: threads_per_core.len(),
            name: first_thread.name.clone(),
            mhz: first_thread.mhz,
        });
    }

    physical_cores
}

fn group_by_core_id(all_proc_cpu_infos: Vec<ProcCpuInfo>) -> HashMap<usize, Vec<ProcCpuInfo>> {
    let mut grouped: HashMap<usize, Vec<ProcCpuInfo>> = HashMap::new();

    for proc_cpu_info in all_proc_cpu_infos {
        let core_id = proc_cpu_info.core_id;
        if let std::collections::hash_map::Entry::Vacant(e) = grouped.entry(core_id) {
            e.insert(vec![proc_cpu_info]);
        } else {
            grouped.get_mut(&core_id).unwrap().push(proc_cpu_info);
        }
    }
    grouped
}

/// Parses the given /proc/cpuinfo string into a ProcCpuInfo struct
fn parse_cpuinfo(cpuinfo_str: &str) -> Option<ProcCpuInfo> {
    let mut processor = None;
    let mut core_id = None;
    let mut name = None;
    let mut mhz = None;

    for line in cpuinfo_str.lines() {
        if line.starts_with("processor") {
            processor = line
                .split(':')
                .nth(1)
                .and_then(|value| value.trim().parse().ok());
        } else if line.starts_with("core id") {
            core_id = line
                .split(':')
                .nth(1)
                .and_then(|value| value.trim().parse().ok());
        } else if line.starts_with("model name") {
            name = line.split(':').nth(1).map(|value| value.trim().to_string());
        } else if line.starts_with("cpu MHz") {
            mhz = line
                .split(':')
                .nth(1)
                .and_then(|value| value.trim().parse().ok());
        }
    }

    if processor.is_none() || core_id.is_none() {
        return None;
    }

    Some(ProcCpuInfo {
        processor: processor?,
        core_id: core_id?,
        name: name.unwrap_or_default(),
        mhz: mhz.unwrap_or(0.0),
    })
}

fn get_cores_count(proc_cpuinfo_string: &str) -> (usize, usize) {
    let physical_cores = get_first_proc_cpuinfo_property(proc_cpuinfo_string, "cpu cores")
        .parse()
        .unwrap_or(0);

    let logical_cores = get_first_proc_cpuinfo_property(proc_cpuinfo_string, "siblings")
        .parse()
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
    const AMD_HYPERTHREADING: &str = include_str!("../test_proc_cpuinfo/amd_hyperthreading");

    #[test]
    fn test_get_proc_cpuinfo_property_intel_ht() {
        // GIVEN
        let cpuinfo = INTEL_HYPERTHREADING;
        let property = "siblings";

        // WHEN
        let result = get_first_proc_cpuinfo_property(cpuinfo, property);

        // THEN
        assert_eq!(result, "8");
    }

    #[test]
    fn test_get_proc_cpuinfo_property_amd_ht() {
        // GIVEN
        let cpuinfo = AMD_HYPERTHREADING;
        let property = "siblings";

        // WHEN
        let result = get_first_proc_cpuinfo_property(cpuinfo, property);

        // THEN
        assert_eq!(result, "24");
    }

    #[test]
    fn group_by_core_id_single_core() {
        // GIVEN
        let proc_cpu_infos = vec![ProcCpuInfo {
            processor: 0,
            core_id: 0,
            name: "Intel Core i7".to_string(),
            mhz: 3000.0,
        }];

        // WHEN
        let result = group_by_core_id(proc_cpu_infos);

        // THEN
        assert_eq!(result.len(), 1);
        assert_eq!(result.get(&0).unwrap().len(), 1);
    }

    #[test]
    fn group_by_core_id_multiple_cores() {
        // GIVEN
        let proc_cpu_infos = vec![
            ProcCpuInfo {
                processor: 0,
                core_id: 0,
                name: "Intel Core i7".to_string(),
                mhz: 3000.0,
            },
            ProcCpuInfo {
                processor: 1,
                core_id: 1,
                name: "Intel Core i7".to_string(),
                mhz: 3000.0,
            },
            ProcCpuInfo {
                processor: 2,
                core_id: 0,
                name: "Intel Core i7".to_string(),
                mhz: 3000.0,
            },
            ProcCpuInfo {
                processor: 3,
                core_id: 1,
                name: "Intel Core i7".to_string(),
                mhz: 3000.0,
            },
        ];

        // WHEN
        let result = group_by_core_id(proc_cpu_infos);

        // THEN
        assert_eq!(result.len(), 2);
        assert_eq!(result.get(&0).unwrap().len(), 2);
        assert_eq!(result.get(&1).unwrap().len(), 2);
    }

    #[test]
    fn group_by_core_id_empty_list() {
        // GIVEN
        let proc_cpu_infos = vec![];

        // WHEN
        let result = group_by_core_id(proc_cpu_infos);

        // THEN
        assert!(result.is_empty());
    }

    #[test]
    fn group_by_core_id_single_core_multiple_processors() {
        // GIVEN
        let proc_cpu_infos = vec![
            ProcCpuInfo {
                processor: 0,
                core_id: 0,
                name: "Intel Core i7".to_string(),
                mhz: 3000.0,
            },
            ProcCpuInfo {
                processor: 1,
                core_id: 0,
                name: "Intel Core i7".to_string(),
                mhz: 3000.0,
            },
            ProcCpuInfo {
                processor: 2,
                core_id: 0,
                name: "Intel Core i7".to_string(),
                mhz: 3000.0,
            },
        ];

        // WHEN
        let result = group_by_core_id(proc_cpu_infos);

        // THEN
        assert_eq!(result.len(), 1);
        assert_eq!(result.get(&0).unwrap().len(), 3);
    }
}
