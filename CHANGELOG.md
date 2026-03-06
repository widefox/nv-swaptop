# Changelog

All notable changes to nv-swaptop are documented in this file.

## [2.0.1] - 2026-03-02

### Added
- Manpage generation at build time via `clap_mangen`
- `clap` CLI with `--demo`, `--help`, `--version` flags
- NUMA memory locality colour coding (green=local, orange=remote, red=GPU HBM)
- Integration and smoke test suite (25 tests, 127 total)
- `aarch64-unknown-linux-musl` build target for 64KB page systems (Grace Blackwell)
- Makefile with `make install` and `make uninstall` targets

### Fixed
- GB200 topology: 2 GPUs per Grace CPU (1:2 ratio per superchip)
- Parse `kernelpagesize_kB` per line in `numa_maps` (correct handling of mixed page sizes)

### Changed
- Enhanced unified view with multi-GPU and per-NUMA-node columns

### Removed
- All Windows support (`#[cfg(target_os = "windows")]` blocks, cross-compile targets)

## [2.0.0] - 2026-03-01

### Added
- Complete rewrite: modular architecture with `DataProvider` trait and pure parsing functions
- NUMA topology view with per-process memory distribution
- GPU view with `nvidia-smi` CSV parsing (no NVML dependency)
- Unified CPU+GPU+NUMA process table with HBM migration detection
- CPU NODE column to NUMA view with misalignment highlighting
- `--demo` mode for automated view cycling and screenshot recording
- 5 colour themes (Default, Solarized, Monokai, Dracula, Nord)
- TTL-based caching for expensive data sources
- 127 unit tests covering all parsing and data merging
- Cross-compilation for ARM64, Power, RISC-V, s390x, LoongArch

### Removed
- Windows support (Linux-only from v2.0.0)

## [1.0.4] - 2025-11-27

### Added
- Swap device listing view (toggle with `h`)
- PgUp/PgDown and Home/End keyboard shortcuts

### Fixed
- Display render when swap devices panel not shown

## [1.0.2] - 2025-09-12

### Fixed
- Use `target_os = "linux"` instead of `feature = "linux"` for platform gating

## [1.0.0] - 2025-05-11

### Added
- Initial release
- Real-time swap usage monitoring with animated chart
- Per-process swap consumption tracking
- Aggregate mode (group by process name)
- Unit conversion (KB/MB/GB)
- Configurable refresh interval
- Scrollable process list
- Multiple colour themes
