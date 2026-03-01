# nv-swaptop

A real-time terminal monitor for **swap usage**, **NUMA topology**, and **GPU memory** on Linux. Built for systems like NVIDIA Grace Blackwell (GB200) where GPU HBM is exposed as a NUMA node — but works on any system with swap.

![Badge](https://hitscounter.dev/api/hit?url=https%3A%2F%2Fgithub.com%2Fwidefox%2Fnv-swaptop&label=views&icon=github&color=%23cfe2ff)
[![Crates.io](https://img.shields.io/crates/v/nv-swaptop.svg)](https://crates.io/crates/nv-swaptop)

![nv-swaptop demo](docs/swaptop-demo.gif)

> [!TIP]
> Blog post about the original swaptop project: https://blog.wired.rs/posts/swaptop

## Features

### Swap View (Tab 1)
- Real-time animated swap usage graph
- Swap device listing with usage per disk/type
- Per-process swap consumption tracking
- Grouped view by software (aggregate mode)

### NUMA Topology View (Tab 2)
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
- Configurable refresh interval (1ms–10s)
- TTL-based caching for expensive data sources (NUMA topology, nvidia-smi)
- Architectures: x86_64, ARM64, Power, RISC-V, s390x, LoongArch

## Use Cases

**Standard swap monitoring** — Track which processes consume the most swap, identify memory-hungry applications, monitor swap pressure over time with the live chart.

**NVIDIA Grace Blackwell (GB200) / Grace Hopper** — On these systems, GPU HBM is exposed as a NUMA node. nv-swaptop detects HBM NUMA nodes, shows which CPU processes have memory migrated to GPU RAM, and highlights HBM migration in the unified view.

**GPU workstation monitoring** — See GPU VRAM usage alongside swap consumption. Identify processes that are both swapping and using GPU memory. Useful for ML training, rendering, and simulation workloads.

**NUMA-aware debugging** — Understand memory locality of your processes across NUMA nodes. Identify processes with memory spread across multiple nodes (potential performance issue).

## Supported Architectures

| Architecture | Status |
|---|---|
| x86_64 (amd64) | Supported |
| ARM64 (aarch64) | Supported |
| Power (ppc64le) | Supported |
| RISC-V (riscv64) | Supported |
| s390x | Supported |
| LoongArch (loongarch64) | Supported |

## Installation

### From crates.io
```bash
cargo install nv-swaptop
```

### From source
```bash
git clone https://github.com/widefox/nv-swaptop
cd nv-swaptop
cargo build --release
./target/release/nv-swaptop
```

### Prerequisites
- [Rust 1.88.0+](https://rustup.rs/) (Rust 2024 edition)
- Linux kernel 4.4+, procfs mounted at `/proc`
- **GPU features**: `nvidia-smi` in PATH (optional — GPU view degrades gracefully)

## Usage

```bash
nv-swaptop          # interactive mode
nv-swaptop --demo   # auto-cycle all views and quit (for recording)
```

### Keyboard Controls

| Key | Action |
|---|---|
| `Tab` | Cycle through views (Swap → NUMA → GPU → Unified) |
| `1` | Switch to Swap view |
| `2` | Switch to NUMA view |
| `3` | Switch to GPU view |
| `4` | Switch to Unified view |
| `s` | Cycle sort column (swap → gpu_mem → numa → name) |
| `q` / `Esc` | Quit |
| `k` / `m` / `g` | Switch units (KB / MB / GB) |
| `h` | Toggle swap device display (Swap view) |
| `a` | Toggle aggregate mode (group by process name) |
| `t` | Cycle color theme |
| `↑` / `u` | Scroll up |
| `↓` / `d` | Scroll down |
| `Home` | Jump to top |
| `End` | Jump to bottom |
| `PgUp` / `PgDown` | Page up / down |
| `←` / `→` | Decrease / increase refresh interval |
| `Ctrl+C` | Force quit |

### View Cycle

```workflow
┌─→ [Swap (1)] → [NUMA (2)] → [GPU (3)] → [Unified (4)] ─┐
└────────────────────────────────────────────────────────┘
```

## Example Output

The examples below show two hardware scenarios. The Swap View is hardware-independent; the remaining views adapt to the system's NUMA topology and GPU configuration.

### Swap View
```text
╭─ nv-swaptop [Swap] sort:swap ── < 1000ms >  Tab/1-4:view  s:sort ── theme (t): Dracula ────╮
│  ┌ Swap Usage ──────────────────────────┐                                                  │
│  │ Total: 8388608 KB  Used: 1245184 KB  │                                                  │
│  │ ██████████░░░░░░░░░░░░░ 14.8%        │                                                  │
│  └──────────────────────────────────────┘                                                  │
│  ┌ Processes Using Swap ───────────────────────────────────────────┐                       │
│  │     PID  NAME                    SWAP                           │                       │
│  │   12045  firefox              524288 KB                         │                       │
│  │    8923  code                 312456 KB                         │                       │
│  │    3456  chrome               204800 KB                         │                       │
│  └─────────────────────────────────────────────────────────────────┘                       │
╰────────────────────────────────────────────────────────────────────────────────────────────╯
```

### x86_64 — AMD EPYC Dual-Socket + 4× NVIDIA A100 80GB

Two CPU NUMA nodes (128 GB each, 32 cores each), four discrete GPUs (VRAM is **not** a NUMA node).

#### NUMA Topology View
```text
╭ NUMA Topology ─────────────────────────────────────────────────────────╮
│  NODE │ TYPE       │  MEM TOTAL │  MEM USED │ CPUs                     │
│     0 │ CPU        │  128.00 GB │  63.68 GB │ 0-31                     │
│     1 │ CPU        │  128.00 GB │  69.29 GB │ 32-63                    │
╰────────────────────────────────────────────────────────────────────────╯
╭ Per-Process NUMA Distribution (top 20 swap consumers) ─────────────────╮
│      PID │ PROCESS              │ CPU │    TOTAL │      N0 │     N1    │
│    12045 │ firefox              │  0* │ 59.49 MB │ 26.84 MB │32.66 MB  │
│     8923 │ code                 │   1 │ 38.44 MB │  2.50 MB │35.94 MB  │
│     3456 │ chrome               │   0 │ 18.75 MB │ 18.75 MB │     -    │
│  * = amber: CPU node ≠ dominant memory node                            │
╰────────────────────────────────────────────────────────────────────────╯
```

#### GPU View
```text
╭ GPU Devices ───────────────────────────────────────────────────────────╮
│  #0  NVIDIA A100-SXM4-80GB    80.00 GB total  42.31 GB used  65°C      │
│  #1  NVIDIA A100-SXM4-80GB    80.00 GB total  12.80 GB used  58°C      │
│  #2  NVIDIA A100-SXM4-80GB    80.00 GB total  76.20 GB used  71°C      │
│  #3  NVIDIA A100-SXM4-80GB    80.00 GB total   0.50 GB used  41°C      │
╰────────────────────────────────────────────────────────────────────────╯
╭ GPU Processes ─────────────────────────────────────────────────────────╮
│     PID  NAME                 GPU#     VRAM USED                       │
│   15678  python3                 0     38.20 GB                        │
│   15690  python3                 1     12.80 GB                        │
│   15701  python3                 2     76.20 GB                        │
│    9012  Xorg                    0      4.11 GB                        │
╰────────────────────────────────────────────────────────────────────────╯
```

#### Unified CPU+GPU+NUMA View
```text
╭ Unified CPU+GPU+NUMA View ──────────────────── (orange = HBM migration) ───────────╮
│      PID  NAME             CPU→N GPU→N        N0        N1       SWAP   GPU MEM    │
│    15678  python3          0     0        8.20 GB   4.10 GB    128 MB  38.20 GB    │
│    12045  firefox          0*    -        6.70 GB   8.20 GB    524 MB     -        │
│    15701  python3          1     2          -       2.05 GB      -     76.20 GB    │
│     9012  Xorg             0     0          -         -          -      4.11 GB    │
╰────────────────────────────────────────────────────────────────────────────────────╯
```

### aarch64 — 2× NVIDIA Grace Blackwell (GB200)

Each GB200 superchip pairs one Grace CPU with two B200 GPUs. Two superchips give 2 Grace CPUs (480 GB LPDDR5X each, 72 cores each) + 4 B200 GPUs (192 GB HBM3e each). GPU HBM is exposed as NUMA nodes N2–N5.

#### NUMA Topology View
```text
╭ NUMA Topology ───────────────────────────────────────────────────────────────────────────╮
│  NODE │ TYPE       │  MEM TOTAL │  MEM USED │ CPUs                                       │
│     0 │ CPU        │  480.00 GB │ 210.50 GB │ 0-71                                       │
│     1 │ CPU        │  480.00 GB │ 185.30 GB │ 72-143                                     │
│     2 │ GPU HBM 0  │  192.00 GB │  96.00 GB │ -                                          │
│     3 │ GPU HBM 1  │  192.00 GB │  48.00 GB │ -                                          │
│     4 │ GPU HBM 2  │  192.00 GB │ 180.50 GB │ -                                          │
│     5 │ GPU HBM 3  │  192.00 GB │  12.00 GB │ -                                          │
╰──────────────────────────────────────────────────────────────────────────────────────────╯
╭ Per-Process NUMA Distribution (top 20 swap consumers) ───────────────────────────────────────────────────────╮
│      PID │ PROCESS              │ CPU │      TOTAL │     N0 │     N1 │ N2(HBM) │ N3(HBM) │ N4(HBM) │ N5(HBM) │
│    20001 │ training_job         │  0* │  324.50 GB │ 4.5 GB │   -    │ 96.0 GB │ 48.0 GB │ 176.0 GB│    -    │
│    20045 │ inference_srv        │  72 │   60.50 GB │   -    │ 0.5 GB │    -    │    -    │    -    │ 12.0 GB │
│    18200 │ data_loader          │   0 │    8.20 GB │ 6.2 GB │ 2.0 GB │    -    │    -    │    -    │    -    │
│  * = amber: CPU node ≠ dominant memory node                                                                  │
╰──────────────────────────────────────────────────────────────────────────────────────────────────────────────╯
```

#### GPU View
```text
╭ GPU Devices ───────────────────────────────────────────────────────────╮
│  #0  NVIDIA B200            192.00 GB total  96.00 GB used  62°C       │
│  #1  NVIDIA B200            192.00 GB total  48.00 GB used  55°C       │
│  #2  NVIDIA B200            192.00 GB total 180.50 GB used  73°C       │
│  #3  NVIDIA B200            192.00 GB total  12.00 GB used  44°C       │
╰────────────────────────────────────────────────────────────────────────╯
╭ GPU Processes ─────────────────────────────────────────────────────────╮
│     PID  NAME                 GPU#     VRAM USED                       │
│   20001  training_job            0     96.00 GB                        │
│   20001  training_job            2    176.00 GB                        │
│   20045  inference_srv           3     12.00 GB                        │
╰────────────────────────────────────────────────────────────────────────╯
```

#### Unified CPU+GPU+NUMA View
```text
╭ Unified CPU+GPU+NUMA View ───────────────────────────────────────────────────────────────────── (orange = HBM migration) ╮
│      PID  NAME             CPU→N GPU→N        N0        N1   N2(HBM)   N3(HBM)   N4(HBM)   N5(HBM)       SWAP   GPU MEM  │
│    20001  training_job     0*    0,2      4.50 GB      -     96.00 GB  48.00 GB 176.00 GB      -       512 MB 272.00 GB  │
│    20045  inference_srv    72    3           -      0.50 GB      -         -         -     12.00 GB      -     12.00 GB  │
│    18200  data_loader      0     -        6.20 GB  2.00 GB      -         -         -         -       2.05 GB     -      │
╰──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────╯
```

## Themes

Cycle through 5 themes with `t`:
1. Default
2. Solarized
3. Monokai
4. Dracula
5. Nord

![Swaptop theme Demo](docs/theme-demo.gif)

> [!NOTE]
> Theme demo shows an earlier version. Current version includes additional columns and views not shown here.

## Architecture

```tree
src/
├── main.rs              # Thin entry point
├── app.rs               # App struct, event loop, state, key handling, caching
├── theme.rs             # Color theme definitions
├── data/
│   ├── mod.rs           # DataProvider trait, ProcDataProvider, merge_process_data()
│   ├── types.rs         # All shared types and pure functions
│   ├── swap.rs          # Swap data collection
│   ├── numa.rs          # NUMA topology parsing
│   └── gpu.rs           # nvidia-smi CSV parsing
└── ui/
    ├── mod.rs           # UI module re-exports
    ├── chart.rs         # Animated swap usage chart
    ├── process_list.rs  # Process list with scrolling
    ├── swap_devices.rs  # Swap device table
    ├── numa_view.rs     # NUMA topology + per-process distribution
    ├── gpu_view.rs      # GPU device summary + process list
    └── unified_view.rs  # Combined CPU+GPU+NUMA process table
```

All data collection is behind a `DataProvider` trait, enabling mock-based testing. Parsing functions are pure (`&str -> T`) for full testability without real hardware. 80 unit tests cover type conversions, aggregation, NUMA/GPU parsing, page-size-aware numa_maps parsing, CPU-to-NUMA mapping, process merging, and demo mode scheduling.

## Technical Details

### Data Sources
- **Swap**: `/proc/meminfo`, `/proc/[pid]/status` via `procfs` crate
- **Swap devices**: `/proc/swaps` via `proc-mounts` crate
- **NUMA**: `/sys/devices/system/node/nodeN/meminfo`, `/sys/devices/system/node/nodeN/cpulist`, `/proc/[pid]/numa_maps`
- **GPU**: `nvidia-smi` (from PATH) `--query-compute-apps` and `--query-gpu` CSV output
- **GPU-NUMA mapping**: `/sys/bus/pci/devices/<pci_bus_id>/numa_node`

### Page Size Handling
NUMA memory values are parsed from `/proc/[pid]/numa_maps`, where each mapping line reports page counts that may use a different page size. nv-swaptop handles this correctly:

- **Architecture-dependent base pages**: x86_64 uses 4 KB pages, aarch64 (NVIDIA Grace) uses 64 KB. Detected at runtime via `procfs::page_size()`.
- **Per-line `kernelpagesize_kB`**: Each mapping line in `numa_maps` can specify its own page size (e.g., `kernelpagesize_kB=2048` for THP, `kernelpagesize_kB=1048576` for 1 GB hugetlb pages). When present, this overrides the default for that line.
- **Mixed page sizes**: A single process can have 4 KB, 2 MB (THP), and 1 GB (hugetlb) pages simultaneously. Page counts are multiplied by their per-line page size and accumulated as KB at parse time, since raw page counts across different page sizes are not comparable.

### Caching
| Data Source | TTL | Notes |
|---|---|---|
| NUMA topology | 30s | Topology rarely changes |
| NUMA maps | 5s | Only refreshed when NUMA or Unified view is active, top 20 processes |
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
