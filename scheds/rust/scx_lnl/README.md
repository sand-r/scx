# scx_lnl

This is a single user-defined scheduler used within [`sched_ext`](https://github.com/sched-ext/scx/tree/main), which is a Linux kernel feature which enables implementing kernel thread schedulers in BPF and dynamically loading them. [Read more about `sched_ext`](https://github.com/sched-ext/scx/tree/main).

## Overview

An interactive laptop-oriented scheduler (Lunar Lake-focused) which aims to reduce energy usage
without compromising everyday responsiveness (e.g. browsing/scrolling).

The design is based on `scx_flash` (EDF with latency weighting), but adds:

- Preference for energy-efficient CPUs (e.g. E-cores) under light load.
- A performance escape hatch for interactive bursts (e.g. P-cores).
- CPU frequency control via `scx_bpf_cpuperf_set()` (intended for `schedutil`).

## Typical Use Case

Daily-driver laptops where power efficiency matters, but interactive latency is
still the priority.

## Production Ready?

Experimental.
