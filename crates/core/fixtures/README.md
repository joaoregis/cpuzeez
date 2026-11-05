# Test fixtures (S0-T04)

Recorded inputs for the pure parsers in `cpuzeez-core`, so every OS runs the
Linux, Windows and macOS decoding tests without that OS or its hardware.

- One folder per source: `linux/` (files under `/proc` and `/sys`), later
  `nvme/`, `smart/`, `smu/`, `presentmon/`, `mangohud/`.
- Name a file after what it records (`diskstats` is `/proc/diskstats`).
- Tests load them with `include_str!("../fixtures/…")` / `include_bytes!`,
  so a missing fixture fails the build, not the run.
- Strip anything identifying before committing a real capture: serial
  numbers, hostnames, MAC/IP addresses, user names, paths.
