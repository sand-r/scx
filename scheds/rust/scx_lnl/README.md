# scx_lnl

This is a single user-defined scheduler used within [`sched_ext`](https://github.com/sched-ext/scx/tree/main), which is a Linux kernel feature which enables implementing kernel thread schedulers in BPF and dynamically loading them. [Read more about `sched_ext`](https://github.com/sched-ext/scx/tree/main).

## Overview

An interactive laptop-oriented scheduler (Lunar Lake-focused) which aims to reduce energy usage
without compromising everyday responsiveness (e.g. browsing/scrolling), while providing full
performance when the system is set to performance mode.

The design is based on `scx_flash` (EDF with latency weighting), but adds:

- A primary scheduling domain (typically E-cores) to consolidate light background work efficiently.
- A performance domain (typically P-cores) used for interactive bursts and performance mode.
- Optional CPU frequency hinting via `scx_bpf_cpuperf_set()` (intended for `schedutil`).
- Power-profile tracking to adapt behavior at runtime (no reload required).

## Concepts

- **Primary domain** (`--primary-domain`): the default set of CPUs used for dispatching. With a
  hybrid CPU this is typically set to the efficiency cores in balanced mode.
- **Perf domain** (`--perf-domain`): preferred CPUs for interactive bursts. On hybrid systems this
  is typically the performance cores.

In `--primary-domain auto` mode, `scx_lnl` updates the primary domain and a few profile-driven
knobs when the system power profile changes (e.g. balanced → performance).

## Power Profile Integration

`scx_lnl` tracks the system power profile at runtime using:

- `power-profiles-daemon` (via D-Bus), if available, or
- `/sys/devices/system/cpu/cpufreq/policy0/energy_performance_preference` (or `scaling_governor`).

When `--primary-domain auto` is used:

- In non-performance profiles, it prefers energy-efficient CPUs as the primary domain.
- In performance profile, it prefers performance CPUs as the primary domain and enables more
  aggressive performance knobs (e.g. overflowing to non-primary sooner, preferring perf cores for
  interactive bursts).

## CPU Frequency Control (`--cpufreq`)

With `--cpufreq`, `scx_lnl` drives `scx_bpf_cpuperf_set()` requests based on observed CPU load and
interactive wakeups. This is intended for use with the `schedutil` governor (e.g. `intel_pstate=passive`
with `intel_cpufreq` on Intel systems).

In performance profile (with `--primary-domain auto`), `scx_lnl` requests max cpuperf on the perf
domain to enable peak boost when needed.

## Usage

### Build

From the `scx/` repo root:

```bash
cargo build -p scx_lnl --release
```

### Run

Example (auto domains + cpuperf control):

```bash
sudo ./target/release/scx_lnl --primary-domain auto --perf-domain performance --cpufreq
```

Notes:

- If you set `--primary-domain` to a fixed domain (not `auto`), `scx_lnl` will not change domains or
  profile-driven knobs when the system profile changes.
- For the full list of options, run `./target/release/scx_lnl --help`.

### Monitoring / Stats

Print a one-line delta summary every second while the scheduler runs:

```bash
sudo ./target/release/scx_lnl --stats 1 --primary-domain auto --perf-domain performance --cpufreq
```

`cpuperf -> max:` reports how often the scheduler requested max cpuperf from load tracking during
the last interval.

## Typical Use Case

Daily-driver laptops where power efficiency matters, but interactive latency is
still the priority.

## Production Ready?

Experimental.
