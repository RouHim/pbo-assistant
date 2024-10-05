use std::fs;
use itertools::Itertools;

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

    transform_to_cpu_info(cpu_infos, proc_cpuinfo)
}


/// Transforms the given proc_cpuinfos into an CpuInfo struct
fn transform_to_cpu_info(parsed_cpu_info: Vec<ProcCpuInfo>, proc_cpuinfo: &str) -> Vec<CpuInfo> {
    let mut physical_cores: Vec<CpuInfo> = vec![];

    // Group by core.id
    let cpu_info_by_core_id = parsed_cpu_info.chunk_by(|a, b| a.core_id == b.core_id);

    for (iter_index, core_chunk) in cpu_info_by_core_id.enumerate() {
        // Sort logical threads by processor
        // FIXME: this is broken
        let mut core_chunk = core_chunk.to_vec();
        core_chunk.sort_by(|a, b| a.processor.cmp(&b.processor));

        // Get first logical thread
        let first_logical_thread = core_chunk.first().unwrap();

        // Compute properties
        let thread_count = core_chunk.len();
        let name = get_proc_cpuinfo_property_by_processor(
            proc_cpuinfo,
            first_logical_thread.processor,
            "model name",
        );
        let mhz = get_proc_cpuinfo_property_by_processor(
            proc_cpuinfo,
            first_logical_thread.processor,
            "cpu MHz",
        )
        .parse()
        .unwrap_or(0.0);
        
        // print ids and mhs for each core
        println!("Core: {} ({}), Mhz: {}", iter_index, first_logical_thread.processor, mhz);

        physical_cores.push(CpuInfo {
            id: iter_index,
            proc_cpu_id: first_logical_thread.processor,
            thread_count,
            name,
            mhz,
        })
    }

    physical_cores
}

/// Finds the specified property value for the given processor id
fn get_proc_cpuinfo_property_by_processor(
    proc_cpu_info: &str,
    processor_id: usize,
    prperty_name: &str,
) -> String {
    let cpuinfo = proc_cpu_info.lines().collect::<Vec<&str>>();
    let mut property_value = String::new();

    for (index, line) in cpuinfo.iter().enumerate() {
        if line.starts_with("processor") && line.ends_with(&processor_id.to_string()) {
            for next_line in &cpuinfo[index..] {
                if next_line.starts_with(prperty_name) {
                    property_value = next_line.split(":").nth(1).unwrap().trim().to_string();
                    break;
                }
            }
        }
    }

    property_value
}

/// Parses the given /proc/cpuinfo string into a ProcCpuInfo struct
fn parse_cpuinfo(cpuinfo_str: &str) -> Option<ProcCpuInfo> {
    let processor = get_first_proc_cpuinfo_property(cpuinfo_str, "processor").parse();
    let core_id = get_first_proc_cpuinfo_property(cpuinfo_str, "core id").parse();

    if processor.is_err() || core_id.is_err() {
        return None;
    }

    Some(ProcCpuInfo {
        processor: processor.unwrap(),
        core_id: core_id.unwrap(),
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
    use assertor::{assert_that, EqualityAssertion};

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
    fn test_parse_cpus_info_intel_ht() {
        // GIVEN
        let cpuinfo = INTEL_HYPERTHREADING;

        // WHEN
        let result = parse_cpus_info(cpuinfo);

        // THEN
        assert_that!(result).is_equal_to(vec![
            CpuInfo {
                id: 0,
                proc_cpu_id: 0,
                thread_count: 2,
                name: "Intel(R) Core(TM) i7-6820HQ CPU @ 2.70GHz".to_string(),
                mhz: 800.071,
            },
            CpuInfo {
                id: 1,
                proc_cpu_id: 1,
                thread_count: 2,
                name: "Intel(R) Core(TM) i7-6820HQ CPU @ 2.70GHz".to_string(),
                mhz: 799.998,
            },
            CpuInfo {
                id: 2,
                proc_cpu_id: 2,
                thread_count: 2,
                name: "Intel(R) Core(TM) i7-6820HQ CPU @ 2.70GHz".to_string(),
                mhz: 800.000,
            },
            CpuInfo {
                id: 3,
                proc_cpu_id: 3,
                thread_count: 2,
                name: "Intel(R) Core(TM) i7-6820HQ CPU @ 2.70GHz".to_string(),
                mhz: 800.000,
            },
        ]);
    }

    #[test]
    fn test_parse_cpus_info_amd_ht() {
        // GIVEN
        let cpuinfo = AMD_HYPERTHREADING;

        // WHEN
        let result = parse_cpus_info(cpuinfo);

        // THEN
        assert_that!(result).is_equal_to(vec![
            CpuInfo {
                id: 0,
                proc_cpu_id: 0,
                thread_count: 2,
                name: "AMD Ryzen 9 5900X 12-Core Processor".to_string(),
                mhz: 2200.0,
            },
            CpuInfo {
                id: 1,
                proc_cpu_id: 1,
                thread_count: 2,
                name: "AMD Ryzen 9 5900X 12-Core Processor".to_string(),
                mhz: 2200.034,
            },
            CpuInfo {
                id: 2,
                proc_cpu_id: 2,
                thread_count: 2,
                name: "AMD Ryzen 9 5900X 12-Core Processor".to_string(),
                mhz: 2200.0,
            },
            CpuInfo {
                id: 3,
                proc_cpu_id: 3,
                thread_count: 2,
                name: "AMD Ryzen 9 5900X 12-Core Processor".to_string(),
                mhz: 2200.0,
            },
            CpuInfo {
                id: 4,
                thread_count: 2,
                name: "AMD Ryzen 9 5900X 12-Core Processor".to_string(),
                mhz: 2200.0,
            },
            CpuInfo {
                id: 5,
                thread_count: 2,
                name: "AMD Ryzen 9 5900X 12-Core Processor".to_string(),
                mhz: 2200.0,
            },
            CpuInfo {
                id: 6,
                thread_count: 2,
                name: "AMD Ryzen 9 5900X 12-Core Processor".to_string(),
                mhz: 3597.451,
            },
            CpuInfo {
                id: 7,
                thread_count: 2,
                name: "AMD Ryzen 9 5900X 12-Core Processor".to_string(),
                mhz: 3339.015,
            },
            CpuInfo {
                id: 8,
                thread_count: 2,
                name: "AMD Ryzen 9 5900X 12-Core Processor".to_string(),
                mhz: 2200.0,
            },
            CpuInfo {
                id: 9,
                thread_count: 2,
                name: "AMD Ryzen 9 5900X 12-Core Processor".to_string(),
                mhz: 2200.0,
            },
            CpuInfo {
                id: 10,
                thread_count: 2,
                name: "AMD Ryzen 9 5900X 12-Core Processor".to_string(),
                mhz: 2200.0,
            },
            CpuInfo {
                id: 11,
                thread_count: 2,
                name: "AMD Ryzen 9 5900X 12-Core Processor".to_string(),
                mhz: 2200.0,
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
}
