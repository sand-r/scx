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
- **Overflow**: tasks which can't run on the primary domain (or tasks spilling due to load) are
  queued separately so non-primary CPUs don't wake up unless overflow is needed.

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

## Energy Model (CPU Selection)

`scx_lnl` can optionally use the kernel energy model (as exported via sysfs and ingested by user
space) to rank idle CPUs and prefer the most energy-efficient CPU within the candidate set.

Enable with:

- Enabled by default.
- `--no-energy-aware`: disable energy-model-based idle CPU ranking.

## Kernel Kconfig (`__kconfig` externs)

The BPF side uses a small number of `__kconfig` externs (currently `CONFIG_HZ`) for timing.

If your kernel doesn't expose system Kconfig (e.g. missing `/proc/config.gz` and
`/boot/config-$(uname -r)`), `scx_lnl` will try to auto-detect a config from:

- `/boot/config-<release>`
- `/lib/modules/<release>/build/.config`
- `/lib/modules/<release>/build/include/config/auto.conf`

You can also pass a config explicitly via `--kconfig <path>` (see `--help`).

## Usage

### Build

From the `scx/` repo root:

```bash
cargo build -p scx_lnl --release
```

### Install

Recommended (system-wide, good for systemd):

```bash
sudo install -Dm755 ./target/release/scx_lnl /usr/local/bin/scx_lnl
```

Alternative (user-local):

```bash
cargo install --path scheds/rust/scx_lnl
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

### Autostart After Boot (systemd)

1) Create `/etc/systemd/system/scx_lnl.service`:

```ini
[Unit]
Description=sched_ext scheduler (scx_lnl)
ConditionPathExists=/sys/kernel/sched_ext
After=multi-user.target

[Service]
Type=simple
ExecStart=/usr/local/bin/scx_lnl
Restart=on-failure
RestartSec=1
LimitMEMLOCK=infinity

[Install]
WantedBy=multi-user.target
```

2) Enable and start it:

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now scx_lnl.service
```

3) Verify:

```bash
systemctl status scx_lnl.service
cat /sys/kernel/sched_ext/root/ops
```

## Defaults / Key Knobs

The most relevant defaults for CPU placement and responsiveness:

- `--primary-domain auto`: tracks the system power profile and updates domains/knobs at runtime.
- `--perf-domain performance`: prefers performance cores for interactive bursts.
- `--interactive-nvcsw-thresh 4`: classifies tasks as interactive based on voluntary context switch
  rate.
- `--interactive-boost-ms 20`: wakeup boost window for interactive tasks.
- `--interactive-boost-lvl 512`: cpuperf floor during interactive boosts (when cpuperf control is
  enabled).
- `--watchdog-kick-ms 2000`: periodically kicks an idle CPU to avoid sched_ext watchdog false
  positives on some kernels (set to `0` to disable).
- `--cpu-busy-thresh -1`: dynamic busy threshold (derived from global user CPU time) used to decide
  when a CPU is "busy" and should overflow more aggressively.
- Energy-aware idle CPU selection: enabled by default (disable with `--no-energy-aware`).

In non-performance power profiles, `scx_lnl` tries hard to keep work on the primary domain (typically
E-cores) and uses the perf domain (typically P-cores) as an escape hatch for interactive bursts when
the primary domain is saturated and/or the previously used CPU is considered busy.

## Typical Use Case

Daily-driver laptops where power efficiency matters, but interactive latency is
still the priority.

## Production Ready?

Experimental.
