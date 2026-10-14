# PawnIO on Windows

cpuzeez ships no kernel driver. Without one, Windows user mode cannot read
MSRs, I/O ports, the SMBus or AMD SMN, so on Windows those features come from
[PawnIO](https://pawnio.eu), a signed, open-source driver by namazso. PawnIO
runs small signed Pawn programs ("modules"), and each module only exposes a
narrow, audited set of operations. LibreHardwareMonitor and FanControl use it
too. cpuzeez never uses WinRing0 or any other vulnerable driver.

When PawnIO or its modules are missing, cpuzeez still runs and reports
`Hardware access: none (user mode only)`. Everything that needs no privileges
(CPUID, caches, SMBIOS, OS-provided information) keeps working.

## Installing PawnIO

Install one of these (elevation is required):

- the official installer `PawnIO_setup.exe` from https://pawnio.eu, which
  links to https://github.com/namazso/PawnIO.Setup/releases
- `winget install --id namazso.PawnIO --exact` (the package is in
  [winget-pkgs](https://github.com/microsoft/winget-pkgs/tree/master/manifests/n/namazso/PawnIO)
  and the silent switches are `-install -silent`)

The installer registers `HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\PawnIO`
(`InstallLocation`, `DisplayVersion`) and installs to `%ProgramFiles%\PawnIO`.
The install path cannot be changed. That folder contains `PawnIOLib.dll`.

**Run cpuzeez as Administrator.** From a non-elevated process `pawnio_open`
fails, and cpuzeez falls back to `none (user mode only)`. This was confirmed
with PawnIO 2.2.0. An elevated run on a Zen machine reported
`PawnIO 2.2.0.0 (modules: AMDFamily17, LpcIO, SmbusPIIX4)`. The name lists
every module that loaded, so `RyzenSMU` or `IntelMCHBAR` appear there when
they load.

## Modules

Get the official signed modules from the PawnIO.Modules releases page:
https://github.com/namazso/PawnIO.Modules/releases. cpuzeez was tested with
release **0.2.11**:

- file: `release_0_2_11.zip` (69582 bytes)
- sha256: `43608cb89bc84247fef1368a139013f7d043e17db6d6c8dfc9b46bf0905a81f4`
  (this matches the asset digest GitHub publishes)

Check the hash with
`(Get-FileHash release_0_2_11.zip -Algorithm SHA256).Hash`. Then extract the
`.bin` files into a folder named `pawnio-modules` next to `cpuzeez.exe`.
Copying the whole zip content is fine, because cpuzeez loads only the modules
it uses:

```
cpuzeez.exe
pawnio-modules\
    COPYING                (LGPL-2.1, keep it with the blobs)
    IntelMSR.bin           Intel MSRs
    IntelMCHBAR.bin        Intel memory-controller MMIO (IMC timings)
    AMDFamily17.bin        AMD Zen MSRs + SMN (families 17h-1Ah)
    AMDFamily10.bin        AMD families 10h-16h MSRs
    AMDFamily0F.bin        AMD K8 MSRs
    RyzenSMU.bin           AMD SMU version + PM (metrics) table
    ZhaoxinMSR.bin         Zhaoxin/VIA MSRs
    LpcIO.bin              Super I/O (sensors)
    SmbusI801.bin          Intel SMBus (SPD)
    SmbusPIIX4.bin         AMD SMBus (SPD)
```

For a `cargo run` build, use `target\debug\pawnio-modules\` or
`target\release\pawnio-modules\`.

cpuzeez looks for `<Name>.bin` in this order:

1. `<exe dir>\pawnio-modules\`
2. the PawnIO install directory
3. `<install dir>\modules\`

A module that is missing, or whose `main()` rejects this machine (for example
the wrong CPU family or no matching chipset), is skipped.

**Why the modules are not in the repo.** The modules are LGPL-2.1-or-later.
The official signed builds come from the PawnIO.Modules releases, and the
project wiki tells applications to obtain them there and embed them.
Redistributing them is allowed if you follow the LGPL: ship the license text
and point to the matching source tag. LibreHardwareMonitor does this. cpuzeez
is MIT OR Apache-2.0, so to keep the repository single-licensed we do not
commit the blobs. A packager may put them in `pawnio-modules\` together with
the module repo's `COPYING` and a note naming the release tag. The PawnIO
driver itself is not redistributed. Its official binaries are proprietary,
although redistributing the installer is allowed, so users install it
themselves.

## What needs PawnIO, and which module provides it

| cpuzeez feature | Hw call | Module, function |
|---|---|---|
| Core clock (APERF/MPERF), Intel ratios (0xCE, 0x198, 0x1AD), RAPL, temperatures | `rdmsr` | `IntelMSR` → `ioctl_read_msr` |
| AMD P-states (0xC0010064..6B), CPPC, APERF/MPERF (**only the read-only copies at 0xC00000E7/E8**; 0xE7/0xE8 are not allowed), RAPL | `rdmsr` | `AMDFamily17` (fam. 17h–1Ah), `AMDFamily10` (10h–16h), `AMDFamily0F` → `ioctl_read_msr` |
| AMD UMC (memory timings on Zen), SMN registers | `amd_smn_read` | `AMDFamily17` → `ioctl_read_smn` |
| AMD SMU firmware version, PM table (FCLK/UCLK/MCLK, SoC voltages, power) | `amd_smu_version`, `amd_smu_pm_table` | `RyzenSMU` → `ioctl_get_smu_version`; `ioctl_resolve_pm_table`, `ioctl_update_pm_table`, `ioctl_read_pm_table` |
| Super I/O sensors (voltages, fans, temperatures) | `io_in8` / `io_out8` | `LpcIO` → `ioctl_select_slot`, `ioctl_find_bars`, `ioctl_pio_inb`, `ioctl_pio_outb` |
| SPD (DIMM EEPROM), DIMM thermal sensors, DDR5 PMIC | `smbus()` | `SmbusI801` (Intel PCH), `SmbusPIIX4` (AMD FCH) → `ioctl_smbus_xfer`, `ioctl_identity` |
| Intel IMC timings | `mchbar_read32` / `mchbar_read64` | `IntelMCHBAR` → `ioctl_read_dword`, `ioctl_read_qword` (offset must be aligned and inside the generation's MCHBAR: 32, 64 or 128 KiB) |

The PM table comes back as a whole 4 KiB page, which is 1024 f32 values. The
layout depends on the table version, and any values past the end of that
version's layout are padding.

On Linux, the same calls come from the out-of-tree `ryzen_smu` driver
(`/sys/kernel/ryzen_smu_drv/{version,pm_table_version,pm_table}`). MCHBAR is
read from host bridge config registers 0x48/0x4C plus an uncached `/dev/mem`
mapping. Both need root.

Limits, each enforced by the modules themselves:

- **MSR allow-list.** Every MSR module answers only the MSRs it lists. Any
  other MSR returns `ACCESS_DENIED`, which cpuzeez shows as "not available".
- **Ports.** LpcIO allows only the Super I/O index/data ports (0x2E/0x2F,
  0x4E/0x4F) and the base addresses it finds while the chip is in config mode.
  cpuzeez calls `ioctl_find_bars` just before the caller leaves config mode, or
  on the first denied access. PCI ports 0xCF8–0xCFF are always denied.
- **No generic PCI config space or physical memory access.** `pci_read`,
  `io_in32` and `phys_read` are unavailable on Windows.
- **SMBus.** PIIX4 uses port 0, the one DIMMs sit on. The controller's
  secondary ports are not selected.

cpuzeez honors the cross-process mutexes that the modules document, so it
does not collide with other monitoring tools: `Global\Access_PCI` (SMN, SMU),
`Global\Access_ISABUS.HTP.Method` (Super I/O) and
`Global\Access_SMBUS.HTP.Method` (SMBus).

## API reference (as used)

From `PawnIOLib.h` in https://github.com/namazso/PawnIO. Every function
returns an `HRESULT` and uses `STDAPICALLTYPE`:

```c
HRESULT pawnio_version(PULONG version);          // (major<<16)|(minor<<8)|patch
HRESULT pawnio_open(PHANDLE handle);             // one handle = one module
HRESULT pawnio_load(HANDLE h, const UCHAR* blob, SIZE_T size);
HRESULT pawnio_execute(HANDLE h, PCSTR name,     // name < 32 chars, "ioctl_*"
                       const ULONG64* in, SIZE_T in_size,
                       PULONG64 out, SIZE_T out_size, PSIZE_T return_size);
HRESULT pawnio_close(HANDLE h);
```

cpuzeez loads `PawnIOLib.dll` at runtime with `LoadLibraryW` and
`GetProcAddress`, so there is no link-time dependency. MSR modules run on
whichever CPU executes the call, so `rdmsr(cpu, …)` first pins the thread
to that CPU.
