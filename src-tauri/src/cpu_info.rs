use itertools::Itertools;
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
    pub thread_count: usize,
    pub name: String,
    pub mhz: f64,
}

pub fn get_first_logical_core_id_for(physical_core_id: usize) -> usize {
    let cpu_info = get().unwrap();
    let cpu = cpu_info
        .cpus
        .iter()
        .find(|cpu| cpu.id == physical_core_id)
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

    let mut cpus: Vec<CpuInfo> = parse_cpus_info(&proc_cpuinfo_string);

    let (physical_cores, logical_cores) = get_cores_count(&proc_cpuinfo_string);

    Ok(CpusInfo {
        cpus,
        physical_cores,
        logical_cores,
    })
}

struct ProcCpuInfo{
    processor: usize,
    core_id: usize,
}

fn parse_cpus_info(proc_cpuinfo: &str) -> Vec<CpuInfo> {
    let cpu_split = proc_cpuinfo.split("\n\n").collect::<Vec<&str>>();
    let mut cpus: Vec<ProcCpuInfo> = cpu_split.iter().flat_map(parse_cpuinfo).collect();

    cpus.sort_by(|a, b| a.processor.cmp(&b.processor));

    transform_to_cpu_info(&cpus, &cpu_split)
}

fn transform_to_cpu_info(cpus: &[ProcCpuInfo], cpu_split: &[&str]) -> Vec<CpuInfo> {
    let mut physical_cores: Vec<CpuInfo> = vec![];

    for cpu in cpus {
        if !physical_cores.iter().any(|c| c.id == cpu.core_id) {
            let threads = count_threads_for_core(cpus, cpu.core_id);
            let cpu_info = find_and_parse_cpu_info(cpu_split, cpu.processor);

            physical_cores.push(CpuInfo {
                id: cpu_info.id,
                thread_count: threads,
                name: cpu_info.name,
                mhz: cpu_info.mhz,
            });
        }
    }

    physical_cores
}

fn count_threads_for_core(cpus: &[ProcCpuInfo], core_id: usize) -> usize {
    cpus.iter().filter(|c| c.core_id == core_id).count()
}

fn find_and_parse_cpu_info(cpu_split: &[&str], processor: usize) -> CpuInfo {
    let cpuinfo_str = cpu_split
        .iter()
        .find(|c| c.contains(&format!("processor\t: {}", processor)))
        .unwrap();

    parse_cpu_info(cpuinfo_str)
}

fn parse_cpuinfo(cpuinfo_str: &&str) -> Option<ProcCpuInfo> {
    let processor = get_first_proc_cpuinfo_property(cpuinfo_str, "processor")
        .parse();
    let core_id = get_first_proc_cpuinfo_property(cpuinfo_str, "core id")
        .parse();

    if processor.is_err() || core_id.is_err() {
        return None;
    }

    Some(ProcCpuInfo {
        processor: processor.unwrap(),
        core_id: core_id.unwrap(),
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
    let mut name = "".to_string();
    let mut mhz = 0.0;

    for cpu_line in cpu_lines.clone() {
        let line_index = cpu_line.0;
        let line_data = cpu_line.1;

        if line_index == 0 {
            id = line_data.split(':').last().unwrap().trim().parse().unwrap();
        } else if line_data.starts_with("model name") {
            name = line_data.split(':').last().unwrap().trim().to_string();
        } else if line_data.starts_with("cpu MHz") {
            mhz = line_data.split(':').last().unwrap().trim().parse().unwrap();
        } else if line_data.starts_with("core id") {
            physical_id = line_data.split(':').last().unwrap().trim().parse().unwrap();
        }
    }

    CpuInfo {
        id: physical_id,
        thread_count: 0,
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
    use assertor::{assert_that, EqualityAssertion};
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
    fn test_get_all_proc_cpuinfo_property_intel_ht() {
        // GIVEN
        let cpuinfo = INTEL_HYPERTHREADING;
        let property = "core id";

        // WHEN
        let result = get_all_proc_cpuinfo_property(cpuinfo, property);

        //THEN
        assert_eq!(result, vec!["0", "1", "2", "3", "0", "1", "2", "3"]);
    }

    #[test]
    fn test_parse_cpus_info_intel_ht() {
        // GIVEN
        let cpuinfo = INTEL_HYPERTHREADING;
        let property = "core id";

        // WHEN
        let result = parse_cpus_info(cpuinfo);

        // THEN
        assert_that!(result)
            .is_equal_to(vec![
                CpuInfo {
                    id: 0,
                    physical_id: 0,
                    thread_count: 4,
                    name: "Intel(R) Core(TM) i7-7700HQ CPU @ 2.80GHz".to_string(),
                    mhz: 2800.0,
                },
                CpuInfo {
                    id: 4,
                    physical_id: 1,
                    thread_count: 4,
                    name: "Intel(R) Core(TM) i7-7700HQ CPU @ 2.80GHz".to_string(),
                    mhz: 2800.0,
                },
            ]);
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
    fn test_get_all_proc_cpuinfo_property_amd_ht() {
        // GIVEN
        let cpuinfo = AMD_HYPERTHREADING;
        let property = "core id";

        // WHEN
        let result = get_all_proc_cpuinfo_property(cpuinfo, property);

        // THEN
        assert_eq!(result, vec!["0", "1", "2", "3", "4", "5", "8", "9", "10", "11", "12", "13", "0", "1", "2", "3", "4", "5", "8", "9", "10", "11", "12", "13"]);
    }
}
