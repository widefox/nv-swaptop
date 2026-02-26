use std::collections::HashMap;
use std::process::Command;

use super::types::{GpuDevice, GpuProcessInfo};

/// Convert MiB (nvidia-smi unit) to KB (internal unit).
fn mib_to_kb(mib: u64) -> u64 {
    mib * 1024
}

/// Parse nvidia-smi CSV output for GPU processes.
/// Expected CSV format: gpu_index, pid, process_name, used_gpu_memory [MiB]
pub fn parse_gpu_processes_csv(csv: &str) -> Vec<GpuProcessInfo> {
    let mut results = Vec::new();
    for line in csv.lines() {
        let line = line.trim();
        // Skip headers and empty lines
        if line.is_empty()
            || line.starts_with("gpu")
            || line.starts_with('#')
            || line.contains("[Not Supported]")
            || line.starts_with("index")
        {
            continue;
        }

        let parts: Vec<&str> = line.split(", ").collect();
        if parts.len() < 4 {
            continue;
        }

        let gpu_index = match parts[0].trim().parse::<u32>() {
            Ok(v) => v,
            Err(_) => continue,
        };
        let pid = match parts[1].trim().parse::<u32>() {
            Ok(v) => v,
            Err(_) => continue,
        };
        let name = parts[2].trim().to_string();
        let mem_str = parts[3].trim().replace(" MiB", "");
        let mem_mib = match mem_str.parse::<u64>() {
            Ok(v) => v,
            Err(_) => continue,
        };

        results.push(GpuProcessInfo {
            pid,
            name,
            gpu_index,
            gpu_memory_used_kb: mib_to_kb(mem_mib),
        });
    }
    results
}

/// Parse nvidia-smi CSV output for GPU devices.
/// Expected CSV: index, name, memory.total [MiB], memory.used [MiB], memory.free [MiB],
///               temperature.gpu, pci.bus_id
pub fn parse_gpu_devices_csv(csv: &str) -> Vec<GpuDevice> {
    let mut results = Vec::new();
    for line in csv.lines() {
        let line = line.trim();
        if line.is_empty()
            || line.starts_with("index")
            || line.starts_with('#')
            || line.starts_with("name")
        {
            continue;
        }

        let parts: Vec<&str> = line.split(", ").collect();
        if parts.len() < 7 {
            continue;
        }

        let index = match parts[0].trim().parse::<u32>() {
            Ok(v) => v,
            Err(_) => continue,
        };
        let name = parts[1].trim().to_string();

        let mem_total = parse_mib_field(parts[2]);
        let mem_used = parse_mib_field(parts[3]);
        let mem_free = parse_mib_field(parts[4]);

        let temperature = parts[5].trim().parse::<u32>().ok();
        let pci_bus_id = parts[6].trim().to_string();

        results.push(GpuDevice {
            index,
            name,
            memory_total_kb: mib_to_kb(mem_total),
            memory_used_kb: mib_to_kb(mem_used),
            memory_free_kb: mib_to_kb(mem_free),
            numa_node_id: None, // filled later by get_gpu_numa_mapping
            temperature,
            pci_bus_id,
        });
    }
    results
}

fn parse_mib_field(s: &str) -> u64 {
    s.trim().replace(" MiB", "").parse().unwrap_or(0)
}

/// Run nvidia-smi with given arguments and return stdout.
pub fn run_nvidia_smi(args: &[&str]) -> Result<String, std::io::Error> {
    let output = Command::new("nvidia-smi").args(args).output()?;
    if !output.status.success() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!(
                "nvidia-smi failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ),
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Check if nvidia-smi is available on the system.
pub fn check_nvidia_smi_available() -> bool {
    Command::new("nvidia-smi")
        .arg("--query-gpu=index")
        .arg("--format=csv,noheader")
        .output()
        .is_ok_and(|o| o.status.success())
}

/// Map GPU PCI bus IDs to NUMA node IDs via sysfs.
/// Returns HashMap<gpu_index, numa_node_id>.
pub fn get_gpu_numa_mapping(devices: &[GpuDevice]) -> HashMap<u32, u32> {
    let mut mapping = HashMap::new();
    for device in devices {
        // PCI bus ID from nvidia-smi is like "00000000:01:00.0"
        // sysfs path: /sys/bus/pci/devices/<bus_id>/numa_node
        let sysfs_path = format!("/sys/bus/pci/devices/{}/numa_node", device.pci_bus_id);
        if let Ok(content) = std::fs::read_to_string(&sysfs_path) {
            if let Ok(node_id) = content.trim().parse::<i32>() {
                // -1 means no NUMA affinity
                if node_id >= 0 {
                    mapping.insert(device.index, node_id as u32);
                }
            }
        }
    }
    mapping
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_gpu_processes() {
        let csv = "0, 1234, python3, 2048 MiB\n";
        let result = parse_gpu_processes_csv(csv);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].pid, 1234);
        assert_eq!(result[0].name, "python3");
        assert_eq!(result[0].gpu_index, 0);
        assert_eq!(result[0].gpu_memory_used_kb, 2048 * 1024);
    }

    #[test]
    fn test_parse_gpu_processes_empty() {
        let csv = "";
        let result = parse_gpu_processes_csv(csv);
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_gpu_processes_malformed() {
        let csv = "this is not valid csv\n0, not_a_pid, proc, 100 MiB\n";
        let result = parse_gpu_processes_csv(csv);
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_gpu_devices() {
        let csv = "0, NVIDIA H100, 81920 MiB, 40960 MiB, 40960 MiB, 45, 00000000:01:00.0\n";
        let result = parse_gpu_devices_csv(csv);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].index, 0);
        assert_eq!(result[0].name, "NVIDIA H100");
        assert_eq!(result[0].memory_total_kb, 81920 * 1024);
        assert_eq!(result[0].memory_used_kb, 40960 * 1024);
        assert_eq!(result[0].temperature, Some(45));
        assert_eq!(result[0].pci_bus_id, "00000000:01:00.0");
    }

    #[test]
    fn test_gpu_mib_to_kb() {
        assert_eq!(mib_to_kb(1), 1024);
        assert_eq!(mib_to_kb(1024), 1024 * 1024);
    }

    #[test]
    fn test_parse_multiple_gpus() {
        let csv = "\
0, NVIDIA H100, 81920 MiB, 10000 MiB, 71920 MiB, 42, 00000000:01:00.0
1, NVIDIA H100, 81920 MiB, 20000 MiB, 61920 MiB, 50, 00000000:02:00.0
2, NVIDIA H100, 81920 MiB, 5000 MiB, 76920 MiB, 38, 00000000:03:00.0";
        let result = parse_gpu_devices_csv(csv);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].index, 0);
        assert_eq!(result[1].index, 1);
        assert_eq!(result[2].index, 2);
    }

    #[test]
    fn test_header_row_skipped() {
        let csv = "\
index, name, memory.total [MiB], memory.used [MiB], memory.free [MiB], temperature.gpu, pci.bus_id
0, NVIDIA H100, 81920 MiB, 40960 MiB, 40960 MiB, 45, 00000000:01:00.0";
        let result = parse_gpu_devices_csv(csv);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "NVIDIA H100");
    }

    #[test]
    fn test_gpu_numa_mapping() {
        // Can't test sysfs reads without real hardware, but verify the function
        // handles empty input correctly
        let devices: Vec<GpuDevice> = vec![];
        let mapping = get_gpu_numa_mapping(&devices);
        assert!(mapping.is_empty());
    }

    #[test]
    fn test_mock_provider_gpu() {
        // Verify parse_gpu_processes_csv + parse_gpu_devices_csv round-trip
        let proc_csv = "0, 100, train.py, 4096 MiB\n1, 200, infer.py, 2048 MiB\n";
        let dev_csv = "0, H100, 81920 MiB, 4096 MiB, 77824 MiB, 45, 00000000:01:00.0\n\
                        1, H100, 81920 MiB, 2048 MiB, 79872 MiB, 40, 00000000:02:00.0\n";
        let procs = parse_gpu_processes_csv(proc_csv);
        let devs = parse_gpu_devices_csv(dev_csv);
        assert_eq!(procs.len(), 2);
        assert_eq!(devs.len(), 2);
        assert_eq!(procs[0].gpu_memory_used_kb, 4096 * 1024);
        assert_eq!(devs[1].memory_free_kb, 79872 * 1024);
    }
}
