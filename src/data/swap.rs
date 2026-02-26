use super::types::*;

#[cfg(target_os = "linux")]
use proc_mounts::SwapIter;
#[cfg(target_os = "linux")]
use procfs::{self, Current, Meminfo};

#[cfg(target_os = "linux")]
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

#[cfg(target_os = "linux")]
pub fn get_processes_using_swap(unit: SizeUnits) -> Result<Vec<ProcessSwapInfo>, SwapDataError> {
    let mut swap_processes = Vec::new();

    for process in (procfs::process::all_processes()?).flatten() {
        let pid = process.pid;
        if let Ok(status) = process.status()
            && let Some(swap_kb) = status.vmswap
            && swap_kb > 0
        {
            let name = match process.stat() {
                Ok(stat) => stat.comm,
                Err(_) => "unknown".to_string(),
            };
            let swap_size = convert_swap(swap_kb, unit.clone());
            let info = ProcessSwapInfo {
                pid: pid as u32,
                name,
                swap_size,
            };
            swap_processes.push(info);
        }
    }

    Ok(swap_processes)
}

#[cfg(target_os = "linux")]
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

#[cfg(target_os = "windows")]
pub fn get_processes_using_swap(unit: SizeUnits) -> Result<Vec<ProcessSwapInfo>, SwapDataError> {
    let mut profile_page_processes = Vec::new();

    if let Ok(tasks) = tasklist::Tasklist::new() {
        for task in tasks {
            let meminfo = task.get_memory_info();

            let info = ProcessSwapInfo {
                pid: task.pid,
                name: task.pname,
                swap_size: convert_swap(meminfo.get_pagefile_usage() as u64 / 1024, unit.clone()),
            };
            profile_page_processes.push(info);
        }
    }

    Ok(profile_page_processes)
}

#[cfg(target_os = "linux")]
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

#[cfg(target_os = "windows")]
pub fn get_chart_info() -> Result<SwapUpdate, SwapDataError> {
    use std::mem::MaybeUninit;
    use winapi::um::sysinfoapi::{GlobalMemoryStatusEx, MEMORYSTATUSEX};

    unsafe {
        let mut mem_status = MaybeUninit::<MEMORYSTATUSEX>::zeroed();
        mem_status.as_mut_ptr().write(MEMORYSTATUSEX {
            dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
            ..Default::default()
        });

        if GlobalMemoryStatusEx(mem_status.as_mut_ptr()) == 0 {
            return Err(SwapDataError::Io(std::io::Error::last_os_error()));
        }

        let mem_status = mem_status.assume_init();

        let total_swap = mem_status.ullTotalPageFile / 1024;
        let used_swap = (mem_status.ullTotalPageFile - mem_status.ullAvailPageFile) / 1024;

        Ok(SwapUpdate {
            total_swap,
            used_swap,
        })
    }
}
