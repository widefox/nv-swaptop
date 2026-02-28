use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct ProcessSwapInfo {
    pub pid: u32,
    pub name: String,
    pub swap_size: f64,
    #[cfg(target_os = "linux")]
    pub last_cpu: Option<i32>,
}

#[cfg(target_os = "linux")]
#[derive(Debug, Clone)]
pub struct InfoSwap {
    pub name: String,
    pub kind: String,
    pub size_kb: f64,
    pub used_kb: f64,
    pub priority: isize,
}

#[derive(Debug, Clone, Default)]
pub struct SwapUpdate {
    #[cfg(target_os = "linux")]
    pub swap_devices: Vec<InfoSwap>,
    pub total_swap: u64,
    pub used_swap: u64,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub enum SizeUnits {
    #[default]
    KB,
    MB,
    GB,
}

#[cfg(target_os = "linux")]
#[derive(Debug, Error)]
pub enum SwapDataError {
    #[error("Procfs error: {0}")]
    Procfs(#[from] procfs::ProcError),
    #[error("I/O error accessing /proc: {0}")]
    Io(#[from] std::io::Error),
}

#[cfg(target_os = "windows")]
#[derive(Debug, Error)]
pub enum SwapDataError {
    #[error("I/O error accessing system information: {0}")]
    Io(#[from] std::io::Error),
}

// --- NUMA types (Linux only) ---

#[cfg(target_os = "linux")]
#[derive(Debug, Clone, PartialEq)]
pub enum NumaNodeType {
    Cpu,
    GpuHbm { gpu_index: u32 },
    Unknown,
}

#[cfg(target_os = "linux")]
#[derive(Debug, Clone)]
pub struct NumaNode {
    pub id: u32,
    pub memory_total_kb: u64,
    pub memory_free_kb: u64,
    pub cpus: Vec<u32>,
    pub node_type: NumaNodeType,
}

#[cfg(target_os = "linux")]
#[derive(Debug, Clone)]
pub struct ProcessNumaInfo {
    pub pid: u32,
    pub name: String,
    pub pages_per_node: HashMap<u32, u64>,
    pub total_pages: u64,
    pub cpu_node: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum ActiveView {
    #[default]
    Swap,
    #[cfg(target_os = "linux")]
    Numa,
    Gpu,
    Unified,
}

// --- GPU types ---

#[derive(Debug, Clone)]
pub struct GpuProcessInfo {
    pub pid: u32,
    pub name: String,
    pub gpu_index: u32,
    pub gpu_memory_used_kb: u64,
}

#[derive(Debug, Clone)]
pub struct GpuDevice {
    pub index: u32,
    pub name: String,
    pub memory_total_kb: u64,
    pub memory_used_kb: u64,
    pub memory_free_kb: u64,
    pub numa_node_id: Option<u32>,
    pub temperature: Option<u32>,
    pub pci_bus_id: String,
}

// --- Unified types ---

#[derive(Debug, Clone, PartialEq)]
pub enum ProcessLocation {
    CpuOnly,
    GpuOnly,
    CpuAndGpu,
}

#[derive(Debug, Clone)]
pub struct UnifiedProcessInfo {
    pub pid: u32,
    pub name: String,
    pub swap_kb: u64,
    #[cfg(target_os = "linux")]
    pub numa_node: Option<u32>,
    pub gpu_memory_kb: Option<u64>,
    pub gpu_index: Option<u32>,
    pub location: ProcessLocation,
}

pub fn convert_swap(kb: u64, unit: SizeUnits) -> f64 {
    match unit {
        SizeUnits::KB => kb as f64,
        SizeUnits::MB => kb as f64 / 1024.0,
        SizeUnits::GB => kb as f64 / (1024.0 * 1024.0),
    }
}

pub fn aggregate_processes(processes: Vec<ProcessSwapInfo>) -> Vec<ProcessSwapInfo> {
    let mut name_to_info: HashMap<String, (f64, u32)> = HashMap::new();

    for process in processes {
        let entry = name_to_info.entry(process.name).or_insert((0.0, 0));
        entry.0 += process.swap_size;
        entry.1 += 1;
    }

    let mut aggregated_processes: Vec<ProcessSwapInfo> = name_to_info
        .into_iter()
        .map(|(name, (swap_size, count))| ProcessSwapInfo {
            pid: count,
            name,
            swap_size,
            #[cfg(target_os = "linux")]
            last_cpu: None,
        })
        .collect();

    aggregated_processes.sort_by(|a, b| {
        b.swap_size
            .partial_cmp(&a.swap_size)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    aggregated_processes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_swap_kb() {
        assert_eq!(convert_swap(1024, SizeUnits::KB), 1024.0);
    }

    #[test]
    fn test_convert_swap_mb() {
        assert_eq!(convert_swap(1024, SizeUnits::MB), 1.0);
    }

    #[test]
    fn test_convert_swap_gb() {
        assert_eq!(convert_swap(1048576, SizeUnits::GB), 1.0);
    }

    #[test]
    fn test_aggregate_empty() {
        let result = aggregate_processes(vec![]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_aggregate_dedup() {
        let procs = vec![
            ProcessSwapInfo { pid: 1, name: "firefox".into(), swap_size: 100.0, #[cfg(target_os = "linux")] last_cpu: None },
            ProcessSwapInfo { pid: 2, name: "firefox".into(), swap_size: 200.0, #[cfg(target_os = "linux")] last_cpu: None },
        ];
        let result = aggregate_processes(procs);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "firefox");
        assert_eq!(result[0].swap_size, 300.0);
        assert_eq!(result[0].pid, 2); // count
    }

    #[test]
    fn test_aggregate_sorted() {
        let procs = vec![
            ProcessSwapInfo { pid: 1, name: "small".into(), swap_size: 10.0, #[cfg(target_os = "linux")] last_cpu: None },
            ProcessSwapInfo { pid: 2, name: "big".into(), swap_size: 500.0, #[cfg(target_os = "linux")] last_cpu: None },
            ProcessSwapInfo { pid: 3, name: "medium".into(), swap_size: 100.0, #[cfg(target_os = "linux")] last_cpu: None },
        ];
        let result = aggregate_processes(procs);
        assert_eq!(result[0].name, "big");
        assert_eq!(result[1].name, "medium");
        assert_eq!(result[2].name, "small");
    }

    #[test]
    fn test_size_units_default() {
        assert_eq!(SizeUnits::default(), SizeUnits::KB);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_process_swap_info_has_last_cpu() {
        let info = ProcessSwapInfo {
            pid: 42,
            name: "test".into(),
            swap_size: 100.0,
            last_cpu: Some(3),
        };
        assert_eq!(info.last_cpu, Some(3));
    }
}
