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
        } else if line.contains("MemFree:")
            && let Some(val) = extract_kb_value(line)
        {
            free = val;
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
/// Each line has format: "address policy N0=pages N1=pages ... kernelpagesize_kB=N"
/// Page counts are multiplied by the per-line `kernelpagesize_kB` (or `default_page_size_kb`
/// when absent) and accumulated as KB directly.
pub fn parse_numa_maps(content: &str, pid: u32, name: &str, default_page_size_kb: u64) -> ProcessNumaInfo {
    let mut kb_per_node: HashMap<u32, u64> = HashMap::new();

    for line in content.lines() {
        let mut line_page_size_kb = default_page_size_kb;
        let mut line_nodes: Vec<(u32, u64)> = Vec::new();

        for token in line.split_whitespace() {
            if let Some(eq_pos) = token.find('=') {
                let key = &token[..eq_pos];
                let val = &token[eq_pos + 1..];

                if key == "kernelpagesize_kB" {
                    if let Ok(kps) = val.parse::<u64>() {
                        line_page_size_kb = kps;
                    }
                } else if let Some(node_str) = key.strip_prefix('N')
                    && let (Ok(node_id), Ok(pages)) =
                        (node_str.parse::<u32>(), val.parse::<u64>())
                {
                    line_nodes.push((node_id, pages));
                }
            }
        }

        for (node_id, pages) in line_nodes {
            *kb_per_node.entry(node_id).or_insert(0) += pages * line_page_size_kb;
        }
    }

    let total_kb = kb_per_node.values().sum();
    ProcessNumaInfo {
        pid,
        name: name.to_string(),
        kb_per_node,
        total_kb,
        cpu_node: None,
    }
}

/// Map a CPU ID to the NUMA node it belongs to.
pub fn cpu_to_numa_node(cpu_id: i32, numa_nodes: &[NumaNode]) -> Option<u32> {
    numa_nodes
        .iter()
        .find(|n| n.cpus.contains(&(cpu_id as u32)))
        .map(|n| n.id)
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
        let info = parse_numa_maps(content, 42, "test_proc", 4);
        assert_eq!(info.pid, 42);
        assert_eq!(info.name, "test_proc");
        assert_eq!(info.kb_per_node.get(&0), Some(&52));  // 13 pages * 4 KB
        assert_eq!(info.kb_per_node.get(&1), Some(&20));  // 5 pages * 4 KB
        assert_eq!(info.kb_per_node.get(&2), Some(&28));  // 7 pages * 4 KB
    }

    #[test]
    fn test_parse_numa_maps_aggregation() {
        // Multiple regions on same node should sum
        let content = "\
00400000 default N0=100
00500000 default N0=200
00600000 default N1=50";
        let info = parse_numa_maps(content, 1, "proc", 4);
        assert_eq!(info.kb_per_node.get(&0), Some(&1200));  // 300 pages * 4 KB
        assert_eq!(info.kb_per_node.get(&1), Some(&200));   // 50 pages * 4 KB
    }

    #[test]
    fn test_parse_numa_maps_empty() {
        let info = parse_numa_maps("", 1, "empty", 4);
        assert_eq!(info.total_kb, 0);
        assert!(info.kb_per_node.is_empty());
    }

    #[test]
    fn test_total_kb_sum() {
        let content = "00400000 default N0=10 N1=20 N2=30";
        let info = parse_numa_maps(content, 1, "proc", 4);
        let manual_sum: u64 = info.kb_per_node.values().sum();
        assert_eq!(info.total_kb, manual_sum);
        assert_eq!(info.total_kb, 240);  // 60 pages * 4 KB
    }

    #[test]
    fn test_process_numa_info_has_cpu_node() {
        let info = ProcessNumaInfo {
            pid: 42,
            name: "test".into(),
            kb_per_node: HashMap::from([(0, 400), (1, 200)]),
            total_kb: 600,
            cpu_node: Some(1),
        };
        assert_eq!(info.cpu_node, Some(1));
    }

    #[test]
    fn test_parse_numa_maps_cpu_node_is_none() {
        // parse_numa_maps doesn't know about CPU scheduling, so cpu_node should be None
        let content = "00400000 default N0=10 N1=5";
        let info = parse_numa_maps(content, 42, "test_proc", 4);
        assert_eq!(info.cpu_node, None);
    }

    // --- Page-size-aware tests ---

    #[test]
    fn test_parse_numa_maps_with_kernelpagesize() {
        // Mix of 4KB and 2MB pages on same node
        let content = "\
00400000 default N0=8 kernelpagesize_kB=4
00600000 default N0=512 kernelpagesize_kB=2048";
        let info = parse_numa_maps(content, 1, "proc", 4);
        // N0 = 8*4 + 512*2048 = 32 + 1,048,576 = 1,048,608 KB
        assert_eq!(info.kb_per_node.get(&0), Some(&1_048_608));
    }

    #[test]
    fn test_parse_numa_maps_mixed_pagesizes_multi_node() {
        let content = "\
00400000 default N0=100 N1=200 kernelpagesize_kB=4
00600000 default N0=200 kernelpagesize_kB=2048
00800000 default N1=10 kernelpagesize_kB=64";
        let info = parse_numa_maps(content, 1, "proc", 4);
        // N0 = 100*4 + 200*2048 = 400 + 409,600 = 410,000 KB
        assert_eq!(info.kb_per_node.get(&0), Some(&410_000));
        // N1 = 200*4 + 10*64 = 800 + 640 = 1,440 KB
        assert_eq!(info.kb_per_node.get(&1), Some(&1_440));
    }

    #[test]
    fn test_parse_numa_maps_default_64kb() {
        // No kernelpagesize_kB, default is 64 (aarch64)
        let content = "\
00400000 default N0=10 N1=5";
        let info = parse_numa_maps(content, 1, "proc", 64);
        assert_eq!(info.kb_per_node.get(&0), Some(&640));   // 10 * 64
        assert_eq!(info.kb_per_node.get(&1), Some(&320));   // 5 * 64
    }

    #[test]
    fn test_parse_numa_maps_hugepages_1gb() {
        let content = "7f000000 default N0=4 kernelpagesize_kB=1048576";
        let info = parse_numa_maps(content, 1, "proc", 4);
        // 4 * 1,048,576 = 4,194,304 KB
        assert_eq!(info.kb_per_node.get(&0), Some(&4_194_304));
    }

    #[test]
    fn test_parse_numa_maps_kernelpagesize_before_nodes() {
        // kernelpagesize_kB appears before N= tokens
        let content = "00400000 default kernelpagesize_kB=2048 N0=100";
        let info = parse_numa_maps(content, 1, "proc", 4);
        assert_eq!(info.kb_per_node.get(&0), Some(&204_800));  // 100 * 2048
    }

    #[test]
    fn test_parse_numa_maps_line_without_nodes() {
        // First line has kernelpagesize but no N= tokens; should not contribute
        let content = "\
00400000 default kernelpagesize_kB=2048
00600000 default N0=10 kernelpagesize_kB=4";
        let info = parse_numa_maps(content, 1, "proc", 4);
        assert_eq!(info.kb_per_node.get(&0), Some(&40));  // 10 * 4
        assert_eq!(info.total_kb, 40);
    }

    #[test]
    fn test_cpu_to_numa_node_found() {
        let nodes = vec![
            NumaNode { id: 0, memory_total_kb: 0, memory_free_kb: 0, cpus: vec![0, 1, 2, 3], node_type: NumaNodeType::Cpu },
            NumaNode { id: 1, memory_total_kb: 0, memory_free_kb: 0, cpus: vec![4, 5, 6, 7], node_type: NumaNodeType::Cpu },
        ];
        assert_eq!(cpu_to_numa_node(0, &nodes), Some(0));
        assert_eq!(cpu_to_numa_node(3, &nodes), Some(0));
        assert_eq!(cpu_to_numa_node(4, &nodes), Some(1));
        assert_eq!(cpu_to_numa_node(7, &nodes), Some(1));
    }

    #[test]
    fn test_cpu_to_numa_node_not_found() {
        let nodes = vec![
            NumaNode { id: 0, memory_total_kb: 0, memory_free_kb: 0, cpus: vec![0, 1], node_type: NumaNodeType::Cpu },
        ];
        assert_eq!(cpu_to_numa_node(99, &nodes), None);
    }

    #[test]
    fn test_cpu_to_numa_node_gpu_hbm_skipped() {
        // GPU HBM nodes have no CPUs, so should never match
        let nodes = vec![
            NumaNode { id: 0, memory_total_kb: 0, memory_free_kb: 0, cpus: vec![0, 1], node_type: NumaNodeType::Cpu },
            NumaNode { id: 2, memory_total_kb: 0, memory_free_kb: 0, cpus: vec![], node_type: NumaNodeType::GpuHbm { gpu_index: 0 } },
        ];
        assert_eq!(cpu_to_numa_node(0, &nodes), Some(0));
        assert_eq!(cpu_to_numa_node(5, &nodes), None);
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
