# CPU-Z 2.15: feature inventory (clean-room spec)

This is the spec the Rust reimplementation is built from. It records **what** CPU-Z does and **which data it shows**. It does not record how CPU-Z's code does it. Implementations follow public standards only.

Sources:
- `reference/cpuz_dump.txt`: CPU-Z's own `-txt` report from this machine (Ryzen 9 7900X, MSI PRO X870-P, DDR5, RTX 4060).
- `cpuz_readme.txt` and `cpuz.ini`.
- The REA + Ghidra inventory of strings and RTTI class names in `cpuz.exe`: 429 domain classes. No function was decompiled or translated.

Per-OS legend: ✅ user mode · 🔑 needs admin/root · 🧩 needs a kernel driver (Windows: PawnIO) · ⚠ partial or equivalent data · ❌ impossible on that platform (with the reason).

## 1. Interface and modes

| # | Feature | Win | Linux | macOS |
|---|---|---|---|---|
| 1.1 | Tabs CPU, Caches, Mainboard, Memory, SPD, Graphics, Bench, About | ✅ | ✅ | ✅ |
| 1.2 | Core selector (right-click / combo), P/E/LP cores | ✅ | ✅ | ✅ |
| 1.3 | Tools ▸ Clocks: live clock of every core, bus, cache, memory, GPU | ✅/🧩 | ✅/🔑 | ⚠ |
| 1.4 | Tools ▸ Timers: ACPI timer, perf counter, system timer | ⚠ ACPI 🧩 | ✅ | ⚠ no ACPI PM timer |
| 1.5 | Save Report as .TXT / .HTML | ✅ | ✅ | ✅ |
| 1.6 | Validate / Validator Statistics (valid.x86.fr) | out of scope: CPUID's service |||
| 1.7 | F5 tab → BMP, F6 tab → clipboard, F7 `.cvf` file | ✅ (`.cvf` = our own JSON) | ✅ | ✅ |
| 1.8 | Check for updates | ⚠ GitHub releases of this project | same | same |
| 1.9 | `cpuz.ini`: TextFont*, LabelFont*, ACPI, PCI, MaxPCIBus, DMI, Sensor, SMBus, Display, UseDisplayAPI, BusClock, Chipset, SPD, LPCIO, SMART, XOC, CheckUpdates | ✅ (`cpuzeez.ini`, same keys) | ✅ | ✅ |
| 1.10 | Tray/clock mode when minimized | ✅ | ✅ | ✅ |

## 2. Command line (from the readme plus strings in the exe)

`-txt=<file>`, `-html=<file>`, `-json=<file>`, `-jsontxt=<file>`, `-console`, `-bench`, `-core=<id>`, `-coremask=<hex>`. All of them ✅ on all three platforms.

## 3. Report sections (`-txt`, same order as the dump)

| # | Section | Contents | Win | Linux | macOS |
|---|---|---|---|---|---|
| 3.1 | Binaries | version | ✅ | ✅ | ✅ |
| 3.2 | Processors | CPU groups + mask, sockets, threads | ✅ | ✅ | ✅ |
| 3.3 | APICs | tree Socket → Node → CCD → CCX/Module → Core(ID) → Thread(APIC ID) | ✅ CPUID per pinned thread | ✅ | ⚠ ARM: cluster/perf-level topology from sysctl |
| 3.4 | Timers | ACPI timer, Perf timer, Sys timer frequencies | 🧩/✅ | ✅ | ⚠ |
| 3.5 | Processors Information | see §4 | | | |
| 3.6 | Thread dumps | per thread: APIC ID, topology, AMD/Intel topology, cache descriptors, raw CPUID leaves (std + ext), MSR dump | ✅ CPUID, 🧩 MSR | ✅, 🔑 MSR | ⚠ ARM: MIDR/ID registers via sysctl; ❌ MSR |
| 3.7 | BIOS | UEFI yes/no, BIOS date, AMD AGESA / Intel microcode | ✅ | ✅ | ⚠ Apple firmware version |
| 3.8 | Chipset | northbridge, southbridge + rev, bus spec (PCIe gen), graphic interface, link width/speed, memory type/size/channels, memory frequency + ratio, CL/tRCD/tRP/tRAS/tRC/CR/tRFC, uncore freq | ✅ names/link · 🧩 IMC timings | ✅ · 🔑 IMC (PCI config / AMD SMN / Intel MCHBAR) | ⚠ Apple SoC fabric; ❌ IMC timings |
| 3.9 | Memory SPD | per DIMM: SMBus addr, type, format, module/DRAM mfr (JEP106), size, PMIC, CKD, max bandwidth, max JEDEC, channels, part/serial, date, voltage, temp limits, EPP/XMP/AMP/EXPO, JEDEC table, XMP/EXPO profiles with full timings, raw hex dump | 🧩 SMBus | 🔑 `ee1004`/`spd5118`/`i2c-dev` | ❌ soldered LPDDR, no SPD EEPROM exposed (SMBIOS-equivalent data only) |
| 3.10 | Monitoring | mainboard model + IDs | ✅ | ✅ | ✅ |
| 3.11 | LPCIO | Super I/O vendor/model/IDs, config mode regs, LDN, register space | 🧩 port I/O | 🔑 `/dev/port` | ❌ no LPC Super I/O |
| 3.12 | Hardware Monitors | voltages/temps/fans/power/clock/utilization per chip (Super I/O, DIMM thermal sensor, CPU SMU/MSR, GPU, PSU, battery, UPS) | 🧩 + vendor APIs | ✅ hwmon (kernel drivers), 🔑 raw | ⚠ SMC + IOReport (no root) |
| 3.13 | PCI Devices | full enumeration: location, common header, type 0/1/2 header, every capability (PM, MSI, MSI-X, PCIe, AER, VC, SN, ResizableBAR, …), raw config dump | ✅ basic · 🧩 config space | ✅ sysfs (🔑 full 4 KB) | ⚠ IOKit properties (no raw config) |
| 3.14 | DMI | SMBIOS version + types 0,1,2,3,4,5,6,7,9,11,16,17,8,40 (BIOS, system, baseboard, enclosure, processor, mem controller/module, port connectors, slots, OEM strings, memory array/devices, additional info) | ✅ RSMB | ✅ `/sys/firmware/dmi/tables` (🔑) or `/sys/class/dmi/id` | ✅ Intel Macs (`AppleSMBIOS`); ⚠ Apple Silicon (IODeviceTree) |
| 3.15 | Storage | per drive: path, name, revision, serial, capacity, type SSD/HDD, bus, RPM, SMART, controller, link, volumes | ✅ | ✅ | ✅ |
| 3.16 | USB Devices | name, class/subclass, VID/PID | ✅ | ✅ | ✅ |
| 3.17 | Graphics / Graphic APIs | list of available APIs (D3D, NVAPI, NVML, ADL, IGCL, Level Zero) | ✅ | ✅ (NVML, sysfs amdgpu/i915/xe) | ⚠ Metal |
| 3.18 | Display Adapters | name, board vendor, codename, revision, cores/ROP/TMU/RT/Tensor, process, mem type/vendor/size/bus, power/thermal limits, PCIe link, PCI IDs, base/boost/current clocks, driver, WDDM | ✅ | ✅ | ⚠ Apple GPU core count, Metal family |
| 3.19 | Monitors | EDID: model, vendor, ID, serial, date, size, max resolution, H/V ranges, pixel clock, gamma, input | ✅ | ✅ `/sys/class/drm/*/edid` | ✅ IOKit `IODisplayEDID` |
| 3.20 | AI / NPU Devices | NPUs (DirectML/D3D, oneAPI) | ✅ | ⚠ `/sys/class/accel` | ⚠ ANE present (core count) |
| 3.21 | Software | OS name/edition/version/build, install date, DirectX version | ✅ | ✅ distro + kernel (+ Vulkan/OpenGL version instead of DirectX) | ✅ macOS version (+ Metal) |
| 3.22 | Register Spaces | the PCI/PCIe-ECAM/LPC/SMBus/PMIO spaces in use | 🧩 | 🔑 | ❌ |

## 4. CPU tab fields

Manufacturer, name, codename, specification (brand string), package/socket, CPUID family.model.stepping, ext family/model, core stepping/revision, process technology, TDP, Tjmax, core speed, multiplier × bus, base frequency, instruction sets (MMX…AVX-512 subsets, AMX, AVX10, SHA, AES, VT-x/AMD-V, etc.), microcode revision, L1D/L1I/L2/L3 (count × size, ways, line), preferred cores, max CPUID levels, FID/VID control, P-states (FID/VID → ratio/voltage), AMD SMU version / PPT / TDC / EDC / PBO, Intel PL1/PL2/turbo ratios, per-core temperatures/voltages/powers/clocks, effective max ratio per core, hybrid core types (P/E/LP).

| Data | Win | Linux | macOS x86 | macOS ARM |
|---|---|---|---|---|
| CPUID-derived fields (name, FMS, instructions, caches, topology) | ✅ | ✅ | ✅ | ⚠ sysctl `hw.*`/`machdep.*`, MIDR |
| Codename / package / technology / TDP | ✅ own DB from public data | ✅ | ✅ | ✅ (M1…M5 table) |
| Real core clock, multiplier, bus | 🧩 MSR APERF/MPERF/TSC | 🔑 msr or ✅ cpufreq | ⚠ | ⚠ IOReport |
| Microcode | ✅ registry | ✅ `/proc/cpuinfo` | ✅ sysctl | n/a |
| P-states, SMU, PPT/TDC/EDC, PL1/PL2 | 🧩 | 🔑 msr + `/sys` | ❌ | ⚠ IOReport power |
| Temperatures/voltages/power per core | 🧩 | ✅ hwmon k10temp/zenpower/coretemp/RAPL powercap | ⚠ SMC | ⚠ SMC/IOReport |

## 5. Caches tab

L1D, L1I, L2, L3 (and L4 / eDRAM): size, associativity, line size, instances, inclusive/exclusive, sharing threads. ✅ on all three (ARM: sysctl/sysfs).

## 6. Mainboard tab

Mainboard vendor/model/revision; chipset (NB + SB, rev); LPCIO; BIOS brand/version/date; microcode; PCIe graphic interface bus/current link/max link. ✅ (chipset revision 🧩/🔑).

## 7. Memory tab

Type, size, channels, mem controller freq, uncore freq; DRAM freq, FSB:DRAM, CL, tRCD, tRP, tRAS, tRC, CR, tRFC, tRDRAM, tRCD-to-col, idle timer. Type and size ✅ everywhere. Live timings 🧩 / 🔑 / ❌ on macOS.

## 8. SPD tab

Slot selector, type, max bandwidth, manufacturer, part number, serial, week/year, module size, ranks, registered/buffered, SPD extension (XMP/EXPO/EPP/AMP), timings table (JEDEC + profiles × freq/CL/tRCD/tRP/tRAS/tRC/CR/voltage). Decoders for **SDR, DDR, DDR2 (incl. FB-DIMM), DDR3, DDR4, DDR5 (incl. CUDIMM/CAMM2), LPDDR4/4X/5**, plus XMP 1.x/2.0/3.0, EXPO 1.0, EPP, AMP. The decoders are pure code (✅ testable anywhere). Reading the raw bytes needs 🧩 / 🔑 / ❌ (macOS).

## 9. Graphics tab

GPU selector, name, board mfr, codename, revision, technology, TDP, core/shader/memory clocks, memory size/type/vendor/bus width, compute units. See §3.18.

## 10. Bench tab

Single-thread and multi-thread score, configurable thread count, single cluster/coreset option, AVX2 / AVX-512 variants, reference CPU comparison, "Stress CPU". This will be **our own benchmark**: CPU-Z's algorithm and its reference database of scores belong to CPUID. Scores will not be comparable with CPU-Z's. ✅ everywhere.

## 11. About tab

Version, tools buttons (Validate omitted, Save report, Clocks, Timers), Windows/OS version, DirectX/Metal/Vulkan version. ✅

## 12. Hardware database coverage (CPU-Z RTTI families → our modules)

| CPU-Z module family (count) | Reimplementation |
|---|---|
| CPU info: Intel P5/P6/Netburst/Core/Core2/Nehalem/Sandy Bridge/Haswell/Skylake/Bonnell/Silvermont/Goldmont/KNL/Mobile/Itanium, AMD K5…K17/K19+, Hygon, VIA C7/CNA/CHA, Zhaoxin KX, Cyrix, NexGen, Rise, SiS, UMC, Transmeta, DMP (≈35) | One CPUID decoder + data tables from public sources (Intel SDM, AMD PPR, the Linux `intel-family.h`, InstLatx64). IA-64/Itanium ❌: no Rust target and not a supported OS. Pre-i686 CPUs ❌: none of the three target OSes boots on them |
| FIDVID controls K7…K1A, Core/Core2/Nehalem/SNB/Silvermont/Goldmont/Netburst/Mobile/ZX/CNA (≈22) | P-state decoders over MSR (🧩/🔑) for the families supported by Win10+/Linux/macOS |
| SMBus controllers: Intel ICH/PCH/Z170/SKL-X/HSW-E/5000X/E8500/SCH, AMD SB/X300/X600/AM755/MLN/STP I2C, ATI SB, nVidia, VIA VT82, SiS, ALi, Zhaoxin, Nuvoton 6797, Transmeta, plus Asus/EVGA/Intel multiplexers (≈30) | Windows: PawnIO SMBus modules (i801, piix4/FCH) + our own drivers for the others. Linux: `i2c-i801`/`i2c-piix4` and the other kernel drivers via `i2c-dev` |
| Hardware monitors: Nuvoton NCT6xxx/7xxx, ITE IT87, Winbond W836xx, Fintek F71/F8, SMSC, NS, Analog Devices ADM/ADT, Maxim, CHIL, Volterra, Asus EC/ROG/uGuru, Gigabyte ODIN/Aorus PSU, Dell SMI, ThinkPad, Apple SMC, AMD Ryzen/Radeon SMU, NVIDIA ESA, DIMM TS, batteries, APC UPS (≈90) | Linux: kernel hwmon (covers almost all of them). Windows: our own Super I/O drivers over PawnIO for the chips with public register maps, plus vendor APIs. macOS: SMC + IOReport |
| Display APIs: D3D, PDH, NVAPI, NVML, ADL, IGCL, Level Zero, DirectIO (NVIDIA/ATI/Intel/ASpeed/Glenfly) (≈12) | Win: DXGI + NVML + ADLX/ADL + IGCL; Linux: DRM sysfs + NVML; macOS: Metal/IOKit. DirectIO register pokes ❌ (replaced by the official APIs) |
| PCI capabilities (25 kinds) | Pure decoder (✅) over config bytes |
| DMI structures (13 types) | Pure SMBIOS parser (✅) |
| WMI: BIOS, Baseboard, MemoryDevice, System, SecurityCenter | Windows fallback |
| NPU: D3D, oneAPI | Windows DXCore; Linux accel; macOS ANE |
| Win9x VxD, 32-bit driver, IA-64 driver | ❌ obsolete |

## 13. Explicitly out of scope

- Online validation / submission to valid.x86.fr (CPUID's proprietary service).
- CPU-Z's benchmark algorithm and its reference scores (we ship our own benchmark).
- Code, tables or text copied from `cpuz.exe`. Our names databases are built from public data.
