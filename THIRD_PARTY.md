# Third-party components

CpuZeeZ itself is MIT OR Apache-2.0. This file lists what its packages ship
or compile in from others. Bundled binaries are pinned to a version and a
SHA-256 in [`packaging/pins.txt`](packaging/pins.txt), downloaded from their
official releases and checked by the release workflow.

## Shipped next to the app (Windows packages)

| Component | Version | Author | License | What it's for |
|---|---|---|---|---|
| **PawnIO driver installer** (`PawnIO_setup.exe`) | 2.2.0 | namazso, [pawnio.eu](https://pawnio.eu) | Proprietary; redistribution of the installer is allowed | Hardware access (CPU temperatures, voltages, SPD, memory timings). The MSI installs it through its *Hardware access* feature, which is ticked by default and can be unticked. The portable zip installs it only when you press *Install hardware access* in the app. Shared with LibreHardwareMonitor and FanControl, so uninstalling CpuZeeZ keeps it; remove it from *Installed apps* if you want |
| **PawnIO modules** (`pawnio-modules\*.bin`) | 0.2.11 | namazso | LGPL-2.1-or-later (text in `pawnio-modules\COPYING`; source: [PawnIO.Modules 0.2.11](https://github.com/namazso/PawnIO.Modules/tree/0.2.11)) | Small signed programs the driver runs; each exposes a narrow, audited set of operations. Inert without the driver |

## Compiled into the app

Rust crates, all under licenses compatible with MIT OR Apache-2.0 (MIT,
Apache-2.0, BSD, ISC, Zlib, Unicode-3.0, MPL-2.0, BSL-1.0, OFL-1.1). CI enforces
this with `cargo deny` ([`deny.toml`](deny.toml)). Scoped exceptions:
egui's bundled fonts (Ubuntu Font Licence 1.0) and `ksni`, the Linux tray
(Unlicense, public domain). The full list: `cargo deny list`.

**SQLite** ([sqlite.org](https://sqlite.org), public domain) is compiled in
through `rusqlite`'s `bundled` feature for the App history store; no system
SQLite is used.
