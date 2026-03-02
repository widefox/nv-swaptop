use super::types::*;

use proc_mounts::SwapIter;
use procfs::{self, Current, Meminfo};

pub fn get_swap_devices(unit: SizeUnits) -> std::io::Result<Vec<InfoSwap>> {
    let mut out = Vec::new();
    for swap in SwapIter::new()? {
        let s = swap?;
        out.push(InfoSwap {
            name: s.source.to_string_lossy().into_owned(),
            kind: s.kind.to_string_lossy().into_owned(),
            size_kb: convert_swap(s.size as u64, unit.to_owned()),
            used_kb: convert_swap(s.used as u64, unit.to_owned()),
            priority: s.priority,
        });
    }
    Ok(out)
}

pub fn get_processes_using_swap(unit: SizeUnits) -> Result<Vec<ProcessSwapInfo>, SwapDataError> {
    let mut swap_processes = Vec::new();

    for process in (procfs::process::all_processes()?).flatten() {
        let pid = process.pid;
        if let Ok(status) = process.status()
            && let Some(swap_kb) = status.vmswap
            && swap_kb > 0
        {
            let (name, last_cpu) = match process.stat() {
                Ok(stat) => (stat.comm, stat.processor),
                Err(_) => ("unknown".to_string(), None),
            };
            let swap_size = convert_swap(swap_kb, unit.clone());
            let info = ProcessSwapInfo {
                pid: pid as u32,
                name,
                swap_size,
                last_cpu,
            };
            swap_processes.push(info);
        }
    }

    Ok(swap_processes)
}

pub fn find_mount_device(path: &std::path::Path) -> Option<String> {
    let abs_path = path.canonicalize().ok()?;

    let mountinfo = procfs::process::Process::myself()
        .and_then(|p| p.mountinfo())
        .ok()?;

    let best_mount = mountinfo
        .into_iter()
        .filter(|m| abs_path.starts_with(&m.mount_point))
        .max_by_key(|m| m.mount_point.components().count())?;

    Some(if best_mount.fs_type == "devtmpfs" {
        "RAM".to_owned()
    } else {
        best_mount.mount_source?
    })
}

pub fn get_chart_info(unit: SizeUnits) -> Result<SwapUpdate, SwapDataError> {
    let meminfo = Meminfo::current()?;

    let total_swap_kb = meminfo.swap_total / 1024;
    let used_swap_kb = meminfo.swap_total.saturating_sub(meminfo.swap_free) / 1024;

    Ok(SwapUpdate {
        swap_devices: get_swap_devices(unit)?,
        total_swap: total_swap_kb,
        used_swap: used_swap_kb,
    })
}
