# scx_lnl

`scx_lnl` is an experimental laptop-oriented [`sched_ext`](https://github.com/sched-ext/scx/tree/main)
scheduler forked from `scx_flash` and tuned for Intel hybrid laptops such as Lunar Lake systems.

## Overview

The scheduler keeps the `scx_flash` EDF / wakeup-frequency foundation, then layers on defaults and
small policy changes aimed at everyday laptop use:

- `--primary-domain=auto` prefers E-cores for powersave, balanced, and unknown power profiles.
  Performance profile uses P-cores. Tasks can still overflow outside the primary domain when it is
  saturated or affinity requires it.
- Power-profile changes are handled at runtime by reprogramming the primary CPU domain instead of
  forcing a scheduler restart.
- CPU idle QoS is disabled by default (`--idle-resume-us=-1`) so the platform can use deeper idle
  states unless explicitly configured otherwise.
- On `intel_pstate=active`, HWP owns frequency selection, so `scx_lnl` reports cpufreq control as
  disabled rather than pretending `scx_bpf_cpuperf_set()` is effective.
- A watchdog-kick timer is enabled by default to avoid false sched_ext runnable-stall detection on
  systems that spend long periods idle.
- Wakeup-abuse throttling follows upstream `scx_flash`: it is available via `--wakeup-throttle`, but
  disabled by default.

## Status

Experimental and laptop-focused. It tracks current upstream `scx_flash` framework patterns where
possible, while intentionally diverging in power-domain and idle behavior.

## Build

From the `scx/` repo root:

```bash
SCX_GIT_SHA=$(git rev-parse --short=12 HEAD) cargo build -p scx_lnl --release
```

## Run

```bash
sudo ./target/release/scx_lnl
```

For the full list of options:

```bash
./target/release/scx_lnl --help
```
