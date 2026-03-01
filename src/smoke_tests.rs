//! Integration and smoke tests for nv-swaptop.
//!
//! Part 1: Integration tests use MockDataProvider + TestBackend (no real I/O).
//! Part 2: Smoke tests hit real /proc and /sys paths, with runtime guards.

use std::collections::HashMap;

use ratatui::{Terminal, backend::TestBackend};

use crate::app::{App, SortColumn};
use crate::data::{
    DataProvider, GpuDevice, GpuProcessInfo, MockDataProvider, ProcDataProvider, ProcessLocation,
    SizeUnits, SwapUpdate, merge_process_data,
};
use crate::data::types::{ActiveView, NumaNode, NumaNodeType, ProcessNumaInfo, UnifiedProcessInfo};
use crate::theme::{Theme, ThemeType};
use crate::ui;

// ─── Helpers ──────────────────────────────────────────────────────────────

fn make_test_terminal() -> Terminal<TestBackend> {
    Terminal::new(TestBackend::new(160, 50)).unwrap()
}

/// Rich mock: 2 GPUs with NUMA mapping, 2 CPU NUMA nodes + 1 GPU HBM node,
/// swap processes with last_cpu, GPU processes.
fn make_rich_mock() -> MockDataProvider {
    let mut mock = MockDataProvider::new();
    mock.swap_update = SwapUpdate {
        swap_devices: vec![],
        total_swap: 16_000_000,
        used_swap: 4_000_000,
    };
    mock.processes = vec![
        crate::data::ProcessSwapInfo {
            pid: 100,
            name: "train_model".into(),
            swap_size: 2048.0,
            last_cpu: Some(0),
        },
        crate::data::ProcessSwapInfo {
            pid: 200,
            name: "data_loader".into(),
            swap_size: 1024.0,
            last_cpu: Some(4),
        },
        crate::data::ProcessSwapInfo {
            pid: 300,
            name: "monitor".into(),
            swap_size: 256.0,
            last_cpu: Some(1),
        },
    ];
    mock.numa_nodes = vec![
        NumaNode {
            id: 0,
            memory_total_kb: 16_000_000,
            memory_free_kb: 8_000_000,
            cpus: vec![0, 1, 2, 3],
            node_type: NumaNodeType::Cpu,
        },
        NumaNode {
            id: 1,
            memory_total_kb: 16_000_000,
            memory_free_kb: 10_000_000,
            cpus: vec![4, 5, 6, 7],
            node_type: NumaNodeType::Cpu,
        },
        NumaNode {
            id: 2,
            memory_total_kb: 81_920_000,
            memory_free_kb: 40_960_000,
            cpus: vec![],
            node_type: NumaNodeType::GpuHbm { gpu_index: 0 },
        },
    ];
    mock.numa_available = true;
    mock.gpu_devices = vec![
        GpuDevice {
            index: 0,
            name: "NVIDIA H100".into(),
            memory_total_kb: 81_920_000,
            memory_used_kb: 40_000_000,
            memory_free_kb: 41_920_000,
            numa_node_id: Some(2),
            temperature: Some(55),
            pci_bus_id: "00:01.0".into(),
        },
        GpuDevice {
            index: 1,
            name: "NVIDIA H100".into(),
            memory_total_kb: 81_920_000,
            memory_used_kb: 20_000_000,
            memory_free_kb: 61_920_000,
            numa_node_id: Some(3),
            temperature: Some(42),
            pci_bus_id: "00:02.0".into(),
        },
    ];
    mock.gpu_processes = vec![
        GpuProcessInfo {
            pid: 100,
            name: "train_model".into(),
            gpu_index: 0,
            gpu_memory_used_kb: 30_000_000,
        },
        GpuProcessInfo {
            pid: 100,
            name: "train_model".into(),
            gpu_index: 1,
            gpu_memory_used_kb: 15_000_000,
        },
    ];
    mock.gpu_available = true;
    mock
}

fn gpu_smoke_available() -> bool {
    std::process::Command::new("nvidia-smi")
        .arg("--query-gpu=index")
        .arg("--format=csv,noheader")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn numa_smoke_available() -> bool {
    std::path::Path::new("/sys/devices/system/node/node0").exists()
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Part 1: Integration Tests (MockDataProvider, no real I/O)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[test]
fn test_full_pipeline_swap_gpu_numa_merge() {
    let mock = make_rich_mock();
    let swap_procs = mock.processes.clone();
    let gpu_procs = mock.gpu_processes.clone();

    let numa_infos = vec![
        ProcessNumaInfo {
            pid: 100,
            name: "train_model".into(),
            kb_per_node: HashMap::from([(0, 1000), (2, 500)]),
            total_kb: 1500,
            cpu_node: Some(0),
        },
        ProcessNumaInfo {
            pid: 200,
            name: "data_loader".into(),
            kb_per_node: HashMap::from([(1, 800)]),
            total_kb: 800,
            cpu_node: Some(1),
        },
    ];

    let result = merge_process_data(
        &swap_procs,
        &gpu_procs,
        &numa_infos,
        &mock.numa_nodes,
        &mock.gpu_devices,
    );

    // train_model (pid=100): swap + 2 GPUs = CpuAndGpu
    let train = result.iter().find(|p| p.pid == 100).unwrap();
    assert_eq!(train.location, ProcessLocation::CpuAndGpu);
    assert_eq!(train.gpu_memory_kb, Some(30_000_000 + 15_000_000));
    assert_eq!(train.gpu_indices.len(), 2);
    assert!(train.gpu_indices.contains(&0));
    assert!(train.gpu_indices.contains(&1));
    assert_eq!(train.kb_per_node.get(&0), Some(&1000));
    assert_eq!(train.kb_per_node.get(&2), Some(&500));

    // data_loader (pid=200): swap only = CpuOnly
    let loader = result.iter().find(|p| p.pid == 200).unwrap();
    assert_eq!(loader.location, ProcessLocation::CpuOnly);
    assert!(loader.gpu_memory_kb.is_none());
    assert_eq!(loader.kb_per_node.get(&1), Some(&800));

    // Sorted by total memory desc: train_model (45M GPU + 2K swap) > data_loader (1K) > monitor (256)
    assert_eq!(result[0].pid, 100);
}

#[test]
fn test_full_pipeline_no_gpu() {
    let mock = make_rich_mock();
    let swap_procs = mock.processes.clone();
    let gpu_procs: Vec<GpuProcessInfo> = vec![];

    let result = merge_process_data(
        &swap_procs,
        &gpu_procs,
        &[],
        &mock.numa_nodes,
        &[],
    );

    assert_eq!(result.len(), 3);
    for proc in &result {
        assert_eq!(proc.location, ProcessLocation::CpuOnly);
        assert!(proc.gpu_memory_kb.is_none());
        assert!(proc.gpu_indices.is_empty());
    }
}

#[test]
fn test_full_pipeline_no_numa() {
    let mock = make_rich_mock();
    let swap_procs = mock.processes.clone();
    let gpu_procs = mock.gpu_processes.clone();

    // No NUMA infos, no NUMA nodes
    let result = merge_process_data(
        &swap_procs,
        &gpu_procs,
        &[],
        &[],
        &mock.gpu_devices,
    );

    // train_model has GPU, so CpuAndGpu
    let train = result.iter().find(|p| p.pid == 100).unwrap();
    assert_eq!(train.location, ProcessLocation::CpuAndGpu);
    assert!(train.kb_per_node.is_empty());
    assert_eq!(train.gpu_memory_kb, Some(45_000_000));

    // Others are CpuOnly
    let loader = result.iter().find(|p| p.pid == 200).unwrap();
    assert_eq!(loader.location, ProcessLocation::CpuOnly);
    assert!(loader.kb_per_node.is_empty());
}

#[test]
fn test_full_pipeline_empty_system() {
    let result = merge_process_data(&[], &[], &[], &[], &[]);
    assert!(result.is_empty());
}

#[test]
fn test_numa_maps_to_merge_kb_pipeline() {
    // Parse numa_maps with mixed page sizes, then merge
    let numa_maps_content = "\
00400000 default N0=100 N1=50 kernelpagesize_kB=4
00600000 default N0=10 kernelpagesize_kB=2048";

    let info = crate::data::numa::parse_numa_maps(numa_maps_content, 42, "mixed_app", 4);
    // N0 = 100*4 + 10*2048 = 400 + 20480 = 20880
    assert_eq!(info.kb_per_node.get(&0), Some(&20_880));
    // N1 = 50*4 = 200
    assert_eq!(info.kb_per_node.get(&1), Some(&200));

    let swap_procs = vec![crate::data::ProcessSwapInfo {
        pid: 42,
        name: "mixed_app".into(),
        swap_size: 512.0,
        last_cpu: Some(0),
    }];

    let numa_nodes = vec![
        NumaNode { id: 0, memory_total_kb: 16_000_000, memory_free_kb: 8_000_000, cpus: vec![0, 1], node_type: NumaNodeType::Cpu },
        NumaNode { id: 1, memory_total_kb: 16_000_000, memory_free_kb: 8_000_000, cpus: vec![2, 3], node_type: NumaNodeType::Cpu },
    ];

    let result = merge_process_data(&swap_procs, &[], &[info], &numa_nodes, &[]);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].kb_per_node.get(&0), Some(&20_880));
    assert_eq!(result[0].kb_per_node.get(&1), Some(&200));
}

#[test]
fn test_hbm_migration_full_pipeline() {
    // CPU process with pages on a GPU HBM NUMA node → merge detects CpuAndGpu
    let swap_procs = vec![crate::data::ProcessSwapInfo {
        pid: 500,
        name: "migrated_app".into(),
        swap_size: 1024.0,
        last_cpu: Some(0),
    }];

    let numa_infos = vec![ProcessNumaInfo {
        pid: 500,
        name: "migrated_app".into(),
        kb_per_node: HashMap::from([(0, 2000), (2, 300)]),
        total_kb: 2300,
        cpu_node: Some(0),
    }];

    let numa_nodes = vec![
        NumaNode { id: 0, memory_total_kb: 16_000_000, memory_free_kb: 8_000_000, cpus: vec![0, 1, 2, 3], node_type: NumaNodeType::Cpu },
        NumaNode { id: 2, memory_total_kb: 81_920_000, memory_free_kb: 40_960_000, cpus: vec![], node_type: NumaNodeType::GpuHbm { gpu_index: 0 } },
    ];

    let result = merge_process_data(&swap_procs, &[], &numa_infos, &numa_nodes, &[]);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].location, ProcessLocation::CpuAndGpu);
    assert!(result[0].gpu_indices.is_empty()); // No GPU process, just HBM migration
    assert_eq!(result[0].kb_per_node.get(&2), Some(&300));
}

#[test]
fn test_render_numa_view_smoke() {
    let mut terminal = make_test_terminal();
    let theme = Theme::from(ThemeType::Dracula);

    let numa_nodes = vec![
        NumaNode { id: 0, memory_total_kb: 16_000_000, memory_free_kb: 8_000_000, cpus: vec![0, 1, 2, 3], node_type: NumaNodeType::Cpu },
        NumaNode { id: 1, memory_total_kb: 16_000_000, memory_free_kb: 10_000_000, cpus: vec![4, 5, 6, 7], node_type: NumaNodeType::Cpu },
    ];
    let process_infos = vec![ProcessNumaInfo {
        pid: 42,
        name: "test_proc".into(),
        kb_per_node: HashMap::from([(0, 500), (1, 200)]),
        total_kb: 700,
        cpu_node: Some(0),
    }];

    terminal
        .draw(|frame| {
            ui::numa_view::render_numa_view(
                frame,
                frame.area(),
                &theme,
                &numa_nodes,
                &process_infos,
                true,
                &SizeUnits::KB,
            );
        })
        .unwrap();

    let buf = terminal.backend().buffer().clone();
    // Buffer should be non-empty (has content beyond just spaces)
    let content: String = buf.content().iter().map(|c| c.symbol().to_string()).collect();
    assert!(content.contains("NUMA"));
}

#[test]
fn test_render_numa_view_unavailable() {
    let mut terminal = make_test_terminal();
    let theme = Theme::from(ThemeType::Dracula);

    terminal
        .draw(|frame| {
            ui::numa_view::render_numa_view(
                frame,
                frame.area(),
                &theme,
                &[],
                &[],
                false,
                &SizeUnits::KB,
            );
        })
        .unwrap();

    // Should not panic; check for "not available" message
    let buf = terminal.backend().buffer().clone();
    let content: String = buf.content().iter().map(|c| c.symbol().to_string()).collect();
    assert!(content.contains("not available"));
}

#[test]
fn test_render_gpu_view_smoke() {
    let mut terminal = make_test_terminal();
    let theme = Theme::from(ThemeType::Dracula);

    let devices = vec![GpuDevice {
        index: 0,
        name: "NVIDIA H100".into(),
        memory_total_kb: 81_920_000,
        memory_used_kb: 40_000_000,
        memory_free_kb: 41_920_000,
        numa_node_id: Some(2),
        temperature: Some(55),
        pci_bus_id: "00:01.0".into(),
    }];
    let processes = vec![GpuProcessInfo {
        pid: 100,
        name: "train_model".into(),
        gpu_index: 0,
        gpu_memory_used_kb: 30_000_000,
    }];

    terminal
        .draw(|frame| {
            ui::gpu_view::render_gpu_view(
                frame,
                frame.area(),
                &theme,
                &devices,
                &processes,
                true,
                &SizeUnits::KB,
            );
        })
        .unwrap();

    let buf = terminal.backend().buffer().clone();
    let content: String = buf.content().iter().map(|c| c.symbol().to_string()).collect();
    assert!(content.contains("GPU"));
}

#[test]
fn test_render_gpu_view_no_gpu() {
    let mut terminal = make_test_terminal();
    let theme = Theme::from(ThemeType::Dracula);

    terminal
        .draw(|frame| {
            ui::gpu_view::render_gpu_view(
                frame,
                frame.area(),
                &theme,
                &[],
                &[],
                false,
                &SizeUnits::KB,
            );
        })
        .unwrap();

    let buf = terminal.backend().buffer().clone();
    let content: String = buf.content().iter().map(|c| c.symbol().to_string()).collect();
    assert!(content.contains("No NVIDIA GPU"));
}

#[test]
fn test_render_unified_view_smoke() {
    let mut terminal = make_test_terminal();
    let theme = Theme::from(ThemeType::Dracula);

    let numa_nodes = vec![
        NumaNode { id: 0, memory_total_kb: 16_000_000, memory_free_kb: 8_000_000, cpus: vec![0, 1, 2, 3], node_type: NumaNodeType::Cpu },
        NumaNode { id: 1, memory_total_kb: 16_000_000, memory_free_kb: 10_000_000, cpus: vec![4, 5, 6, 7], node_type: NumaNodeType::Cpu },
    ];

    let procs = vec![
        UnifiedProcessInfo {
            pid: 100,
            name: "train_model".into(),
            swap_kb: 2048,
            cpu_nodes: vec![0],
            gpu_nodes: vec![2],
            kb_per_node: HashMap::from([(0, 1000), (1, 200)]),
            gpu_memory_kb: Some(30_000_000),
            gpu_indices: vec![0],
            location: ProcessLocation::CpuAndGpu,
        },
        UnifiedProcessInfo {
            pid: 200,
            name: "bash".into(),
            swap_kb: 512,
            cpu_nodes: vec![1],
            gpu_nodes: vec![],
            kb_per_node: HashMap::from([(1, 400)]),
            gpu_memory_kb: None,
            gpu_indices: vec![],
            location: ProcessLocation::CpuOnly,
        },
    ];

    terminal
        .draw(|frame| {
            ui::unified_view::render_unified_view(
                frame,
                frame.area(),
                &theme,
                &procs,
                &SizeUnits::KB,
                &numa_nodes,
            );
        })
        .unwrap();

    let buf = terminal.backend().buffer().clone();
    let content: String = buf.content().iter().map(|c| c.symbol().to_string()).collect();
    assert!(content.contains("Unified"));
}

#[test]
fn test_render_unified_view_empty() {
    let mut terminal = make_test_terminal();
    let theme = Theme::from(ThemeType::Dracula);

    terminal
        .draw(|frame| {
            ui::unified_view::render_unified_view(
                frame,
                frame.area(),
                &theme,
                &[],
                &SizeUnits::KB,
                &[],
            );
        })
        .unwrap();

    let buf = terminal.backend().buffer().clone();
    let content: String = buf.content().iter().map(|c| c.symbol().to_string()).collect();
    assert!(content.contains("No process data"));
}

#[test]
fn test_render_unified_view_gb200() {
    // 6 NUMA nodes: 2 CPU + 4 GPU HBM, processes spanning multiple nodes
    let mut terminal = make_test_terminal();
    let theme = Theme::from(ThemeType::Dracula);

    let numa_nodes = vec![
        NumaNode { id: 0, memory_total_kb: 128_000_000, memory_free_kb: 64_000_000, cpus: vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35], node_type: NumaNodeType::Cpu },
        NumaNode { id: 1, memory_total_kb: 128_000_000, memory_free_kb: 80_000_000, cpus: vec![36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 67, 68, 69, 70, 71], node_type: NumaNodeType::Cpu },
        NumaNode { id: 2, memory_total_kb: 98_304_000, memory_free_kb: 50_000_000, cpus: vec![], node_type: NumaNodeType::GpuHbm { gpu_index: 0 } },
        NumaNode { id: 3, memory_total_kb: 98_304_000, memory_free_kb: 60_000_000, cpus: vec![], node_type: NumaNodeType::GpuHbm { gpu_index: 1 } },
        NumaNode { id: 4, memory_total_kb: 98_304_000, memory_free_kb: 70_000_000, cpus: vec![], node_type: NumaNodeType::GpuHbm { gpu_index: 2 } },
        NumaNode { id: 5, memory_total_kb: 98_304_000, memory_free_kb: 90_000_000, cpus: vec![], node_type: NumaNodeType::GpuHbm { gpu_index: 3 } },
    ];

    let procs = vec![
        UnifiedProcessInfo {
            pid: 1000,
            name: "nemo_train".into(),
            swap_kb: 8192,
            cpu_nodes: vec![0],
            gpu_nodes: vec![2, 3, 4, 5],
            kb_per_node: HashMap::from([(0, 5000), (1, 1000), (2, 20_000_000), (3, 18_000_000), (4, 15_000_000), (5, 10_000_000)]),
            gpu_memory_kb: Some(63_000_000),
            gpu_indices: vec![0, 1, 2, 3],
            location: ProcessLocation::CpuAndGpu,
        },
        UnifiedProcessInfo {
            pid: 1001,
            name: "preprocessing".into(),
            swap_kb: 1024,
            cpu_nodes: vec![1],
            gpu_nodes: vec![],
            kb_per_node: HashMap::from([(1, 3000)]),
            gpu_memory_kb: None,
            gpu_indices: vec![],
            location: ProcessLocation::CpuOnly,
        },
    ];

    terminal
        .draw(|frame| {
            ui::unified_view::render_unified_view(
                frame,
                frame.area(),
                &theme,
                &procs,
                &SizeUnits::MB,
                &numa_nodes,
            );
        })
        .unwrap();

    // Should not panic with 6 NUMA columns, and content should include HBM labels
    let buf = terminal.backend().buffer().clone();
    let content: String = buf.content().iter().map(|c| c.symbol().to_string()).collect();
    assert!(content.contains("HBM"));
}

#[test]
fn test_app_view_cycling() {
    let mock = MockDataProvider::new();
    let mut app = App::new(Box::new(mock), false);

    assert_eq!(app.active_view, ActiveView::Swap);

    app.cycle_view();
    assert_eq!(app.active_view, ActiveView::Numa);

    app.cycle_view();
    assert_eq!(app.active_view, ActiveView::Gpu);

    app.cycle_view();
    assert_eq!(app.active_view, ActiveView::Unified);

    app.cycle_view();
    assert_eq!(app.active_view, ActiveView::Swap);
}

#[test]
fn test_app_sort_cycling() {
    let col = SortColumn::Swap;
    let col = col.next();
    assert_eq!(col, SortColumn::GpuMem);
    let col = col.next();
    assert_eq!(col, SortColumn::NumaNode);
    let col = col.next();
    assert_eq!(col, SortColumn::Name);
    let col = col.next();
    assert_eq!(col, SortColumn::Swap);
}

#[test]
fn test_format_mem_boundary_values() {
    use crate::ui::unified_view::format_mem;

    // 0 KB
    let s = format_mem(0, &SizeUnits::KB);
    assert_eq!(s, "0 KB");

    // 1023 KB
    let s = format_mem(1023, &SizeUnits::KB);
    assert_eq!(s, "1023 KB");

    // 1024 KB = 1.00 MB
    let s = format_mem(1024, &SizeUnits::MB);
    assert_eq!(s, "1.00 MB");

    // 1048576 KB = 1.00 GB
    let s = format_mem(1048576, &SizeUnits::GB);
    assert_eq!(s, "1.00 GB");

    // Large value — should not panic
    let s = format_mem(u64::MAX, &SizeUnits::KB);
    assert!(!s.is_empty());

    let s = format_mem(u64::MAX, &SizeUnits::MB);
    assert!(!s.is_empty());

    let s = format_mem(u64::MAX, &SizeUnits::GB);
    assert!(!s.is_empty());
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Part 2: Smoke Tests (Real system, guarded by runtime checks)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[test]
fn test_smoke_proc_swap_info() {
    let provider = ProcDataProvider;
    let result = provider.get_swap_info(&SizeUnits::KB);
    assert!(result.is_ok(), "get_swap_info failed: {:?}", result.err());
    let info = result.unwrap();
    // total_swap should be > 0 on any system with swap configured;
    // if swap is disabled, total_swap == 0 which is still valid
    assert!(info.total_swap >= info.used_swap);
}

#[test]
fn test_smoke_proc_processes_swap() {
    let provider = ProcDataProvider;
    let result = provider.get_processes_swap(&SizeUnits::KB);
    assert!(result.is_ok(), "get_processes_swap failed: {:?}", result.err());
    // May be empty if no processes are using swap — that's OK
}

#[test]
fn test_smoke_numa_available() {
    let provider = ProcDataProvider;
    // Should not panic
    let available = provider.is_numa_available();
    if available {
        let topo = provider.get_numa_topology();
        assert!(topo.is_ok(), "get_numa_topology failed: {:?}", topo.err());
        assert!(
            !topo.unwrap().is_empty(),
            "NUMA available but topology returned 0 nodes"
        );
    }
}

#[test]
fn test_smoke_numa_topology_fields() {
    if !numa_smoke_available() {
        eprintln!("SKIP: NUMA not available on this system");
        return;
    }

    let provider = ProcDataProvider;
    let nodes = provider.get_numa_topology().expect("get_numa_topology failed");

    for node in &nodes {
        assert!(
            node.memory_total_kb > 0,
            "Node {} has 0 memory_total_kb",
            node.id
        );
        match &node.node_type {
            NumaNodeType::Cpu => {
                assert!(
                    !node.cpus.is_empty(),
                    "CPU node {} has empty cpulist",
                    node.id
                );
            }
            NumaNodeType::GpuHbm { .. } => {
                assert!(
                    node.cpus.is_empty(),
                    "GPU HBM node {} should have no CPUs",
                    node.id
                );
            }
            NumaNodeType::Unknown => {
                // Unknown is acceptable (e.g. CXL memory, special devices)
            }
        }
    }
}

#[test]
fn test_smoke_numa_maps_real_process() {
    if !numa_smoke_available() {
        eprintln!("SKIP: NUMA not available on this system");
        return;
    }

    let pid = std::process::id();
    let path = format!("/proc/{}/numa_maps", pid);
    let content = std::fs::read_to_string(&path);
    assert!(content.is_ok(), "Failed to read {}: {:?}", path, content.err());

    let content = content.unwrap();
    let page_size_kb = procfs::page_size() / 1024;
    let info = crate::data::numa::parse_numa_maps(&content, pid, "self", page_size_kb);

    assert!(
        info.total_kb > 0,
        "Our own process should have allocated memory on NUMA nodes"
    );
    assert!(
        !info.kb_per_node.is_empty(),
        "Our own process should have pages on at least one NUMA node"
    );
}

#[test]
fn test_smoke_gpu_available() {
    let provider = ProcDataProvider;
    // Should not panic, just returns true/false
    let _available = provider.is_gpu_available();
}

#[test]
fn test_smoke_gpu_devices() {
    if !gpu_smoke_available() {
        eprintln!("SKIP: nvidia-smi not available");
        return;
    }

    let provider = ProcDataProvider;
    let devices = provider.get_gpu_devices();
    assert!(devices.is_ok(), "get_gpu_devices failed: {:?}", devices.err());
    let devices = devices.unwrap();
    assert!(!devices.is_empty(), "GPU available but 0 devices returned");

    for dev in &devices {
        assert!(!dev.name.is_empty(), "GPU {} has empty name", dev.index);
        assert!(
            dev.memory_total_kb > 0,
            "GPU {} has 0 memory_total_kb",
            dev.index
        );
    }
}

#[test]
fn test_smoke_gpu_processes() {
    if !gpu_smoke_available() {
        eprintln!("SKIP: nvidia-smi not available");
        return;
    }

    let provider = ProcDataProvider;
    let result = provider.get_gpu_processes();
    assert!(result.is_ok(), "get_gpu_processes failed: {:?}", result.err());
    // May be empty if no GPU processes running — that's OK
}

#[test]
fn test_smoke_full_render_cycle() {
    let provider = ProcDataProvider;
    let mut terminal = make_test_terminal();
    let theme = Theme::from(ThemeType::Dracula);

    // Collect all data
    let _swap_info = provider.get_swap_info(&SizeUnits::KB).unwrap_or_default();
    let swap_procs = provider.get_processes_swap(&SizeUnits::KB).unwrap_or_default();
    let gpu_devices = provider.get_gpu_devices().unwrap_or_default();
    let gpu_processes = provider.get_gpu_processes().unwrap_or_default();

    let numa_available = provider.is_numa_available();
    let numa_nodes = if numa_available {
        provider.get_numa_topology().unwrap_or_default()
    } else {
        vec![]
    };

    let numa_infos: Vec<ProcessNumaInfo> = if numa_available {
        swap_procs
            .iter()
            .take(5)
            .filter_map(|p| provider.get_process_numa_maps(p.pid, &p.name).ok())
            .collect()
    } else {
        vec![]
    };

    let unified = merge_process_data(
        &swap_procs,
        &gpu_processes,
        &numa_infos,
        &numa_nodes,
        &gpu_devices,
    );

    // Render NUMA view
    terminal
        .draw(|frame| {
            ui::numa_view::render_numa_view(
                frame,
                frame.area(),
                &theme,
                &numa_nodes,
                &numa_infos,
                numa_available,
                &SizeUnits::KB,
            );
        })
        .unwrap();

    // Render GPU view
    terminal
        .draw(|frame| {
            ui::gpu_view::render_gpu_view(
                frame,
                frame.area(),
                &theme,
                &gpu_devices,
                &gpu_processes,
                provider.is_gpu_available(),
                &SizeUnits::KB,
            );
        })
        .unwrap();

    // Render Unified view
    terminal
        .draw(|frame| {
            ui::unified_view::render_unified_view(
                frame,
                frame.area(),
                &theme,
                &unified,
                &SizeUnits::KB,
                &numa_nodes,
            );
        })
        .unwrap();

    // If we got here, no panics occurred
}
