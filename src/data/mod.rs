pub mod gpu;
pub mod swap;
pub mod types;
#[cfg(target_os = "linux")]
pub mod numa;

pub use types::*;

#[cfg(target_os = "linux")]
use std::collections::HashMap;

pub trait DataProvider {
    fn get_swap_info(&self, unit: &SizeUnits) -> Result<SwapUpdate, SwapDataError>;
    fn get_processes_swap(&self, unit: &SizeUnits) -> Result<Vec<ProcessSwapInfo>, SwapDataError>;
    #[cfg(target_os = "linux")]
    fn get_numa_topology(&self) -> Result<Vec<NumaNode>, SwapDataError>;
    #[cfg(target_os = "linux")]
    fn get_process_numa_maps(&self, pid: u32, name: &str) -> Result<ProcessNumaInfo, SwapDataError>;
    #[cfg(target_os = "linux")]
    fn is_numa_available(&self) -> bool;
    fn get_gpu_devices(&self) -> Result<Vec<GpuDevice>, SwapDataError>;
    fn get_gpu_processes(&self) -> Result<Vec<GpuProcessInfo>, SwapDataError>;
    fn is_gpu_available(&self) -> bool;
}

pub struct ProcDataProvider;

impl DataProvider for ProcDataProvider {
    #[cfg(target_os = "linux")]
    fn get_swap_info(&self, unit: &SizeUnits) -> Result<SwapUpdate, SwapDataError> {
        swap::get_chart_info(unit.clone())
    }

    #[cfg(target_os = "windows")]
    fn get_swap_info(&self, _unit: &SizeUnits) -> Result<SwapUpdate, SwapDataError> {
        swap::get_chart_info()
    }

    fn get_processes_swap(&self, unit: &SizeUnits) -> Result<Vec<ProcessSwapInfo>, SwapDataError> {
        swap::get_processes_using_swap(unit.clone())
    }

    #[cfg(target_os = "linux")]
    fn get_numa_topology(&self) -> Result<Vec<NumaNode>, SwapDataError> {
        // Use GPU NUMA mapping if GPUs are available
        let gpu_map = if gpu::check_nvidia_smi_available() {
            if gpu::run_nvidia_smi(&[
                "--query-gpu=index",
                "--format=csv,noheader",
            ]).is_ok() {
                if let Ok(csv_with_units) = gpu::run_nvidia_smi(&[
                    "--query-gpu=index,name,memory.total,memory.used,memory.free,temperature.gpu,pci.bus_id",
                    "--format=csv,noheader",
                ]) {
                    let devices = gpu::parse_gpu_devices_csv(&csv_with_units);
                    gpu::get_gpu_numa_mapping(&devices)
                } else {
                    HashMap::new()
                }
            } else {
                HashMap::new()
            }
        } else {
            HashMap::new()
        };
        numa::discover_numa_topology("/sys/devices/system/node", &gpu_map)
            .map_err(SwapDataError::Io)
    }

    #[cfg(target_os = "linux")]
    fn get_process_numa_maps(&self, pid: u32, name: &str) -> Result<ProcessNumaInfo, SwapDataError> {
        let path = format!("/proc/{}/numa_maps", pid);
        let content = std::fs::read_to_string(&path).map_err(SwapDataError::Io)?;
        Ok(numa::parse_numa_maps(&content, pid, name))
    }

    #[cfg(target_os = "linux")]
    fn is_numa_available(&self) -> bool {
        std::path::Path::new("/sys/devices/system/node/node0").exists()
    }

    fn get_gpu_devices(&self) -> Result<Vec<GpuDevice>, SwapDataError> {
        if !gpu::check_nvidia_smi_available() {
            return Ok(vec![]);
        }
        let csv = gpu::run_nvidia_smi(&[
            "--query-gpu=index,name,memory.total,memory.used,memory.free,temperature.gpu,pci.bus_id",
            "--format=csv,noheader",
        ])
        .map_err(SwapDataError::Io)?;
        let mut devices = gpu::parse_gpu_devices_csv(&csv);
        let numa_map = gpu::get_gpu_numa_mapping(&devices);
        for dev in &mut devices {
            dev.numa_node_id = numa_map.get(&dev.index).copied();
        }
        Ok(devices)
    }

    fn get_gpu_processes(&self) -> Result<Vec<GpuProcessInfo>, SwapDataError> {
        if !gpu::check_nvidia_smi_available() {
            return Ok(vec![]);
        }
        let csv = gpu::run_nvidia_smi(&[
            "--query-compute-apps=gpu_uuid,pid,process_name,used_gpu_memory",
            "--format=csv,noheader",
        ]);
        // Fallback: try the simpler query format
        let csv = match csv {
            Ok(c) => c,
            Err(_) => {
                gpu::run_nvidia_smi(&[
                    "--query-compute-apps=gpu_bus_id,pid,process_name,used_memory",
                    "--format=csv,noheader",
                ])
                .map_err(SwapDataError::Io)?
            }
        };
        Ok(gpu::parse_gpu_processes_csv(&csv))
    }

    fn is_gpu_available(&self) -> bool {
        gpu::check_nvidia_smi_available()
    }
}

#[cfg(test)]
pub struct MockDataProvider {
    pub swap_update: SwapUpdate,
    pub processes: Vec<ProcessSwapInfo>,
    #[cfg(target_os = "linux")]
    pub numa_nodes: Vec<NumaNode>,
    #[cfg(target_os = "linux")]
    pub numa_available: bool,
    pub gpu_devices: Vec<GpuDevice>,
    pub gpu_processes: Vec<GpuProcessInfo>,
    pub gpu_available: bool,
}

#[cfg(test)]
impl MockDataProvider {
    pub fn new() -> Self {
        Self {
            swap_update: SwapUpdate {
                #[cfg(target_os = "linux")]
                swap_devices: vec![],
                total_swap: 8_000_000,
                used_swap: 2_000_000,
            },
            processes: vec![
                ProcessSwapInfo { pid: 1, name: "test_proc".into(), swap_size: 1024.0 },
                ProcessSwapInfo { pid: 2, name: "another".into(), swap_size: 512.0 },
            ],
            #[cfg(target_os = "linux")]
            numa_nodes: vec![
                NumaNode {
                    id: 0,
                    memory_total_kb: 16_000_000,
                    memory_free_kb: 8_000_000,
                    cpus: vec![0, 1, 2, 3],
                    node_type: NumaNodeType::Cpu,
                },
            ],
            #[cfg(target_os = "linux")]
            numa_available: true,
            gpu_devices: vec![],
            gpu_processes: vec![],
            gpu_available: false,
        }
    }
}

#[cfg(test)]
impl DataProvider for MockDataProvider {
    fn get_swap_info(&self, _unit: &SizeUnits) -> Result<SwapUpdate, SwapDataError> {
        Ok(self.swap_update.clone())
    }

    fn get_processes_swap(&self, _unit: &SizeUnits) -> Result<Vec<ProcessSwapInfo>, SwapDataError> {
        Ok(self.processes.clone())
    }

    #[cfg(target_os = "linux")]
    fn get_numa_topology(&self) -> Result<Vec<NumaNode>, SwapDataError> {
        Ok(self.numa_nodes.clone())
    }

    #[cfg(target_os = "linux")]
    fn get_process_numa_maps(&self, pid: u32, name: &str) -> Result<ProcessNumaInfo, SwapDataError> {
        Ok(ProcessNumaInfo {
            pid,
            name: name.to_string(),
            pages_per_node: HashMap::from([(0, 100)]),
            total_pages: 100,
        })
    }

    #[cfg(target_os = "linux")]
    fn is_numa_available(&self) -> bool {
        self.numa_available
    }

    fn get_gpu_devices(&self) -> Result<Vec<GpuDevice>, SwapDataError> {
        Ok(self.gpu_devices.clone())
    }

    fn get_gpu_processes(&self) -> Result<Vec<GpuProcessInfo>, SwapDataError> {
        Ok(self.gpu_processes.clone())
    }

    fn is_gpu_available(&self) -> bool {
        self.gpu_available
    }
}

use std::collections::HashMap as StdHashMap;

/// Merge swap, GPU, and (optionally) NUMA data into unified process info.
/// Joins by PID. Processes appearing in both swap and GPU get `CpuAndGpu`.
#[cfg(target_os = "linux")]
pub fn merge_process_data(
    swap_procs: &[ProcessSwapInfo],
    gpu_procs: &[GpuProcessInfo],
    numa_infos: &[ProcessNumaInfo],
    numa_nodes: &[NumaNode],
) -> Vec<UnifiedProcessInfo> {
    let mut by_pid: StdHashMap<u32, UnifiedProcessInfo> = StdHashMap::new();

    // Insert swap processes
    for p in swap_procs {
        let numa_node = numa_infos
            .iter()
            .find(|n| n.pid == p.pid)
            .and_then(|n| n.pages_per_node.iter().max_by_key(|(_, v)| **v).map(|(k, _)| *k));

        by_pid.insert(
            p.pid,
            UnifiedProcessInfo {
                pid: p.pid,
                name: p.name.clone(),
                swap_kb: p.swap_size as u64,
                numa_node,
                gpu_memory_kb: None,
                gpu_index: None,
                location: ProcessLocation::CpuOnly,
            },
        );
    }

    // Merge GPU processes
    for gp in gpu_procs {
        if let Some(existing) = by_pid.get_mut(&gp.pid) {
            existing.gpu_memory_kb = Some(gp.gpu_memory_used_kb);
            existing.gpu_index = Some(gp.gpu_index);
            existing.location = ProcessLocation::CpuAndGpu;
        } else {
            by_pid.insert(
                gp.pid,
                UnifiedProcessInfo {
                    pid: gp.pid,
                    name: gp.name.clone(),
                    swap_kb: 0,
                    numa_node: None,
                    gpu_memory_kb: Some(gp.gpu_memory_used_kb),
                    gpu_index: Some(gp.gpu_index),
                    location: ProcessLocation::GpuOnly,
                },
            );
        }
    }

    // Check for HBM migration: CPU process with pages on a GPU HBM NUMA node
    let gpu_hbm_nodes: Vec<u32> = numa_nodes
        .iter()
        .filter(|n| matches!(n.node_type, NumaNodeType::GpuHbm { .. }))
        .map(|n| n.id)
        .collect();

    for info in numa_infos {
        if let Some(proc) = by_pid.get_mut(&info.pid) {
            for &node_id in &gpu_hbm_nodes {
                if info.pages_per_node.get(&node_id).copied().unwrap_or(0) > 0 {
                    // Process has pages on GPU HBM — mark as CpuAndGpu if not already
                    if proc.location == ProcessLocation::CpuOnly {
                        proc.location = ProcessLocation::CpuAndGpu;
                    }
                    break;
                }
            }
        }
    }

    let mut result: Vec<UnifiedProcessInfo> = by_pid.into_values().collect();
    // Sort by total memory (swap + gpu) descending
    result.sort_by(|a, b| {
        let total_a = a.swap_kb + a.gpu_memory_kb.unwrap_or(0);
        let total_b = b.swap_kb + b.gpu_memory_kb.unwrap_or(0);
        total_b.cmp(&total_a)
    });
    result
}

/// Simplified merge for non-Linux platforms (no NUMA data).
#[cfg(not(target_os = "linux"))]
pub fn merge_process_data(
    swap_procs: &[ProcessSwapInfo],
    gpu_procs: &[GpuProcessInfo],
) -> Vec<UnifiedProcessInfo> {
    let mut by_pid: StdHashMap<u32, UnifiedProcessInfo> = StdHashMap::new();

    for p in swap_procs {
        by_pid.insert(
            p.pid,
            UnifiedProcessInfo {
                pid: p.pid,
                name: p.name.clone(),
                swap_kb: p.swap_size as u64,
                gpu_memory_kb: None,
                gpu_index: None,
                location: ProcessLocation::CpuOnly,
            },
        );
    }

    for gp in gpu_procs {
        if let Some(existing) = by_pid.get_mut(&gp.pid) {
            existing.gpu_memory_kb = Some(gp.gpu_memory_used_kb);
            existing.gpu_index = Some(gp.gpu_index);
            existing.location = ProcessLocation::CpuAndGpu;
        } else {
            by_pid.insert(
                gp.pid,
                UnifiedProcessInfo {
                    pid: gp.pid,
                    name: gp.name.clone(),
                    swap_kb: 0,
                    gpu_memory_kb: Some(gp.gpu_memory_used_kb),
                    gpu_index: Some(gp.gpu_index),
                    location: ProcessLocation::GpuOnly,
                },
            );
        }
    }

    let mut result: Vec<UnifiedProcessInfo> = by_pid.into_values().collect();
    result.sort_by(|a, b| {
        let total_a = a.swap_kb + a.gpu_memory_kb.unwrap_or(0);
        let total_b = b.swap_kb + b.gpu_memory_kb.unwrap_or(0);
        total_b.cmp(&total_a)
    });
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_provider_returns_data() {
        let provider = MockDataProvider::new();
        let swap_info = provider.get_swap_info(&SizeUnits::KB).unwrap();
        assert_eq!(swap_info.total_swap, 8_000_000);
        assert_eq!(swap_info.used_swap, 2_000_000);

        let procs = provider.get_processes_swap(&SizeUnits::KB).unwrap();
        assert_eq!(procs.len(), 2);
        assert_eq!(procs[0].name, "test_proc");
    }

    #[test]
    fn test_merge_same_pid() {
        let swap = vec![ProcessSwapInfo { pid: 100, name: "train".into(), swap_size: 1024.0 }];
        let gpu = vec![GpuProcessInfo { pid: 100, name: "train".into(), gpu_index: 0, gpu_memory_used_kb: 4096 }];
        let result = merge_process_data(&swap, &gpu, &[], &[]);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].location, ProcessLocation::CpuAndGpu);
        assert_eq!(result[0].swap_kb, 1024);
        assert_eq!(result[0].gpu_memory_kb, Some(4096));
    }

    #[test]
    fn test_cpu_only_process() {
        let swap = vec![ProcessSwapInfo { pid: 100, name: "bash".into(), swap_size: 512.0 }];
        let gpu: Vec<GpuProcessInfo> = vec![];
        let result = merge_process_data(&swap, &gpu, &[], &[]);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].location, ProcessLocation::CpuOnly);
    }

    #[test]
    fn test_gpu_only_process() {
        let swap: Vec<ProcessSwapInfo> = vec![];
        let gpu = vec![GpuProcessInfo { pid: 200, name: "cuda_app".into(), gpu_index: 0, gpu_memory_used_kb: 8192 }];
        let result = merge_process_data(&swap, &gpu, &[], &[]);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].location, ProcessLocation::GpuOnly);
        assert_eq!(result[0].swap_kb, 0);
    }

    #[test]
    fn test_unified_sorting() {
        let swap = vec![
            ProcessSwapInfo { pid: 1, name: "small".into(), swap_size: 100.0 },
            ProcessSwapInfo { pid: 2, name: "big".into(), swap_size: 5000.0 },
        ];
        let gpu = vec![
            GpuProcessInfo { pid: 3, name: "gpu_big".into(), gpu_index: 0, gpu_memory_used_kb: 10000 },
        ];
        let result = merge_process_data(&swap, &gpu, &[], &[]);
        assert_eq!(result.len(), 3);
        // gpu_big (10000) > big (5000) > small (100)
        assert_eq!(result[0].name, "gpu_big");
        assert_eq!(result[1].name, "big");
        assert_eq!(result[2].name, "small");
    }

    #[test]
    fn test_aggregate_unified() {
        // merge_process_data handles aggregation by PID (not by name)
        let swap = vec![
            ProcessSwapInfo { pid: 1, name: "proc".into(), swap_size: 100.0 },
            ProcessSwapInfo { pid: 2, name: "proc".into(), swap_size: 200.0 },
        ];
        let gpu: Vec<GpuProcessInfo> = vec![];
        let result = merge_process_data(&swap, &gpu, &[], &[]);
        // Different PIDs → separate entries (no name-based aggregation in merge)
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_hbm_migration_detected() {
        let swap = vec![ProcessSwapInfo { pid: 100, name: "migrated".into(), swap_size: 1024.0 }];
        let gpu: Vec<GpuProcessInfo> = vec![];
        let numa_infos = vec![ProcessNumaInfo {
            pid: 100,
            name: "migrated".into(),
            pages_per_node: HashMap::from([(0, 500), (2, 100)]), // pages on node 2 (GPU HBM)
            total_pages: 600,
        }];
        let numa_nodes = vec![
            NumaNode { id: 0, memory_total_kb: 16_000_000, memory_free_kb: 8_000_000, cpus: vec![0, 1], node_type: NumaNodeType::Cpu },
            NumaNode { id: 2, memory_total_kb: 81_920_000, memory_free_kb: 40_960_000, cpus: vec![], node_type: NumaNodeType::GpuHbm { gpu_index: 0 } },
        ];
        let result = merge_process_data(&swap, &gpu, &numa_infos, &numa_nodes);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].location, ProcessLocation::CpuAndGpu); // migrated!
    }

    #[test]
    fn test_graceful_no_gpu() {
        let swap = vec![
            ProcessSwapInfo { pid: 1, name: "proc1".into(), swap_size: 100.0 },
            ProcessSwapInfo { pid: 2, name: "proc2".into(), swap_size: 200.0 },
        ];
        let gpu: Vec<GpuProcessInfo> = vec![];
        let result = merge_process_data(&swap, &gpu, &[], &[]);
        assert_eq!(result.len(), 2);
        assert!(result.iter().all(|p| p.location == ProcessLocation::CpuOnly));
    }

    #[test]
    fn test_graceful_no_numa() {
        let swap = vec![ProcessSwapInfo { pid: 1, name: "proc".into(), swap_size: 100.0 }];
        let gpu = vec![GpuProcessInfo { pid: 1, name: "proc".into(), gpu_index: 0, gpu_memory_used_kb: 500 }];
        // No NUMA data at all
        let result = merge_process_data(&swap, &gpu, &[], &[]);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].location, ProcessLocation::CpuAndGpu);
        assert_eq!(result[0].numa_node, None);
    }
}
