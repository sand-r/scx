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

## CPU Frequency Control

`scx_lnl` can drive `scx_bpf_cpuperf_set()` requests based on observed CPU load and interactive
wakeups. This is intended for use with the `schedutil` governor (e.g. `intel_pstate=passive` with
`intel_cpufreq` on Intel systems).

By default, cpuperf control is auto-enabled when `scx_lnl` detects:

- `schedutil` is active (`/sys/devices/system/cpu/cpufreq/policy0/scaling_governor`), and
- `intel_pstate` is not `active` (so we don't fight the in-kernel hardware governor).

Overrides:

- `--cpufreq`: force enable cpuperf control.
- `--no-cpufreq`: force disable cpuperf control.

In performance profile (with `--primary-domain auto`), `scx_lnl` requests max cpuperf on the perf
domain to enable peak boost when needed.

## Usage

### Build

From the `scx/` repo root:

```bash
cargo build -p scx_lnl --release
```

### Run

Default (recommended):

```bash
sudo ./target/release/scx_lnl
```

Notes:

- Defaults are equivalent to `--primary-domain auto --perf-domain performance` and `cpufreq` set to
  auto.
- If you set `--primary-domain` to a fixed domain (not `auto`), `scx_lnl` will not change domains or
  profile-driven knobs when the system profile changes.
- For the full list of options, run `./target/release/scx_lnl --help`.

### Monitoring / Stats

Print a one-line delta summary every second while the scheduler runs:

```bash
sudo ./target/release/scx_lnl --stats 1
```

`cpuperf -> max:` reports how often the scheduler requested max cpuperf from load tracking during
the last interval.

## Defaults / Key Knobs

The most relevant defaults for CPU placement and responsiveness:

- `--primary-domain auto`: tracks the system power profile and updates domains/knobs at runtime.
- `--perf-domain performance`: prefers performance cores for interactive bursts.
- `--interactive-nvcsw-thresh 4`: classifies tasks as interactive based on voluntary context switch
  rate.
- `--interactive-boost-ms 20`: wakeup boost window for interactive tasks.
- `--interactive-boost-lvl 512`: cpuperf floor during interactive boosts (when cpuperf control is
  enabled).
- `--cpu-busy-thresh -1`: dynamic busy threshold (derived from global user CPU time) used to decide
  when a CPU is "busy" and should overflow more aggressively.

In non-performance power profiles, `scx_lnl` tries hard to keep work on the primary domain (typically
E-cores) and uses the perf domain (typically P-cores) as an escape hatch for interactive bursts when
the primary domain is saturated and/or the previously used CPU is considered busy.

## Typical Use Case

Daily-driver laptops where power efficiency matters, but interactive latency is
still the priority.

## Production Ready?

Experimental.
