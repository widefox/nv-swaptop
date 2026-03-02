use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    // Build the clap::Command using the builder API (avoids needing to import
    // the derive struct from the main crate, which is not accessible to build scripts).
    let version: &'static str = env::var("CARGO_PKG_VERSION").unwrap().leak();
    let cmd = clap::Command::new("nv-swaptop")
        .version(version)
        .about("Real-time TUI monitor for swap, NUMA topology, and GPU memory (Linux)")
        .long_about(
            "nv-swaptop is a terminal user interface (TUI) application that monitors swap \
             usage, NUMA memory topology, and GPU memory in real time on Linux systems.\n\n\
             It is designed for systems where GPU HBM is exposed as a NUMA node \
             (e.g. NVIDIA Grace Blackwell GB200), but works on any Linux system with swap \
             configured. GPU features require nvidia-smi in PATH."
        )
        .after_long_help(
r#"KEYBOARD CONTROLS
    Tab          Cycle through views: Swap → NUMA → GPU → Unified
    1/2/3/4      Jump to Swap / NUMA / GPU / Unified view directly
    Esc, q       Quit
    Ctrl-C       Quit
    d, Down      Scroll down
    u, Up        Scroll up
    Home         Scroll to top
    End          Scroll to bottom
    PageDown     Page down
    PageUp       Page up
    k/m/g        Switch units: KB / MB / GB
    a            Toggle process aggregation by name
    t            Cycle colour theme (Default, Solarized, Monokai, Dracula, Nord)
    s            Cycle sort column (swap, gpu_mem, numa, name)
    h            Toggle swap device panel
    Left/Right   Decrease/increase refresh interval (100ms steps, 1ms–10s)

VIEWS
    Swap       Animated swap usage chart, per-process swap list, optional device panel
    NUMA       NUMA node topology and per-process memory distribution across nodes
    GPU        GPU device summary (memory, temperature) and per-GPU process list
    Unified    Combined CPU+GPU+NUMA process table with per-node memory columns

COLOUR CODING (Unified and NUMA views)
    Memory cells are colour-coded by locality:
      green    Local CPU node (process runs on the same NUMA node)
      orange   Remote CPU node (process runs on a different NUMA node)
      red      GPU HBM node (memory on GPU high-bandwidth memory)

DATA SOURCES
    Swap totals        /proc/meminfo
    Per-process swap   /proc/[pid]/status (VmSwap field)
    Swap devices       /proc/swaps
    NUMA topology      /sys/devices/system/node/nodeN/meminfo, cpulist
    NUMA per-process   /proc/[pid]/numa_maps
    CPU→NUMA mapping   /proc/[pid]/stat field 39 mapped via topology
    GPU devices        nvidia-smi --query-gpu (CSV output)
    GPU processes      nvidia-smi --query-compute-apps (CSV output)
    GPU→NUMA mapping   /sys/bus/pci/devices/<pci_bus_id>/numa_node

CACHING
    nv-swaptop uses TTL-based caching to minimise system overhead:
      NUMA topology    30 seconds
      NUMA maps        5 seconds (only when NUMA or Unified view active)
      GPU devices      10 seconds
      GPU processes    1 second

THEMES
    Five built-in colour themes: Default, Solarized, Monokai, Dracula, Nord.
    Cycle with the 't' key.

SUPPORTED ARCHITECTURES
    x86_64, aarch64 (Grace Blackwell, Grace Hopper), ppc64le, riscv64,
    s390x, loongarch64.

ENVIRONMENT
    nvidia-smi must be in PATH for GPU features. Falls back gracefully
    when absent. NUMA features require /sys/devices/system/node/."#
        )
        .arg(
            clap::Arg::new("demo")
                .long("demo")
                .help("Run with synthetic demo data instead of real system data")
                .action(clap::ArgAction::SetTrue),
        );

    // Generate manpage
    let man = clap_mangen::Man::new(cmd);
    let mut buf = Vec::new();
    man.render(&mut buf).expect("Failed to render manpage");
    let manpage_path = out_dir.join("nv-swaptop.1");
    fs::write(&manpage_path, &buf).expect("Failed to write manpage");

    // Expose OUT_DIR so tests can find the generated manpage
    println!("cargo:rustc-env=NV_SWAPTOP_MANPAGE_DIR={}", out_dir.display());
}
