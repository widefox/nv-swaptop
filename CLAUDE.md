# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

nv-swaptop is a real-time TUI (terminal user interface) monitor for swap usage, NUMA topology, and GPU memory on Linux and Windows. Built with Rust 2024 edition using ratatui for the terminal UI. Designed for systems like NVIDIA Grace Blackwell (GB200) where GPU HBM is exposed as a NUMA node, but works on any system with swap.

## Build and Development Commands

```bash
# Build
cargo build
cargo build --release

# Run
cargo run
./target/release/nv-swaptop

# Run all tests (66 unit tests)
cargo test

# Run a single test
cargo test test_parse_gpu_processes
cargo test --lib data::gpu::tests::test_parse_gpu_processes

# Run tests for a specific module
cargo test --lib data::numa
cargo test --lib data::gpu
cargo test --lib data::types
cargo test --lib data::tests      # merge_process_data tests

# Check without building
cargo check

# Lint
cargo clippy
```

Requires Rust 1.88.0+ (edition 2024). GPU features require `nvidia-smi` in PATH (graceful fallback when absent). NUMA features require `/sys/devices/system/node/` (Linux only).

### Runtime Data Sources and Paths

Linux:
- **Swap**: `/proc/meminfo` (totals), `/proc/[pid]/status` (`VmSwap` field) via `procfs` crate
- **Swap devices**: `/proc/swaps` via `proc-mounts` crate
- **NUMA topology**: `/sys/devices/system/node/nodeN/meminfo`, `/sys/devices/system/node/nodeN/cpulist`
- **NUMA per-process**: `/proc/[pid]/numa_maps`
- **CPU-NUMA mapping**: `/proc/[pid]/stat` field 39 (`processor`) mapped to NUMA node via topology
- **NUMA availability**: checks `/sys/devices/system/node/node0` exists
- **GPU**: `nvidia-smi` (resolved from PATH, no hardcoded path)
- **GPU-NUMA mapping**: `/sys/bus/pci/devices/<pci_bus_id>/numa_node`

Windows:
- **Swap**: `winapi` `GlobalMemoryStatusEx` (totals), `tasklist` crate (per-process pagefile)
- **GPU**: same `nvidia-smi` from PATH

## Architecture

### Data Flow

`main.rs` creates an `App` with a `Box<dyn DataProvider>` and runs the event loop. The `DataProvider` trait abstracts all system data collection, enabling a `MockDataProvider` for tests.

### Platform Separation

Heavy use of `#[cfg(target_os = "linux")]` / `#[cfg(target_os = "windows")]` throughout. Key differences:
- Linux: has NUMA view (Tab 2), swap device listing, uses `procfs`/`proc-mounts` crates
- Windows: uses `winapi`/`tasklist`/`sysinfo` crates, no NUMA support
- Both: swap view, GPU view, unified view

Functions that differ by platform are implemented as separate `#[cfg(...)]` blocks rather than runtime detection. This includes `App::run()`, `App::render()`, `App::on_key_event()`, `merge_process_data()`, and the `SwapDataError` enum.

### Module Organization

```tree
src/
  main.rs              # Thin entry point: creates App with ProcDataProvider, runs event loop
  app.rs               # Event loop, state, TTL-based caching, key handling, view rendering dispatch
  theme.rs             # 5 color themes (Default, Solarized, Monokai, Dracula, Nord)
  data/
    mod.rs             # DataProvider trait, ProcDataProvider, MockDataProvider, merge_process_data()
    types.rs           # All shared types and pure utility functions (convert_swap, aggregate_processes)
    swap.rs            # Swap data from /proc/meminfo (Linux) or sysinfo (Windows)
    numa.rs            # Pure NUMA parsing (meminfo, cpulist, numa_maps); sysfs topology discovery
    gpu.rs             # nvidia-smi CSV parsing; all parsing is pure &str -> T for testability
  ui/
    mod.rs             # UI module re-exports
    chart.rs           # Animated swap usage chart
    process_list.rs    # Process list with scrolling
    swap_devices.rs    # Swap device table (Linux only)
    numa_view.rs       # NUMA topology + per-process distribution (Linux only)
    gpu_view.rs        # GPU device summary + process list
    unified_view.rs    # Combined CPU+GPU+NUMA process table
```

### Key Design Patterns

- **Pure parsing functions** — GPU and NUMA parsers take `&str` input and return typed data, no I/O. This is how the 44 tests work without real hardware.
- **TTL caching** — `App` caches expensive data with different TTLs: NUMA topology (30s), NUMA maps (5s, only when NUMA view active), GPU devices (10s), GPU processes (1s).
- **Lazy refresh** — NUMA maps only refresh when the NUMA tab is active. GPU data only refreshes when GPU or Unified tab is active.
- **Unified view merge** — `merge_process_data()` joins swap, GPU, and NUMA data by PID. Detects HBM migration (CPU process with pages on a GPU HBM NUMA node).

### CI/CD

GitHub Actions workflow (`.github/workflows/release.yml`) triggers on `v*.*.*` tags. Cross-compiles for Linux (native) and Windows (MinGW). Creates GitHub release with archived binaries.
