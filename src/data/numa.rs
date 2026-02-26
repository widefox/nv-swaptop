use std::collections::HashMap;

use super::types::{NumaNode, NumaNodeType, ProcessNumaInfo};

/// Parse MemTotal and MemFree from a NUMA node's meminfo content.
/// Expects lines like "Node 0 MemTotal:  1234 kB"
pub fn parse_numa_meminfo(content: &str) -> (u64, u64) {
    let mut total = 0u64;
    let mut free = 0u64;
    for line in content.lines() {
        if line.contains("MemTotal:") {
            if let Some(val) = extract_kb_value(line) {
                total = val;
            }
        } else if line.contains("MemFree:") {
            if let Some(val) = extract_kb_value(line) {
                free = val;
            }
        }
    }
    (total, free)
}

fn extract_kb_value(line: &str) -> Option<u64> {
    // Format: "Node N FieldName:     12345 kB"
    let colon_pos = line.find(':')?;
    let after_colon = line[colon_pos + 1..].trim();
    let num_str = after_colon.split_whitespace().next()?;
    num_str.parse().ok()
}

/// Parse a CPU list string like "0-3,8-11" into a sorted Vec of CPU IDs.
/// Empty string returns empty vec (indicates GPU HBM node with no CPUs).
pub fn parse_cpulist(content: &str) -> Vec<u32> {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    let mut cpus = Vec::new();
    for part in trimmed.split(',') {
        let part = part.trim();
        if let Some(dash_pos) = part.find('-') {
            let start: u32 = match part[..dash_pos].parse() {
                Ok(v) => v,
                Err(_) => continue,
            };
            let end: u32 = match part[dash_pos + 1..].parse() {
                Ok(v) => v,
                Err(_) => continue,
            };
            for cpu in start..=end {
                cpus.push(cpu);
            }
        } else if let Ok(cpu) = part.parse::<u32>() {
            cpus.push(cpu);
        }
    }
    cpus.sort();
    cpus
}

/// Classify a NUMA node as CPU, GPU HBM, or Unknown.
/// - Has CPUs → Cpu
/// - No CPUs but in gpu_map → GpuHbm
/// - Otherwise → Unknown
pub fn classify_numa_node(node: &NumaNode, gpu_map: &HashMap<u32, u32>) -> NumaNodeType {
    if !node.cpus.is_empty() {
        NumaNodeType::Cpu
    } else if let Some(&gpu_index) = gpu_map.get(&node.id) {
        NumaNodeType::GpuHbm { gpu_index }
    } else {
        NumaNodeType::Unknown
    }
}

/// Parse /proc/[pid]/numa_maps content into ProcessNumaInfo.
/// Each line has format: "address policy N0=pages N1=pages ..."
pub fn parse_numa_maps(content: &str, pid: u32, name: &str) -> ProcessNumaInfo {
    let mut pages_per_node: HashMap<u32, u64> = HashMap::new();

    for line in content.lines() {
        for token in line.split_whitespace() {
            if let Some(eq_pos) = token.find('=') {
                let key = &token[..eq_pos];
                let val = &token[eq_pos + 1..];
                // Match N<digit>=<pages> pattern
                if key.starts_with('N') {
                    if let (Ok(node_id), Ok(pages)) =
                        (key[1..].parse::<u32>(), val.parse::<u64>())
                    {
                        *pages_per_node.entry(node_id).or_insert(0) += pages;
                    }
                }
            }
        }
    }

    let total_pages = pages_per_node.values().sum();
    ProcessNumaInfo {
        pid,
        name: name.to_string(),
        pages_per_node,
        total_pages,
    }
}

/// Discover NUMA topology by reading sysfs.
/// sys_path should be "/sys/devices/system/node" (or a test path).
pub fn discover_numa_topology(
    sys_path: &str,
    gpu_map: &HashMap<u32, u32>,
) -> std::io::Result<Vec<NumaNode>> {
    let mut nodes = Vec::new();

    let entries = std::fs::read_dir(sys_path)?;
    for entry in entries {
        let entry = entry?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if !name_str.starts_with("node") {
            continue;
        }
        let id_str = &name_str[4..];
        let id: u32 = match id_str.parse() {
            Ok(v) => v,
            Err(_) => continue,
        };

        let node_path = entry.path();

        // Read meminfo
        let meminfo_path = node_path.join("meminfo");
        let meminfo_content = std::fs::read_to_string(&meminfo_path).unwrap_or_default();
        let (memory_total_kb, memory_free_kb) = parse_numa_meminfo(&meminfo_content);

        // Read cpulist
        let cpulist_path = node_path.join("cpulist");
        let cpulist_content = std::fs::read_to_string(&cpulist_path).unwrap_or_default();
        let cpus = parse_cpulist(&cpulist_content);

        let mut node = NumaNode {
            id,
            memory_total_kb,
            memory_free_kb,
            cpus,
            node_type: NumaNodeType::Unknown,
        };
        node.node_type = classify_numa_node(&node, gpu_map);
        nodes.push(node);
    }

    nodes.sort_by_key(|n| n.id);
    Ok(nodes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_meminfo() {
        let content = "\
Node 0 MemTotal:       16384000 kB
Node 0 MemFree:         8192000 kB
Node 0 MemUsed:         8192000 kB";
        let (total, free) = parse_numa_meminfo(content);
        assert_eq!(total, 16384000);
        assert_eq!(free, 8192000);
    }

    #[test]
    fn test_parse_cpulist_range() {
        assert_eq!(parse_cpulist("0-3,8-11"), vec![0, 1, 2, 3, 8, 9, 10, 11]);
    }

    #[test]
    fn test_parse_cpulist_empty() {
        assert_eq!(parse_cpulist(""), Vec::<u32>::new());
    }

    #[test]
    fn test_parse_cpulist_single() {
        assert_eq!(parse_cpulist("5"), vec![5]);
    }

    #[test]
    fn test_classify_cpu_node() {
        let node = NumaNode {
            id: 0,
            memory_total_kb: 16384000,
            memory_free_kb: 8192000,
            cpus: vec![0, 1, 2, 3],
            node_type: NumaNodeType::Unknown,
        };
        let gpu_map = HashMap::new();
        assert_eq!(classify_numa_node(&node, &gpu_map), NumaNodeType::Cpu);
    }

    #[test]
    fn test_classify_gpu_hbm() {
        let node = NumaNode {
            id: 2,
            memory_total_kb: 81920000,
            memory_free_kb: 40960000,
            cpus: vec![],
            node_type: NumaNodeType::Unknown,
        };
        let mut gpu_map = HashMap::new();
        gpu_map.insert(2, 0); // node 2 -> GPU 0
        assert_eq!(
            classify_numa_node(&node, &gpu_map),
            NumaNodeType::GpuHbm { gpu_index: 0 }
        );
    }

    #[test]
    fn test_classify_unknown() {
        let node = NumaNode {
            id: 3,
            memory_total_kb: 0,
            memory_free_kb: 0,
            cpus: vec![],
            node_type: NumaNodeType::Unknown,
        };
        let gpu_map = HashMap::new();
        assert_eq!(classify_numa_node(&node, &gpu_map), NumaNodeType::Unknown);
    }

    #[test]
    fn test_parse_numa_maps() {
        let content = "\
00400000 default N0=10 N1=5
00600000 default N0=3 N2=7";
        let info = parse_numa_maps(content, 42, "test_proc");
        assert_eq!(info.pid, 42);
        assert_eq!(info.name, "test_proc");
        assert_eq!(info.pages_per_node.get(&0), Some(&13));
        assert_eq!(info.pages_per_node.get(&1), Some(&5));
        assert_eq!(info.pages_per_node.get(&2), Some(&7));
    }

    #[test]
    fn test_parse_numa_maps_aggregation() {
        // Multiple regions on same node should sum
        let content = "\
00400000 default N0=100
00500000 default N0=200
00600000 default N1=50";
        let info = parse_numa_maps(content, 1, "proc");
        assert_eq!(info.pages_per_node.get(&0), Some(&300));
        assert_eq!(info.pages_per_node.get(&1), Some(&50));
    }

    #[test]
    fn test_parse_numa_maps_empty() {
        let info = parse_numa_maps("", 1, "empty");
        assert_eq!(info.total_pages, 0);
        assert!(info.pages_per_node.is_empty());
    }

    #[test]
    fn test_total_pages_sum() {
        let content = "00400000 default N0=10 N1=20 N2=30";
        let info = parse_numa_maps(content, 1, "proc");
        let manual_sum: u64 = info.pages_per_node.values().sum();
        assert_eq!(info.total_pages, manual_sum);
        assert_eq!(info.total_pages, 60);
    }

    #[test]
    fn test_topology_sorted() {
        // We can't easily test discover_numa_topology without a real /sys,
        // but we test that the sort logic works by creating nodes and sorting
        let mut nodes = vec![
            NumaNode {
                id: 2,
                memory_total_kb: 0,
                memory_free_kb: 0,
                cpus: vec![],
                node_type: NumaNodeType::Unknown,
            },
            NumaNode {
                id: 0,
                memory_total_kb: 0,
                memory_free_kb: 0,
                cpus: vec![0],
                node_type: NumaNodeType::Cpu,
            },
            NumaNode {
                id: 1,
                memory_total_kb: 0,
                memory_free_kb: 0,
                cpus: vec![1],
                node_type: NumaNodeType::Cpu,
            },
        ];
        nodes.sort_by_key(|n| n.id);
        assert_eq!(nodes[0].id, 0);
        assert_eq!(nodes[1].id, 1);
        assert_eq!(nodes[2].id, 2);
    }
}
