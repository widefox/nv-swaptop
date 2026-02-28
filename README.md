# nv-swaptop

A real-time terminal monitor for **swap usage**, **NUMA topology**, and **GPU memory** on Linux and Windows. Built for systems like NVIDIA Grace Blackwell (GB200) where GPU HBM is exposed as a NUMA node — but works on any system with swap.

![Badge](https://hitscounter.dev/api/hit?url=https%3A%2F%2Fgithub.com%2Fluis-ota%2Fswaptop&label=views&icon=github&color=%23cfe2ff)
[![Crates.io](https://img.shields.io/crates/v/nv-swaptop.svg)](https://crates.io/crates/nv-swaptop)

![Swaptop Demo (original version)](docs/swaptop.gif)

> [!TIP]
> Blog post about the original swaptop project: https://blog.wired.rs/posts/swaptop

## Features

### Swap View (Tab 1)
- Real-time animated swap usage graph
- Swap device listing with usage per disk/type (Linux)
- Per-process swap consumption tracking
- Grouped view by software (aggregate mode)

### NUMA Topology View (Tab 2 — Linux only)
- Discover all NUMA nodes and classify as CPU, GPU HBM, or Unknown
- Per-node memory totals and usage
- CPU list per node
- Per-process NUMA memory distribution (top 20 swap consumers)
- CPU NODE column shows which NUMA node each process is executing on
- Amber highlighting when CPU node differs from dominant memory node (NUMA misalignment)
- Detects GPU HBM NUMA nodes on NVIDIA Grace Blackwell systems

### GPU View (Tab 3)
- GPU device summary: name, memory total/used/free, temperature, PCI bus ID
- GPU process list: PID, name, GPU index, VRAM used
- Parses `nvidia-smi` CSV output (no NVML dependency required)
- Graceful fallback when no NVIDIA GPU is detected

### Unified CPU+GPU+NUMA View (Tab 4)
- Combined process table: PID | NAME | SWAP | GPU MEM | NUMA | LOCATION
- Processes classified as CPU-only, GPU-only, or CPU+GPU
- Color-coded location column (orange = HBM migration detected)
- Sortable by swap, GPU memory, NUMA node, or name

### General
- Multiple color themes (Default, Solarized, Monokai, Dracula, Nord)
- Unit conversion (KB/MB/GB)
- Configurable refresh interval (100ms–10s)
- TTL-based caching for expensive data sources (NUMA topology, nvidia-smi)
- Cross-platform: Linux and Windows

## Use Cases

**Standard swap monitoring** — Track which processes consume the most swap, identify memory-hungry applications, monitor swap pressure over time with the live chart.

**NVIDIA Grace Blackwell (GB200) / Grace Hopper** — On these systems, GPU HBM is exposed as a NUMA node. nv-swaptop detects HBM NUMA nodes, shows which CPU processes have memory migrated to GPU RAM, and highlights HBM migration in the unified view.

**GPU workstation monitoring** — See GPU VRAM usage alongside swap consumption. Identify processes that are both swapping and using GPU memory. Useful for ML training, rendering, and simulation workloads.

**NUMA-aware debugging** — Understand memory locality of your processes across NUMA nodes. Identify processes with memory spread across multiple nodes (potential performance issue).

## Installation

### From crates.io
```bash
cargo install nv-swaptop
```

### From source
```bash
git clone https://github.com/luis-ota/swaptop
cd swaptop
cargo build --release
./target/release/nv-swaptop
```

### Prerequisites
- [Rust 1.88.0+](https://rustup.rs/) (Rust 2024 edition)
- **Linux**: kernel 4.4+, procfs mounted at `/proc`
- **GPU features**: `nvidia-smi` in PATH (optional — GPU view degrades gracefully)
- **Windows**: download from [releases](https://github.com/luis-ota/swaptop/releases)

## Usage

```bash
nv-swaptop
```

### Keyboard Controls

| Key | Action |
|---|---|
| `Tab` | Cycle through views (Swap → NUMA → GPU → Unified) |
| `1` | Switch to Swap view |
| `2` | Switch to NUMA view (Linux only) |
| `3` | Switch to GPU view |
| `4` | Switch to Unified view |
| `s` | Cycle sort column (swap → gpu_mem → numa → name) |
| `q` / `Esc` | Quit |
| `k` / `m` / `g` | Switch units (KB / MB / GB) |
| `h` | Toggle swap device display (Linux, Swap view) |
| `a` | Toggle aggregate mode (group by process name) |
| `t` | Cycle color theme |
| `↑` / `u` | Scroll up |
| `↓` / `d` | Scroll down |
| `Home` | Jump to top |
| `End` | Jump to bottom |
| `PgUp` / `PgDown` | Page up / down |
| `←` / `→` | Decrease / increase refresh interval |
| `Ctrl+C` | Force quit |

## Example Output

### Swap View
```text
╭─ nv-swaptop [Swap] sort:swap ── < 1000ms >  Tab/1-4:view  s:sort ── theme (t): Dracula ─╮
│  ┌ Swap Usage ──────────────────────────┐                                                  │
│  │ Total: 8388608 KB  Used: 1245184 KB  │                                                  │
│  │ ██████████░░░░░░░░░░░░░ 14.8%        │                                                  │
│  └──────────────────────────────────────-┘                                                  │
│  ┌ Processes Using Swap ────────────────────────────────────────────┐                       │
│  │     PID  NAME                    SWAP                           │                       │
│  │   12045  firefox              524288 KB                         │                       │
│  │    8923  code                 312456 KB                         │                       │
│  │    3456  chrome               204800 KB                         │                       │
│  └─────────────────────────────────────────────────────────────────┘                       │
╰───────────────────────────────────────────────────────────────────────────────────────────-─╯
```

### NUMA Topology View (Linux)
```text
╭ NUMA Topology ─────────────────────────────────────────╮
│  NODE  TYPE       MEM TOTAL    MEM FREE    CPUS        │
│     0  CPU        128.00 GB    64.32 GB    0-31        │
│     1  CPU        128.00 GB    58.71 GB    32-63       │
│     2  GPU HBM     96.00 GB    82.45 GB    (none)      │
│                                                        │
│  Per-Process NUMA Distribution (top 20):               │
│     PID  NAME               CPU  TOTAL PG  N0    N1    │
│   12045  firefox              1*    15230  6870  8360  │
│    8923  training_job         0      9840  9200   640  │
│  * = amber: CPU node ≠ dominant memory node            │
╰────────────────────────────────────────────────────────╯
```

### GPU View
```text
╭ GPU Devices ───────────────────────────────────────────────────────╮
│  #0  NVIDIA B200             80.00 GB total  42.31 GB used  65°C  │
│  #1  NVIDIA B200             80.00 GB total  12.80 GB used  58°C  │
╰────────────────────────────────────────────────────────────────────╯
╭ GPU Processes ─────────────────────────────────────────────────────╮
│     PID  NAME                 GPU#     VRAM USED                  │
│   15678  python3                 0     38.20 GB                   │
│   15690  python3                 1     12.80 GB                   │
│    9012  Xorg                    0      4.11 GB                   │
╰────────────────────────────────────────────────────────────────────╯
```

### Unified CPU+GPU+NUMA View
```text
╭ Unified CPU+GPU+NUMA View ──────── (orange = HBM migration detected) ─╮
│      PID  NAME                      SWAP      GPU MEM    NUMA  LOCATION│
│    15678  python3                512.00 MB    38.20 GB      0   CPU+GPU│
│    12045  firefox                524288 KB           -      0   CPU    │
│    15690  python3                  0.00 MB    12.80 GB      1   GPU    │
│     8923  training_job           128.00 MB     4.00 GB      2   CPU+GPU│
╰────────────────────────────────────────────────────────────────────────╯
```

## Themes

Cycle through 5 themes with `t`:
1. Default
2. Solarized
3. Monokai
4. Dracula
5. Nord

![Swaptop theme Demo](docs/theme-demo.gif)

## Architecture

```tree
src/
  main.rs              # Thin entry point
  app.rs               # App struct, event loop, state, key handling, caching
  theme.rs             # Color theme definitions
  data/
    mod.rs             # DataProvider trait, ProcDataProvider, merge_process_data()
    types.rs           # All shared types and pure functions
    swap.rs            # Swap data collection (Linux/Windows)
    numa.rs            # NUMA topology parsing (Linux only)
    gpu.rs             # nvidia-smi CSV parsing
  ui/
    mod.rs             # UI module re-exports
    chart.rs           # Animated swap usage chart
    process_list.rs    # Process list with scrolling
    swap_devices.rs    # Swap device table (Linux only)
    numa_view.rs       # NUMA topology + per-process distribution (Linux only)
    gpu_view.rs        # GPU device summary + process list
    unified_view.rs    # Combined CPU+GPU+NUMA process table
```

All data collection is behind a `DataProvider` trait, enabling mock-based testing. Parsing functions are pure (`&str -> T`) for full testability without real hardware. 44 unit tests cover type conversions, aggregation, NUMA/GPU parsing, CPU-to-NUMA mapping, and process merging.

## Technical Details

### Data Sources
- **Swap**: `/proc/meminfo`, `/proc/[pid]/status` (Linux); `winapi` `GlobalMemoryStatusEx` + `tasklist` crate (Windows)
- **Swap devices**: `/proc/swaps` via `proc-mounts` crate (Linux only)
- **NUMA**: `/sys/devices/system/node/nodeN/meminfo`, `/sys/devices/system/node/nodeN/cpulist`, `/proc/[pid]/numa_maps` (Linux only)
- **GPU**: `nvidia-smi` (from PATH) `--query-compute-apps` and `--query-gpu` CSV output
- **GPU-NUMA mapping**: `/sys/bus/pci/devices/<pci_bus_id>/numa_node` (Linux only)

### Caching
| Data Source | TTL | Notes |
|---|---|---|
| NUMA topology | 30s | Topology rarely changes |
| NUMA maps | 5s | Only refreshed when NUMA view is active, top 20 processes |
| GPU devices | 10s | Device info changes rarely |
| GPU processes | 1s | Process list changes frequently |

### Performance
- Updates every 1 second by default (configurable 100ms–10s)
- <1% CPU usage on modern systems
- Memory footprint: ~4MB

## Troubleshooting

**No swap data showing?**
```bash
swapon --show
```

**Permission issues?**
```bash
sudo -E nv-swaptop
```

**No GPU data?**
Ensure `nvidia-smi` is in your PATH:
```bash
nvidia-smi
```
The GPU view will show "No NVIDIA GPU detected" if nvidia-smi is unavailable — the rest of the application works normally.

**NUMA view shows no nodes?**
NUMA topology requires `/sys/devices/system/node/` to be present. Verify:
```bash
ls /sys/devices/system/node/
```

## Contributors

<table>
<tr>
  <td align="center" style="word-wrap: break-word; width: 150.0; height: 150.0">
        <a href=https://github.com/luis-ota>
            <img src=https://avatars.githubusercontent.com/u/76752591?v=4 width="100" style="border-radius:50%;align-items:center;justify-content:center;overflow:hidden;padding-top:10px" alt="luis-ota"/>
            <br />
            <sub style="font-size:14px"><b>luis</b></sub>
        </a>
    </td>
    <td align="center" style="word-wrap: break-word; width: 150.0; height: 150.0">
        <a href=https://github.com/kpcyrd>
            <img src=https://avatars.githubusercontent.com/u/7763184?v=4 width="100" style="border-radius:50%;align-items:center;justify-content:center;overflow:hidden;padding-top:10px" alt="kpcyrd"/>
            <br />
            <sub style="font-size:14px"><b>kpcyrd</b></sub>
        </a>
    </td>
    <td align="center" style="word-wrap: break-word; width: 150.0; height: 150.0">
        <a href=https://github.com/RafaelKC>
            <img src=https://avatars.githubusercontent.com/u/72219344?v=4 width="100" style="border-radius:50%;align-items:center;justify-content:center;overflow:hidden;padding-top:10px" alt="RafaelKC"/>
            <br />
            <sub style="font-size:14px"><b>Rafael Chicovis</b></sub>
        </a>
    </td>
    <td align="center" style="word-wrap: break-word; width: 150.0; height: 150.0">
        <a href=https://github.com/widefox>
            <img src=https://avatars.githubusercontent.com/u/819378?v=4 width="100" style="border-radius:50%;align-items:center;justify-content:center;overflow:hidden;padding-top:10px" alt="widefox"/>
            <br />
            <sub style="font-size:14px"><b>widefox</b></sub>
        </a>
    </td>
</tr>
</table>

## License

MIT License - see [LICENSE](LICENSE) for details.

---

no matter where you are everyone is always connected

⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⣀⡤⢤⣤⣤⣄⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢀⣶⣧⣐⠍⢙⣀⣼⣿⣿⣅⡐⠆⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢀⡀⠀⠉⠙⠻⣿⣿⣿⣿⣿⣯⣄⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠠⣶⠆⠀⢀⣺⡃⣀⠀⠀⠀⠈⢿⣿⣿⣿⣿⡿⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⣸⣷⣼⠆⠀⡌⢹⣿⣿⠀⢄⠀⠀⠈⣿⣿⣿⣿⠃⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢀⣿⣿⣿⠀⠸⣷⣿⣿⣿⣆⣠⣿⡄⠀⣼⣿⣿⣿⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢸⣿⣿⣿⠂⠀⠹⡿⣿⣿⣿⣿⣿⠀⠀⠟⠈⠏⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⣿⣿⣿⠏⢰⣿⣦⡀⠚⠛⢿⣿⡿⠀⠀⢸⠇⢀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢠⢿⣿⣿⠀⢾⣿⣿⡇⢻⡏⠀⠀⠀⠀⠀⡆⢰⣿⣿⡗⢠⣿⣿⣷⣦⣤⣤⣀⣤⣀⣀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠈⠙⠒⠿⠿⣿⣿⠸⠇⠀⠀⠀⠀⠀⣷⠘⣿⠟⣠⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣷⢆⣠⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢸⣧⠀⠀⠀⠀⠀⠀⣿⠀⣃⣾⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⡿⠏⠘⠉⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢸⣧⠀⠀⠀⠀⠀⠀⢄⣸⣿⣿⣿⣿⣭⣭⡉⠉⠉⠉⠈⠉⠀⠉⠉⠁⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢸⠛⢸⡄⠀⠀⠀⠀⢸⣿⣿⣿⣿⣿⣿⣿⣧⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢸⣶⢸⣷⠀⠀⠀⠀⠈⣇⣽⣿⣿⣿⣿⣿⣿⡆⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠸⠛⠈⠉⠀⠀⠀⠀⠀⢸⣿⣿⣿⣿⣿⣿⣿⠇⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠁⢷⡆⠀⠀⠀⠀⠀⠀⠈⣄⣼⣿⣿⣿⣿⠃⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠈⠃⠀⠀⠀⠀⠀⠀⠀⢹⠛⢿⣿⠟⠁⣀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠚⠀⣴⡀⢰⡆⢀⣤⣄⣒⡉⠀⣶⣀⣎⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⣰⣿⣿⡟⠀⣼⡿⣷⢾⠇⢠⣿⣿⣿⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢀⣉⠛⠛⠁⢴⣽⣷⣧⣼⡄⠸⣽⡿⠟⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢀⣼⣿⣿⣿⣯⣿⣿⣿⣿⣿⣷⡶⣦⣴⡦⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⣸⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⡯⢽⢿⡇⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢠⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣷⣿⡇⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢀⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⡇⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⣾⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⢿⣿⡇⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⣸⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣰⣿⡇⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠴⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣷⢾⣿⡇⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠐⠀⠀⠉⠙⠛⠻⠿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣏⣾⣿⡇⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠈⠉⠙⠛⠻⠿⢿⣿⣿⣿⣿⡇⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠈⢰⣶⠀⠇⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠈⠉⠙⠃⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢸⣿⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠘⠋⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⣼⣿⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠘⠛⠋⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
