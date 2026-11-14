# Graph Report - cpuzeez  (2026-11-13)

## Corpus Check
- 300 files · ~412,900 words
- Verdict: corpus is large enough that graph structure adds value.
- Unclassified: 42 file(s) not represented in the graph (top: (none) 22, .csv 5, .toml 4)

## Summary
- 6367 nodes · 14861 edges · 250 communities (226 shown, 24 thin omitted)
- Extraction: 92% EXTRACTED · 8% INFERRED · 0% AMBIGUOUS · INFERRED: 1161 edges (avg confidence: 0.8)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `07b715fd`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- sensors.rs
- Sub
- intel.rs
- Stage
- System Performance Charts
- x86.rs
- live_sessions.rs
- CPU Benchmark Runner
- SPD Decoding
- Memory Benchmark
- Storage Drives Probe
- core/src/smbios.rs
- system_history.rs
- Inner
- Metric Engine
- network.rs
- Ctx
- with
- CPU Sensors and Power Limits
- CPU Model Database
- system.rs
- update.rs
- Process Snapshot Platform
- Super I/O Sensor Chips
- actions.rs
- system_users.rs
- Windows GPU Probe
- cpusuite.rs
- Single Instance and SIDs
- data.rs
- mod.rs
- palette.rs
- elevation.rs
- disk_io.rs
- layout.rs
- icon.rs
- linux.rs
- tr
- drivetest.rs
- system_services.rs
- decode.rs
- smart.rs
- windows.rs
- peripherals.rs
- f
- src/os.rs
- diff.rs
- split.rs
- frames.rs
- hotkeys_portal.rs
- overview.rs
- mod.rs (2)
- proc_ops.rs
- Telemetry
- procs.rs
- display_drm.rs
- dxcore.rs
- details
- bottleneck_ui.rs
- fuzz.yml
- Cell
- software.rs
- gpubench.rs
- proc_net.rs
- history.rs
- tray.rs (2)
- bottleneck.rs
- mod.rs (3)
- live_sessions.rs
- procs.rs (2)
- query.rs
- mod.rs (4)
- trayicon.rs
- Class
- telemetry.rs (2)
- chipset.rs
- Reading
- desktop.rs
- display.rs
- gpu_engines.rs
- theme.rs
- executor.rs
- amd_pm.rs
- mod.rs (5)
- users.rs
- adapters
- large_pages.rs
- Column
- core/src/robustness.rs
- report.rs
- overlay.rs
- Unit
- soak.py
- nvml.rs
- live.rs
- F
- windows.rs (4)
- devices.rs
- pins.rs
- screens.rs
- diskbench.rs
- query.rs
- sensors/windows.rs
- zeez-live.md
- ci.yml
- battery.rs
- usb.rs
- startup.rs (2)
- a11y.rs
- nightly.yml
- Page
- details
- Period
- tray.rs
- availability.rs
- live.rs (2)
- PresentMon bundled sidecar (ETW)
- Ctx
- hashmap
- Column
- table.rs
- memtest.rs
- CHANGELOG.md
- live_frames.rs
- R
- .render_table
- core/src/services.rs
- superio.rs
- Source
- layout_view.rs
- cpu_os.rs
- linux.rs (2)
- Batteries
- health.rs
- .render_table
- src/health.rs
- Map
- gui/tools.rs
- Action
- V3
- src/health.rs
- FEATURES.md (CPU-Z 2.15 clean-room spec)
- ServiceOp
- CPU ZeeZ 1.1.0 plan
- CpuZeeZ 1.0 Plan (hub)
- pkgver.py
- .show_ui
- core/src/pci/mod.rs
- .groups
- raster.py
- Array
- .render_table
- direct_io.rs
- smc.rs
- tempdir.rs
- Anchor
- queryable
- dependabot.yml
- ci.yml (2)
- superio.rs (2)
- StartupType
- engines.rs
- windows.rs (5)
- Id
- jobs
- detector.rs
- memplus.rs
- timers.rs
- gpu/macos.rs
- serialize
- net_io.rs
- Bottleneck Detector Design
- best_hz
- node
- probe
- fixture.rs
- ServiceState
- ToolView
- StartupType
- diskio.rs
- src/health.rs
- gui/screens.rs
- storage-and-cleanup.md
- .show_ui
- ROADMAP.md
- make_icons.py
- superio.rs (4)
- Graphify neste projeto
- Clock
- actions.rs (2)
- StartupRow
- Op
- layout.rs (2)
- Preset
- Disks
- encryption.rs
- Period
- Id
- steps
- CLAUDE.md
- benchkit.rs
- src/soak.rs
- imp
- steps
- db.rs
- src/smbios.rs
- unsafe_ratchet.py
- startup.rs (4)
- instant
- src/pci/mod.rs
- App
- shotdiff.py
- parse_airport
- Widget
- run
- ci.yml (4)
- steps
- instant
- small_text_parsers
- StartupRow
- ci.yml (5)
- LimiterKind
- macos.rs (4)
- .graphics_tab
- strings.rs
- gui
- Monitor
- Theme
- Step
- page_tests.rs
- lib.rs
- .mainboard_more
- Cargo.toml
- fetch_pawnio.sh
- CHANGELOG.md (3)
- WindowOp
- lint_workflows.py
- Impact

## God Nodes (most connected - your core abstractions)
1. `Ctx` - 222 edges
2. `tr()` - 190 edges
3. `palette()` - 114 edges
4. `trf()` - 86 edges
5. `()` - 64 edges
6. `card()` - 63 edges
7. `Series` - 51 edges
8. `Unit` - 40 edges
9. `V` - 40 edges
10. `get()` - 39 edges

## Surprising Connections (you probably didn't know these)
- `live_rows()` --calls--> `g()`  [INFERRED]
  src/gpu/mod.rs → crates/core/src/report.rs
- `Release notes template` --semantically_similar_to--> `Unsigned releases and SHA256SUMS verification`  [INFERRED] [semantically similar]
  packaging/RELEASE_NOTES.md → README.md
- `Test fixtures README (S0-T04)` --semantically_similar_to--> `CPU-Z DDR5 TeamGroup SPD dump (test data)`  [INFERRED] [semantically similar]
  crates/core/fixtures/README.md → src/spd/testdata/cpuz_ddr5_teamgroup.txt
- `device_card()` --references--> `body`  [INFERRED]
  src/gui/devices.rs → .github/ISSUE_TEMPLATE/board-sensors.yml
- `drawer()` --references--> `body`  [INFERRED]
  src/gui/split.rs → .github/ISSUE_TEMPLATE/board-sensors.yml

## Import Cycles
- 1-file cycle: `src/live_frames.rs -> src/live_frames.rs`
- 1-file cycle: `crates/core/src/layout.rs -> crates/core/src/layout.rs`
- 1-file cycle: `src/gui/tray.rs -> src/gui/tray.rs`
- 1-file cycle: `crates/platform/src/desktop.rs -> crates/platform/src/desktop.rs`
- 1-file cycle: `crates/platform/src/display.rs -> crates/platform/src/display.rs`
- 1-file cycle: `crates/platform/src/elevation.rs -> crates/platform/src/elevation.rs`
- 1-file cycle: `src/storage.rs -> src/storage.rs`
- 1-file cycle: `crates/platform/src/gpu_engines.rs -> crates/platform/src/gpu_engines.rs`
- 1-file cycle: `crates/platform/src/large_pages.rs -> crates/platform/src/large_pages.rs`
- 1-file cycle: `crates/platform/src/memory.rs -> crates/platform/src/memory.rs`
- 1-file cycle: `crates/platform/src/procs.rs -> crates/platform/src/procs.rs`
- 1-file cycle: `src/battery.rs -> src/battery.rs`
- 1-file cycle: `src/gui/tools.rs -> src/gui/tools.rs`
- 1-file cycle: `src/gpu/dl.rs -> src/gpu/dl.rs`
- 1-file cycle: `src/usb.rs -> src/usb.rs`
- 1-file cycle: `src/gui/system_history.rs -> src/gui/system_history.rs`
- 1-file cycle: `src/npu.rs -> src/npu.rs`
- 1-file cycle: `src/software.rs -> src/software.rs`
- 1-file cycle: `src/peripherals.rs -> src/peripherals.rs`
- 2-file cycle: `src/gui/sensors.rs -> src/gui/stress.rs -> src/gui/sensors.rs`

## Communities (250 total, 24 thin omitted)

### Community 0 - "sensors.rs"
Cohesion: 0.06
Nodes (59): cpu_temp(), gpu_adapters(), resolve(), App, avg_clock(), CHART_MAX_H, CHART_MIN_H, charts() (+51 more)

### Community 1 - "Sub"
Cohesion: 0.05
Nodes (60): Conditions, conditions(), Job, median_spread(), merge(), points(), repeat(), scores_and_medians() (+52 more)

### Community 2 - "intel.rs"
Cohesion: 0.05
Nodes (56): ADL, ARL, arrow_lake(), BCLK_FREQ, decode(), degrade(), F, f() (+48 more)

### Community 3 - "Stage"
Cohesion: 0.13
Nodes (19): Updates, command(), download_and_verify(), fetch_and_verify(), in_path(), Installer, launch(), open_path() (+11 more)

### Community 4 - "System Performance Charts"
Cohesion: 0.07
Nodes (43): ago_text(), AXIS_FONT, chart(), Line, LINE_WIDTH, MARGIN_BOTTOM, MARGIN_LEFT, nearest() (+35 more)

### Community 5 - "x86.rs"
Cohesion: 0.21
Nodes (23): amd_legacy_caches(), bit(), bits(), brand(), CacheDesc, caches(), cpuid(), decodes_this_cpu() (+15 more)

### Community 7 - "live_sessions.rs"
Cohesion: 0.11
Nodes (19): Marker, Mode, Automatic, Imported, Manual, Timed, display_hz(), a_written_session_reads_back() (+11 more)

### Community 8 - "CPU Benchmark Runner"
Cohesion: 0.07
Nodes (39): Bench, CHECK_10K, CHUNK, cli(), deterministic(), fmix(), hostname(), LANES (+31 more)

### Community 9 - "SPD Decoding"
Cohesion: 0.08
Nodes (65): decode(), displayid(), displayid_ext(), dtd(), Monitor, parse(), PNP, pnp_name() (+57 more)

### Community 10 - "Memory Benchmark"
Cohesion: 0.08
Nodes (42): alloc(), Bandwidth, bandwidth(), BLOCK_LINES, build_chain(), chain_is_one_cycle_over_all_lines(), chase(), CHASE_STEP (+34 more)

### Community 11 - "Storage Drives Probe"
Cohesion: 0.07
Nodes (53): of(), ata(), ata_identify(), ata_string(), BUS, CM_Free_Log_Conf_Handle(), CM_Free_Res_Des_Handle(), CM_Get_Device_IDW() (+45 more)

### Community 12 - "core/src/smbios.rs"
Cohesion: 0.09
Nodes (45): smbios_tables(), agesa(), array_ecc(), array_location(), array_use(), Bios, bios_info(), bios_of() (+37 more)

### Community 13 - "system_history.rs"
Cohesion: 0.17
Nodes (20): accumulate(), add(), AggregatedApp, AppDayRecord, apps(), clear(), days(), ES (+12 more)

### Community 14 - "Inner"
Cohesion: 0.11
Nodes (12): Actor, Buf, Cost, fast_worker(), History, Hook, Inner, slow_worker() (+4 more)

### Community 15 - "Metric Engine"
Cohesion: 0.10
Nodes (23): a_panicking_source_is_paused_and_reported(), a_stuck_source_does_not_stall_the_others(), clamp_period(), Config, Engine, engine_stops_with_its_last_handle(), fake(), history_bounded_by_keep_stats_since_start() (+15 more)

### Community 16 - "network.rs"
Cohesion: 0.11
Nodes (22): AddrNode, band_from_mhz(), BssList, channel_from_mhz(), Close, collect(), dbm_to_pct(), DRP_DRIVER (+14 more)

### Community 17 - "Ctx"
Cohesion: 0.08
Nodes (24): Header, header(), text(), acpi(), Firmware, read(), SECURE_BOOT, BATTERIES (+16 more)

### Community 18 - "with"
Cohesion: 0.05
Nodes (52): container, needs, runs-on, steps, env, CARGO_TERM_COLOR, GH_TOKEN, TAG (+44 more)

### Community 19 - "CPU Sensors and Power Limits"
Cohesion: 0.18
Nodes (21): get(), amd(), amd_limits(), amd_rapl(), ccd_base(), core_clocks(), core_threads(), fabric_clocks() (+13 more)

### Community 20 - "CPU Model Database"
Cohesion: 0.05
Nodes (48): a(), AM2P, AM3, AM3P, AM4, AM5, ANY, APPLE (+40 more)

### Community 21 - "system.rs"
Cohesion: 0.09
Nodes (26): AffinityModal, ALL_COL_IDS, ConfirmEnd, csv_and_json_export_and_snapshot_diff(), diff_snapshots(), DiffEntry, DiffKind, Added (+18 more)

### Community 22 - "update.rs"
Cohesion: 0.11
Nodes (26): API, Asset, channels_by_label(), check(), checksum_listing(), download_dirs_are_fresh_private_and_unique(), each_channel_gets_its_newest(), FAILED (+18 more)

### Community 23 - "Process Snapshot Platform"
Cohesion: 0.07
Nodes (40): mach_timebase(), parse_major_faults(), the_rpc_endpoint_mapper_is_listening(), boot_ms(), command_line(), decoders(), elf_arch(), filetime_conversion() (+32 more)

### Community 24 - "Super I/O Sensor Chips"
Cohesion: 0.08
Nodes (46): ASR_AM4_FANS, ASR_AM4_VOLTS, ASR_AM5_VOLTS, ASR_INTEL_FANS, ASR_INTEL_VOLTS, ASR_X2_VOLTS, ASUS_800_VOLTS, ASUS_800_VTT_VOLTS (+38 more)

### Community 25 - "actions.rs"
Cohesion: 0.07
Nodes (37): Action, InstallPawnIo, LoadKernelModule, Process, Service, Startup, audit_lines_are_json(), AuditEntry (+29 more)

### Community 26 - "system_users.rs"
Cohesion: 0.04
Nodes (43): BootSummary, UserSession, ES, PT_BR, ES, PT_BR, tb(), failure_text() (+35 more)

### Community 27 - "Windows GPU Probe"
Cohesion: 0.07
Nodes (57): Adl, adl(), adl_alloc(), AdlInfo, apply_nvapi(), cm_bdf(), CM_Get_DevNode_Registry_PropertyW(), CM_Get_Parent() (+49 more)

### Community 28 - "cpusuite.rs"
Cohesion: 0.09
Nodes (40): Aes, aes_arm(), AES_BLOCKS, aes_hw(), aes_matches_fips_197(), aes_ni(), aes_sw(), counters() (+32 more)

### Community 29 - "Single Instance and SIDs"
Cohesion: 0.07
Nodes (18): WaitForSingleObject(), Claim, claim(), First, Running, H, Instance, key() (+10 more)

### Community 30 - "data.rs"
Cohesion: 0.08
Nodes (39): categories_sizes_and_delete(), Category, .ALL, AppLog, AuditLog, BenchmarkHistory, Diagnostics, Exports (+31 more)

### Community 31 - "mod.rs"
Cohesion: 0.08
Nodes (40): write_fuzz_seeds(), cpu_label(), cpu_temps(), display_name(), fake_sysfs(), put(), read(), readings() (+32 more)

### Community 32 - "palette.rs"
Cohesion: 0.13
Nodes (22): Act, ResetMinMax, SaveHtml, SaveTxt, Stress, ToggleCsv, ToggleMini, ACTIONS (+14 more)

### Community 33 - "elevation.rs"
Cohesion: 0.08
Nodes (25): can_relaunch(), elevation_query_does_not_crash(), handoff_dirs_are_fresh_private_and_unique(), handoff_marker(), handoff_marker_reports_started(), is_elevated(), Outcome, Cancelled (+17 more)

### Community 34 - "disk_io.rs"
Cohesion: 0.09
Nodes (21): CFDictionaryGetValue(), CFNumberGetValue(), CFRelease(), cfstr(), CFStringCreateWithCString(), CFTypeRef, GAP, IOIteratorNext() (+13 more)

### Community 35 - "layout.rs"
Cohesion: 0.11
Nodes (26): all(), benchmark(), big(), compact(), detailed(), expanded(), fps(), grid() (+18 more)

### Community 36 - "icon.rs"
Cohesion: 0.02
Nodes (106): ALL, ARROW_CIRCLE_UP, ARROW_CLOCKWISE, ARROW_COUNTER_CLOCKWISE, ARROW_DOWN, ARROW_ELBOW_DOWN_RIGHT, ARROW_RIGHT, ARROW_UP (+98 more)

### Community 37 - "linux.rs"
Cohesion: 0.05
Nodes (16): I2C_SLAVE_FORCE, I2C_SMBUS, I2C_SMBUS_BYTE, I2C_SMBUS_BYTE_DATA, I2C_SMBUS_READ, I2C_SMBUS_WORD_DATA, I2C_SMBUS_WRITE, I2cDev (+8 more)

### Community 38 - "tr"
Cohesion: 0.06
Nodes (39): Access, pawnio_doc(), relaunch_elevated(), bar(), save_row(), class_color(), draw_analysis(), evidence_text() (+31 more)

### Community 39 - "drivetest.rs"
Cohesion: 0.11
Nodes (36): ata_kind(), ata_log_parses_newest_first(), ata_logs(), ata_selftest(), ata_status_text(), BUCKETS, ctrl(), Entry (+28 more)

### Community 40 - "system_services.rs"
Cohesion: 0.16
Nodes (10): dispatch(), Finished, a_refused_change_leaves_no_undo_and_resets_the_editor(), combo(), hash_name(), link_button(), Modal, ConfirmStop (+2 more)

### Community 41 - "decode.rs"
Cohesion: 0.17
Nodes (28): bars(), bars_and_ext(), bytes_text(), cap(), cap_name(), cap_node(), caps(), dev() (+20 more)

### Community 42 - "smart.rs"
Cohesion: 0.16
Nodes (18): ata_attr_name(), ata_attributes(), ata_smart(), AtaAttr, entry(), from_smartctl(), from_status(), HealthLevel (+10 more)

### Community 43 - "windows.rs"
Cohesion: 0.10
Nodes (32): driver_installed(), driver_modules_found(), CloseFn, CreateMutexW(), ExecFn, exits_config(), GetProcAddress(), GlobalMutex (+24 more)

### Community 44 - "peripherals.rs"
Cohesion: 0.07
Nodes (26): Data, State, Audio, Bars, bars(), bars_of(), batteries(), BATTERY (+18 more)

### Community 45 - "f"
Cohesion: 0.11
Nodes (18): collect(), heavy_test(), main(), Out, out(), Html, Json, JsonTxt (+10 more)

### Community 46 - "src/os.rs"
Cohesion: 0.07
Nodes (32): cpu_groups(), CTRL_C, final_path(), FreeConsole(), GetActiveProcessorCount(), GetActiveProcessorGroupCount(), GetConsoleProcessList(), GetCurrentThread() (+24 more)

### Community 47 - "diff.rs"
Cohesion: 0.12
Nodes (27): Change, change(), ChangeKind, Added, Modified, Removed, cli(), diff() (+19 more)

### Community 48 - "split.rs"
Cohesion: 0.14
Nodes (21): a_narrow_window_opens_the_panel_over_the_table(), changed_id(), clamp(), drawer(), HANDLE, harness(), MAX_FRAC, MIN (+13 more)

### Community 49 - "frames.rs"
Cohesion: 0.07
Nodes (43): o(), curve_histogram_and_window(), exclusive_fullscreen(), fps(), frame_csv_line(), FRAMES_CSV_HEADER, histogram(), is_stutter() (+35 more)

### Community 50 - "hotkeys_portal.rs"
Cohesion: 0.10
Nodes (21): bind(), create_session(), DEST, IFACE, listen(), no_bus_means_unavailable(), off_bindings_are_not_requested(), PATH (+13 more)

### Community 51 - "overview.rs"
Cohesion: 0.12
Nodes (27): page_caps(), drives(), customize_presets_and_restore(), editor(), ES, find(), health(), identity() (+19 more)

### Community 52 - "mod.rs (2)"
Cohesion: 0.13
Nodes (25): collect(), decode_dimm(), Dimm, dimm_temps(), DUMP, dumps(), FakeHub, golden_ddr5_matches_cpuz() (+17 more)

### Community 53 - "proc_ops.rs"
Cohesion: 0.12
Nodes (25): acts_on_a_child_process(), check(), class(), connections(), dump(), eco(), end(), err() (+17 more)

### Community 54 - "Telemetry"
Cohesion: 0.09
Nodes (9): publish_throttling(), cli_log(), every_source_keeps_the_contract(), Gpus, lock(), sources(), status(), Telemetry (+1 more)

### Community 55 - "procs.rs"
Cohesion: 0.09
Nodes (29): anti_cheat(), classify(), cpu_percent(), critical(), grouping_and_critical(), HIGH_CPU, HISTORY, Kind (+21 more)

### Community 56 - "display_drm.rs"
Cohesion: 0.10
Nodes (28): best_hz(), card_modes(), CardRes, CONNECTED, connector_name(), connector_names_match_the_kernels(), Crtc, driver_of() (+20 more)

### Community 57 - "dxcore.rs"
Cohesion: 0.12
Nodes (23): compute_adapters(), ComputeAdapter, CORE_COMPUTE, Count, Create, CreateList, GENERIC_ML, GetAdapter (+15 more)

### Community 58 - "details"
Cohesion: 0.13
Nodes (20): RecoveryAction, .EDITABLE, None, Reboot, Restart, RunProgram, action_of(), apply() (+12 more)

### Community 59 - "bottleneck_ui.rs"
Cohesion: 0.10
Nodes (21): Analysis, Confidence, High, Low, Medium, Evidence, fill(), Recommendation (+13 more)

### Community 60 - "fuzz.yml"
Cohesion: 0.06
Nodes (32): MINUTES, name, runs-on, steps, strategy, timeout-minutes, minutes, jobs (+24 more)

### Community 61 - "Cell"
Cohesion: 0.12
Nodes (11): ES, link_status(), link_text(), PT_BR, CoreRow, ES, group_of(), PT_BR (+3 more)

### Community 62 - "software.rs"
Cohesion: 0.24
Nodes (12): after(), collect(), created_secs(), CV, dates(), fields(), mdy(), os_line() (+4 more)

### Community 63 - "gpubench.rs"
Cohesion: 0.11
Nodes (17): Ctl, adapters(), BIG, block_on(), flops(), FLOPS_PER_ITER, FMA, Gpu (+9 more)

### Community 64 - "proc_net.rs"
Cohesion: 0.10
Nodes (25): begin(), inet_diag_message(), KERNEL_NETWORK, KEYWORDS, LAST, nettop_csv(), on_event(), parse_inet_diag() (+17 more)

### Community 65 - "history.rs"
Cohesion: 0.16
Nodes (23): CM_Get_Parent(), DevInfoData, DevPropKey, DIGCF_ALLCLASSES, DIGCF_PRESENT, Guid, pci_devices(), PCI_PROPS (+15 more)

### Community 66 - "tray.rs (2)"
Cohesion: 0.11
Nodes (14): argb(), build(), Imp, items(), menu(), menu_follows_its_state(), MenuState, Row (+6 more)

### Community 67 - "bottleneck.rs"
Cohesion: 0.15
Nodes (47): a_real_presentmon_capture_tells_cpu_bound_loading_from_gpu_bound_play(), a_session_sums_up_time_per_class(), analyze(), analyze_verdicts(), avg(), base(), class(), classify() (+39 more)

### Community 68 - "mod.rs (3)"
Cohesion: 0.15
Nodes (23): Adapter, adapter_nodes(), adapters(), apply_nvml(), backend(), collect(), fill_from_db(), gen_speed() (+15 more)

### Community 69 - "live_sessions.rs"
Cohesion: 0.15
Nodes (20): Session, System, a_mangohud_log_imports_as_a_session(), bars(), COLORS, compare(), sessions_dir(), ES (+12 more)

### Community 70 - "procs.rs (2)"
Cohesion: 0.12
Nodes (7): Collector, collector_builds_rows_and_rates(), Meta, Prev, Sampler, Snapshot, worker()

### Community 71 - "query.rs"
Cohesion: 0.26
Nodes (8): sysctl_str(), sysctl_u64(), build(), cpuinfo_field(), cpuinfo_midr(), implementer(), midr_text(), part()

### Community 72 - "mod.rs (4)"
Cohesion: 0.14
Nodes (23): amd_pstates(), apic_tree(), arm_free_sysctl(), build(), bus_mhz(), Cache, cache_line(), Cluster (+15 more)

### Community 73 - "trayicon.rs"
Cohesion: 0.11
Nodes (21): Tone, Danger, Good, Normal, Warning, placeholder(), Canvas, ctx() (+13 more)

### Community 74 - "Class"
Cohesion: 0.14
Nodes (12): Class, .ALL, Background, Cpu, Disk, Gpu, Inconclusive, Limiter (+4 more)

### Community 75 - "telemetry.rs (2)"
Cohesion: 0.12
Nodes (27): throttle_kind(), AT_LIMIT, c(), CLOCK_DROP, Cond, cond(), Debounce, DEBOUNCE (+19 more)

### Community 76 - "chipset.rs"
Cohesion: 0.13
Nodes (27): amd_chipset(), amd_umc(), chipset_names(), chipset_token(), ChipsetSummary, collect(), decode_umc(), decode_umc_more() (+19 more)

### Community 77 - "Reading"
Cohesion: 0.17
Nodes (13): snap(), Reading, battery(), cpu_extra(), gpu(), gpu_group(), ids(), memory() (+5 more)

### Community 78 - "desktop.rs"
Cohesion: 0.09
Nodes (3): each(), Window, windows()

### Community 79 - "display.rs"
Cohesion: 0.10
Nodes (10): advanced_colour(), Colour, device(), each_monitor(), mode(), modes(), POWER_SAVER, ScreenRect (+2 more)

### Community 80 - "gpu_engines.rs"
Cohesion: 0.05
Nodes (38): array(), LAST, PREV, Query, QUERY, Raw, read(), Residency (+30 more)

### Community 81 - "theme.rs"
Cohesion: 0.16
Nodes (17): apply(), DARK, fonts(), GAP, LIGHT, metric(), PAD_CARD, Palette (+9 more)

### Community 82 - "executor.rs"
Cohesion: 0.14
Nodes (21): audit(), audit_log(), audit_path(), bundled_pawnio(), Done, Cancelled, Failed, Ok (+13 more)

### Community 83 - "amd_pm.rs"
Cohesion: 0.16
Nodes (27): APU, cells(), close(), Core, Cores, cores(), decode(), enabled_slots() (+19 more)

### Community 84 - "mod.rs (5)"
Cohesion: 0.07
Nodes (8): Bdf, cfg_path(), Caps, detect(), Hw, NoHw, none(), PAUSED

### Community 85 - "users.rs"
Cohesion: 0.09
Nodes (5): query_wts_str(), send_message(), sessions(), state_label(), to_wide()

### Community 86 - "adapters"
Cohesion: 0.14
Nodes (17): Adapter, adapters(), driver_info(), drvinfo(), fmt_mac(), IfAddrs, ifaddrs(), node() (+9 more)

### Community 87 - "large_pages.rs"
Cohesion: 0.08
Nodes (4): alloc(), HUGE, lock_memory_privilege(), Region

### Community 88 - "Column"
Cohesion: 0.19
Nodes (15): b64(), device(), list(), next_tag(), parse_plist(), plist(), Pv, Arr (+7 more)

### Community 89 - "core/src/robustness.rs"
Cohesion: 0.12
Nodes (15): corrupt_sessions_stay_bounded(), edid_blocks(), FIXTURE, hardware_and_mangohud_logs(), layout_files(), n(), output_lines_ignore_chunking(), pci_config_space() (+7 more)

### Community 90 - "report.rs"
Cohesion: 0.13
Nodes (31): esc(), fields(), find(), find_section(), g(), groups(), gv(), hex() (+23 more)

### Community 91 - "overlay.rs"
Cohesion: 0.09
Nodes (4): apply(), each(), Find, Style

### Community 92 - "Unit"
Cohesion: 0.09
Nodes (18): Csv, csv_header_rows_and_new_columns(), Series, Unit, Ampere, Celsius, Count, Mbps (+10 more)

### Community 93 - "soak.py"
Cohesion: 0.14
Nodes (15): analyze(), frame_trend(), _linux_sampler(), _macos_sampler(), make_sampler(), profile(), report(), run_gui() (+7 more)

### Community 94 - "nvml.rs"
Cohesion: 0.17
Nodes (13): db_apple(), cstr(), Dev, FanRpm, get(), Info, load(), Memory (+5 more)

### Community 95 - "live.rs"
Cohesion: 0.12
Nodes (9): HotkeyAction, .ALL, Capture, CycleStyle, EditMode, Marker, ToggleOverlay, Hotkeys (+1 more)

### Community 96 - "F"
Cohesion: 0.12
Nodes (15): description, label, placeholder, render, value, body, attributes, id (+7 more)

### Community 97 - "windows.rs (4)"
Cohesion: 0.14
Nodes (8): fn_name(), live_dump(), PawnIo, pch_decode(), pch_raw(), pm_cells(), unpack_f32(), on_cpu()

### Community 98 - "devices.rs"
Cohesion: 0.17
Nodes (20): adapter(), battery(), device_card(), drive(), ES, gb(), Health, health() (+12 more)

### Community 99 - "pins.rs"
Cohesion: 0.22
Nodes (17): cache_id(), card_id(), card_text(), current_card(), enter_card(), ES, is_pinned(), latest() (+9 more)

### Community 100 - "screens.rs"
Cohesion: 0.16
Nodes (21): nearest_anchor(), A, B, C, choose(), choosing_where_to_draw(), locate(), locate_and_place_round_trip() (+13 more)

### Community 102 - "diskbench.rs"
Cohesion: 0.14
Nodes (16): write_at(), a_tiny_run_measures_and_cleans_up(), a_tiny_sustained_write_cleans_up(), bench(), phase(), rates(), rates_math(), Run (+8 more)

### Community 103 - "query.rs"
Cohesion: 0.26
Nodes (15): half_typed_and_odd_input_never_fails(), hit(), matches(), number(), parse(), plain(), row(), syntax() (+7 more)

### Community 104 - "sensors/windows.rs"
Cohesion: 0.22
Nodes (13): battery(), decode(), monitors(), plist_int(), temp_keys(), temp_label(), value(), collect() (+5 more)

### Community 105 - "zeez-live.md"
Cohesion: 0.06
Nodes (40): ADR 0006: Dependencies, rusqlite (bundled SQLite), windows-sys, ADR 0013: glow renderer with wgpu fallback, glow (OpenGL) renderer, wgpu fallback (CZ_RENDERER=wgpu, WARP), Now card, live readings, session analysis, Benchmark principles (safe, reproducible, 1000 = reference 7900X) (+32 more)

### Community 106 - "ci.yml"
Cohesion: 0.16
Nodes (24): name, runs-on, steps, name, runs-on, steps, steps, GH_TOKEN (+16 more)

### Community 107 - "battery.rs"
Cohesion: 0.13
Nodes (20): batteries(), Battery, chemistry_name(), collect(), GUID_DEVICE_BATTERY, info(), IOCTL_BATTERY_QUERY_INFORMATION, IOCTL_BATTERY_QUERY_STATUS (+12 more)

### Community 108 - "usb.rs"
Cohesion: 0.13
Nodes (21): CM_Get_Parent(), controller(), parent(), reg_prop(), reg_str(), reg_strs(), reg_u32(), class_from_compat() (+13 more)

### Community 109 - "startup.rs (2)"
Cohesion: 0.11
Nodes (20): extract_exe_path(), StartupRow, StartupSource, CommonFolder, HkcuRun, HkcuRunOnce, HklmRun, HklmRun32 (+12 more)

### Community 110 - "a11y.rs"
Cohesion: 0.10
Nodes (26): at_every_zoom(), beyond(), bottleneck_sample(), chart_colours_meet_the_graphics_minimum(), charts_and_bars_say_what_they_show(), class_colours_are_distinct_and_readable(), collisions(), contrast() (+18 more)

### Community 111 - "nightly.yml"
Cohesion: 0.09
Nodes (21): minutes, jobs, soak, default, description, name, on, schedule (+13 more)

### Community 112 - "Page"
Cohesion: 0.11
Nodes (18): NAV, NAV_BOTTOM, Page, About, Benchmarks, Devices, Graphics, Live (+10 more)

### Community 113 - "details"
Cohesion: 0.22
Nodes (10): diagnostics_bundle(), info(), install_panic_hook(), LOCK, MAX, path(), warn(), write() (+2 more)

### Community 114 - "Period"
Cohesion: 0.12
Nodes (11): Cmd, Clear, Flush, Query, Period, All, Days30, Days7 (+3 more)

### Community 115 - "tray.rs"
Cohesion: 0.06
Nodes (24): Click, App, Mini, Action, Lock, Mini, Pause, Quit (+16 more)

### Community 116 - "availability.rs"
Cohesion: 0.07
Nodes (41): Capability, .ALL, BoardSensors, ChipsetTemp, CpuSensors, MemoryTimings, Spd, caps() (+33 more)

### Community 117 - "live.rs (2)"
Cohesion: 0.19
Nodes (13): EDGE, exe_name(), hotkey_conflicts(), hotkeys_normalize_and_conflict(), normalize_keys(), Overlay, portal_request_path(), portal_trigger() (+5 more)

### Community 118 - "PresentMon bundled sidecar (ETW)"
Cohesion: 0.14
Nodes (13): ADR 0001: ZeeZ Live is the brand, ZeeZ Live (Overlay, Capture, Reports), ADR 0003: FPS on Windows from PresentMon, PresentMon bundled sidecar (ETW), ADR 0004: FPS on Linux from MangoHud, MangoHud integration (detect, install, config, logs), SetLayeredWindowAttributes click-through fix, global-hotkey crate (RegisterHotKey/Carbon/XGrabKey) (+5 more)

### Community 119 - "Ctx"
Cohesion: 0.09
Nodes (17): rdtsc(), active_from_deltas(), active_mhz(), add_chain_mhz(), aperf_mperf(), aperf_mperf_regs(), chain(), core_mhz() (+9 more)

### Community 120 - "hashmap"
Cohesion: 0.32
Nodes (3): acpi_table(), hwid_field(), list()

### Community 121 - "Column"
Cohesion: 0.21
Nodes (7): cell_text(), Column, Column<'a, R>, fit(), fit_width(), Header, Table

### Community 122 - "table.rs"
Cohesion: 0.23
Nodes (18): cols(), columns_move_and_keep_their_order(), ES, filter_sort_and_group(), header_resizes_moves_and_sorts_with_the_mouse(), MIN_W, move_column(), P (+10 more)

### Community 123 - "memtest.rs"
Cohesion: 0.17
Nodes (9): every_pattern_catches_a_flipped_bit(), fill(), MemTest, PATTERNS, runs_and_stops(), seed(), value(), verify() (+1 more)

### Community 124 - "CHANGELOG.md"
Cohesion: 0.09
Nodes (45): ADR 0009: releases stay unsigned for now, ADR 0013: glow with wgpu fallback, 1.1.0 roadmap (docs/v1.1), Security review 2026-11-09, 1.0 plan (docs/v1), ADR 0009: Releases stay unsigned, SHA256SUMS verification for unsigned releases, ADR 0011: Workspace split by layer (+37 more)

### Community 125 - "live_frames.rs"
Cohesion: 0.07
Nodes (52): Frame, App, apps(), begin_capture(), CAPTURE_MAX, Demo, demo(), due() (+44 more)

### Community 126 - "R"
Cohesion: 0.28
Nodes (8): R, Item, Group, Row, shown(), State, Table<'a, R>, to_tsv()

### Community 127 - ".render_table"
Cohesion: 0.21
Nodes (18): add(), dates(), Entry, entry(), join(), load(), load_from(), memory_line() (+10 more)

### Community 128 - "core/src/services.rs"
Cohesion: 0.10
Nodes (30): small_text_parsers(), a_dacl_that_refuses_administrators(), CHANGE_CONFIG, dependency_chain_is_cycle_safe_and_filters_work(), is_protected(), is_third_party(), matches_filter(), parse_systemd_secs() (+22 more)

### Community 129 - "superio.rs"
Cohesion: 0.28
Nodes (12): apply(), fintek(), Io, ite(), nct6687_like_the_dump(), nct_ec(), nct_new(), nct_old() (+4 more)

### Community 130 - "Source"
Cohesion: 0.06
Nodes (15): actor(), check_contract(), Class, Fast, Slow, Fake, Source, spawn() (+7 more)

### Community 131 - "layout_view.rs"
Cohesion: 0.07
Nodes (34): cpu_suite_modes(), ctx(), memory_and_latency_views(), storage_view_warns_before_writing(), themed(), ES, KINDS, layout_editor_edits() (+26 more)

### Community 132 - "cpu_os.rs"
Cohesion: 0.15
Nodes (11): CODE_INTEGRITY, KVA_SHADOW, NtQuerySystemInformation(), Power, power(), query(), read(), SPECULATION_CONTROL (+3 more)

### Community 133 - "linux.rs (2)"
Cohesion: 0.14
Nodes (19): raw(), read_trim(), read_u64(), adapter(), cards(), dpm(), freq(), hwmon_live() (+11 more)

### Community 134 - "Batteries"
Cohesion: 0.12
Nodes (3): Batteries, Drives, MeasuredClock

### Community 135 - "health.rs"
Cohesion: 0.26
Nodes (16): a_healthy_build_has_nothing_to_say(), check(), Display, each_mistake(), Facts, Finding, finding(), good() (+8 more)

### Community 136 - ".render_table"
Cohesion: 0.22
Nodes (14): CtxCmd, Affinity, Dump, Eco, End, Inspect, Priority, RestartShell (+6 more)

### Community 137 - "src/health.rs"
Cohesion: 0.26
Nodes (5): check(), facts(), fields_named(), integrated(), mhz_in()

### Community 138 - "Map"
Cohesion: 0.17
Nodes (13): b(), Board, board_lookup(), board_map(), BOARDS, EC_GENERIC, ITE_GENERIC, ITE_OLD_GENERIC (+5 more)

### Community 139 - "gui/tools.rs"
Cohesion: 0.11
Nodes (19): Tools, changes(), CHART_BOTTOM, CHART_HEIGHT, CHART_LEFT, CHART_LINE, Compare, diff_as_text() (+11 more)

### Community 140 - "Action"
Cohesion: 0.33
Nodes (11): ascii(), ata(), ata_fields_and_dump(), ata_text(), hex_dump(), kelvin(), le16(), le32() (+3 more)

### Community 141 - "V3"
Cohesion: 0.38
Nodes (11): add(), dot(), hit(), LIGHT, mul(), norm(), ray(), SCENE (+3 more)

### Community 142 - "src/health.rs"
Cohesion: 0.24
Nodes (9): cmd(), logs(), adapter(), apple_nm(), hex_id(), ioreg_edids(), metal(), probe() (+1 more)

### Community 143 - "FEATURES.md (CPU-Z 2.15 clean-room spec)"
Cohesion: 0.05
Nodes (42): ADR 0012: no code injection, ever, ADR 0014: no hardware balance bar in 1.0, ADR 0015: Wayland shortcuts through the GlobalShortcuts portal, Anti-cheat safe mode, App history (SQLite store), Bottleneck detector (What limits my FPS?), Build health checks, Per-action elevation (one-shot elevated copy, closed operation list) (+34 more)

### Community 144 - "ServiceOp"
Cohesion: 0.33
Nodes (5): ServiceAccount, .ALL, LocalService, LocalSystem, NetworkService

### Community 145 - "CPU ZeeZ 1.1.0 plan"
Cohesion: 0.06
Nodes (39): AccessKit names, states and values for painted widgets, Accessibility (S3-03), Keyboard navigation and table shortcuts, Scale test 75-200% (at_every_zoom), Screen readers cannot read painted tables and charts, WCAG AA contrast check from theme palette, Read-only helper service (after 1.0), ADR 0007: One 1.0.0 release in gated stages (+31 more)

### Community 146 - "CpuZeeZ 1.0 Plan (hub)"
Cohesion: 0.05
Nodes (40): ADR 0008: Planning documents are in English, Command palette (Ctrl+K), Shared page patterns (detail levels, Ctrl+K palette, banners, pin, units), rusqlite (bundled SQLite), ADRs in docs/adr, Contract test suites and fakes, core crate (pure logic, no OS/GUI), Definition of done (+32 more)

### Community 147 - "pkgver.py"
Cohesion: 0.21
Nodes (5): _test(), versions(), pick(), selftest(), wanted()

### Community 148 - ".show_ui"
Cohesion: 0.31
Nodes (4): app_history_tab_shows_the_view(), format_duration_hms(), render_daily_chart(), State

### Community 149 - "core/src/pci/mod.rs"
Cohesion: 0.38
Nodes (8): class_desc(), Ids, ids_parse_and_labels(), mcfg(), mcfg_base(), parse_ids(), vendor_label(), vendor_short()

### Community 150 - ".groups"
Cohesion: 0.43
Nodes (4): GetProcAddress(), Lib, LoadLibraryW(), sym!

### Community 151 - "raster.py"
Cohesion: 0.20
Nodes (8): dist_seg(), in_poly(), in_rrect(), on_arc(), png(), render(), render_png(), sample()

### Community 153 - ".render_table"
Cohesion: 0.19
Nodes (11): windows-sys crate, D10 Hand-written FFI, D1 APERF/MPERF clock busy-spin, D2 rdmsr spawns thread per read, D3 SMART probe inside sampler, D5 series() clones history; duplicate GUI sampler, Diagnosis of 0.3.0 (D1-D10), Metric subscriptions and rate classes (+3 more)

### Community 154 - "direct_io.rs"
Cohesion: 0.18
Nodes (7): ALIGN, Buffer, FILE_FLAG_NO_BUFFERING, FILE_FLAG_WRITE_THROUGH, open(), read_at(), uncached_round_trip()

### Community 155 - "smc.rs"
Cohesion: 0.18
Nodes (18): available(), call(), conn(), IOConnectCallStructMethod(), IOObjectRelease(), IOServiceGetMatchingService(), IOServiceMatching(), IOServiceOpen() (+10 more)

### Community 156 - "tempdir.rs"
Cohesion: 0.32
Nodes (14): base_is_safe(), base_validation_follows_symlinks_in_the_path(), base_validation_rejects_swappable_ancestors(), base_validation_rejects_swappable_symlinks(), base_validation_requires_sticky_when_shared(), create(), create_exclusive_private(), private_and_ours() (+6 more)

### Community 157 - "Anchor"
Cohesion: 0.11
Nodes (23): Anchor, .ALL, Bottom, BottomLeft, BottomRight, Center, Left, Right (+15 more)

### Community 158 - "queryable"
Cohesion: 0.18
Nodes (10): Channel, .ALL, Alpha, Beta, Snapshot, Stable, cli(), downloads_and_verifies_latest_release() (+2 more)

### Community 159 - "dependabot.yml"
Cohesion: 0.13
Nodes (14): patterns, prefix, patterns, actions, egui, interval, updates, commit-message (+6 more)

### Community 160 - "ci.yml (2)"
Cohesion: 0.13
Nodes (16): container, name, runs-on, strategy, name, os, shots, distros (+8 more)

### Community 161 - "superio.rs (2)"
Cohesion: 0.28
Nodes (9): cfg_rd(), cfg_rd16(), cfg_wr(), Chip, detect(), Fake, finish(), probe() (+1 more)

### Community 162 - "StartupType"
Cohesion: 0.20
Nodes (9): StartupType, Automatic, AutomaticDelayed, Disabled, .EDITABLE, Manual, Masked, Static (+1 more)

### Community 163 - "engines.rs"
Cohesion: 0.31
Nodes (11): AdapterEngines, aggregate(), engine_label(), labels(), parse(), per_process(), per_process_is_the_busiest_engine(), per_process_memory() (+3 more)

### Community 164 - "windows.rs (5)"
Cohesion: 0.24
Nodes (8): open_never_panics(), PawnSmbus, Xfer, xfer_args(), ReadByte, ReadWord, SendByte, WriteByte

### Community 165 - "Id"
Cohesion: 0.20
Nodes (6): egui::InnerResponse<T>, FINTEK, Id, ITE, lookup(), WINBOND_NUVOTON

### Community 166 - "jobs"
Cohesion: 0.10
Nodes (20): env, CARGO_TERM_COLOR, jobs, msi-releases, msi-upgrade, msrv, visual, name (+12 more)

### Community 168 - "memplus.rs"
Cohesion: 0.33
Nodes (13): chain(), chain_is_one_cycle(), gups(), latency(), LINE, pin(), Plan, run() (+5 more)

### Community 169 - "timers.rs"
Cohesion: 0.26
Nodes (13): ACPI_PM_HZ, acpi_timer(), arm_cntfrq(), fadt(), fadt_pm_timer(), hz_text(), measure_pm(), perf_timer() (+5 more)

### Community 170 - "gpu/macos.rs"
Cohesion: 0.27
Nodes (9): measure(), pair(), ROUNDS, statistical(), statistical_mode_measures_unpinned(), summary(), summary_reads_the_upper_triangle(), two_cores_answer() (+1 more)

### Community 171 - "serialize"
Cohesion: 0.09
Nodes (16): defaults(), fires_only_when_the_whole_window_is_over(), firing(), pts(), Rule, rules_load_from_older_settings(), Clock, Auto (+8 more)

### Community 172 - "net_io.rs"
Cohesion: 0.17
Nodes (4): FILTER, HARDWARE, read(), RTM_IFINFO2

### Community 173 - "Bottleneck Detector Design"
Cohesion: 0.12
Nodes (19): Bottleneck detector (measured evidence only), Build health checks on Overview, ADR 0014: No hardware balance bar in 1.0, Configuration checks / Build health (single channel, XMP off, PCIe, ReBAR...), Bottleneck Detector Design, Evidence-tied recommendations (no snake oil), Heuristic fallback without GPU Busy (lower confidence), Limiters (V-Sync, VRR, FPS cap, compositor) (+11 more)

### Community 174 - "best_hz"
Cohesion: 0.46
Nodes (5): compress(), hex(), Sha256, sha256_fips_vectors(), sha256_of()

### Community 175 - "node"
Cohesion: 0.29
Nodes (9): bus_name(), controller_class(), controller_line(), formats(), gib(), link_line(), node(), pci_controller() (+1 more)

### Community 176 - "probe"
Cohesion: 0.12
Nodes (24): ascii(), ata_cmd(), ata_cmd_lba(), ata_smart(), ata_smart_nodata(), cstr(), find_key(), Handle (+16 more)

### Community 177 - "fixture.rs"
Cohesion: 0.14
Nodes (15): a_row_pins_under_its_card(), detail_levels_filter_cards(), adapters(), app(), cpu(), ctx(), drives(), every_hardware_page_draws_from_the_fixtures() (+7 more)

### Community 178 - "ServiceState"
Cohesion: 0.25
Nodes (7): ServiceState, Paused, Running, StartPending, Stopped, StopPending, Unknown

### Community 179 - "ToolView"
Cohesion: 0.32
Nodes (4): cpu_load(), parse_proc_stat(), throttle_counts(), ticks()

### Community 180 - "StartupType"
Cohesion: 0.13
Nodes (13): Risk, Critical, Irreversible, Normal, ServiceOp, Pause, Restart, SetAccount (+5 more)

### Community 181 - "diskio.rs"
Cohesion: 0.30
Nodes (11): a_reset_counter_is_no_activity_not_garbage(), Counters, DISKSTATS, diskstats_keeps_whole_disks_only(), diskstats_units(), is_whole_disk(), parse_diskstats(), Rates (+3 more)

### Community 182 - "src/health.rs"
Cohesion: 0.19
Nodes (8): Size, Decimal, Iec, Windows, Units, .DEFAULT, CURRENT, set()

### Community 183 - "gui/screens.rs"
Cohesion: 0.29
Nodes (3): game_centre(), label(), points()

### Community 184 - "storage-and-cleanup.md"
Cohesion: 0.10
Nodes (19): ADR 0002: Privileges, Restart as administrator button, ADR 0005: Task Manager under one System entry, System hub (Processes, Performance, App history, Startup, Users, Services), BOARDS Super I/O map (src/sensors/superio.rs), Ideas for later versions, Storage audit and safe cleanup (idea), Duplicate file finder (size, partial hash, SHA-256) (+11 more)

### Community 186 - "ROADMAP.md"
Cohesion: 0.18
Nodes (15): Anti-cheat match test (spike X08), 1.0.0-beta.1 manual test checklist, macOS validation checklist (S0), CHANGELOG 1.0.0 Consolidated (pointer), 1.0 release criteria, CpuZeeZ 1.0 Detailed Roadmap, Sampler measurements on Ryzen 9 7900X, S0 Foundation (+7 more)

### Community 187 - "make_icons.py"
Cohesion: 0.24
Nodes (6): walk(), imports(), icns(), ico(), main(), render()

### Community 188 - "superio.rs (4)"
Cohesion: 0.17
Nodes (12): Family, Fintek, Ite, NctEc, NctNew, NctOld, MV10_6, MV11 (+4 more)

### Community 189 - "Graphify neste projeto"
Cohesion: 0.33
Nodes (5): Graphify neste projeto, `.graphifyignore`, Notas, Setup (uma vez por projeto), Uso no dia a dia

### Community 190 - "Clock"
Cohesion: 0.31
Nodes (9): StartupOp, SetEnabled, apply(), enum_reg_strings(), query(), read_approved_map(), read_fw_post_time(), RegKey (+1 more)

### Community 191 - "actions.rs (2)"
Cohesion: 0.18
Nodes (8): Priority, AboveNormal, .ALL, BelowNormal, High, Idle, Normal, Realtime

### Community 192 - "StartupRow"
Cohesion: 0.33
Nodes (9): entry_point_version(), ioreg_hex(), synth(), truncated_and_uuid(), uuid_bytes(), apple_table(), fetch(), sysfs_fallback() (+1 more)

### Community 193 - "Op"
Cohesion: 0.29
Nodes (7): Op, Contains, Eq, Ge, Gt, Le, Lt

### Community 194 - "layout.rs (2)"
Cohesion: 0.18
Nodes (10): Style, .ALL, Bar, Graph, Number, Ring, Theme, Dark (+2 more)

### Community 195 - "Preset"
Cohesion: 0.29
Nodes (7): Preset, .ALL, Essential, Everything, Gaming, LeakHunting, Security

### Community 197 - "encryption.rs"
Cohesion: 0.31
Nodes (6): crypt(), DONE, read(), STARTED, Status, status()

### Community 198 - "Period"
Cohesion: 0.70
Nodes (4): imported_reports(), n(), settings_from_any_version(), update_answers()

### Community 199 - "Id"
Cohesion: 0.29
Nodes (7): SysTab, AppHistory, Performance, Processes, Services, Startup, Users

### Community 200 - "steps"
Cohesion: 0.40
Nodes (5): container, name, runs-on, steps, arch-pkg

### Community 203 - "src/soak.rs"
Cohesion: 0.23
Nodes (9): App, enabled(), frame(), NAME, overlay_frame(), overlay_only(), PID, start() (+1 more)

### Community 204 - "imp"
Cohesion: 0.50
Nodes (5): Conn, connections(), guid_str(), imp(), le32()

### Community 205 - "steps"
Cohesion: 0.50
Nodes (4): macos-pkg, name, runs-on, steps

### Community 206 - "db.rs"
Cohesion: 0.27
Nodes (7): AMD, INTEL, lookup(), NV, s(), Spec, TABLE

### Community 207 - "src/smbios.rs"
Cohesion: 0.08
Nodes (23): cluster_cpus(), find_agesa(), rom_agesa(), section(), uefi(), cppc(), intel_limits(), Mode (+15 more)

### Community 208 - "unsafe_ratchet.py"
Cohesion: 0.25
Nodes (4): count(), selftest(), tree(), verdict()

### Community 209 - "startup.rs (4)"
Cohesion: 0.36
Nodes (9): attr_value(), decode_startup_approved(), encode_startup_approved(), filetime_to_ymd(), impact_thresholds_and_wdi_xml_parser(), parse_wdi_startup_xml(), startup_approved_round_trip_and_filetime_decoding(), tag_u64() (+1 more)

### Community 210 - "instant"
Cohesion: 0.50
Nodes (4): Direction, Grid, Horizontal, Vertical

### Community 211 - "src/pci/mod.rs"
Cohesion: 0.30
Nodes (7): collect(), devices(), ecam_base(), enumerate(), ids(), ids_name(), register_spaces()

### Community 212 - "App"
Cohesion: 0.15
Nodes (3): App, Graphics, Minis

### Community 213 - "shotdiff.py"
Cohesion: 0.29
Nodes (4): compare(), diff(), read_png(), _test()

### Community 214 - "parse_airport"
Cohesion: 0.50
Nodes (3): macos_tools(), parse_airport(), parse_hardware_ports()

### Community 215 - "Widget"
Cohesion: 0.22
Nodes (9): Kind, Bar, Clock, Header, LabelValue, Separator, Sparkline, Throttle (+1 more)

### Community 217 - "run"
Cohesion: 0.27
Nodes (4): App, app_icon(), decode_png(), run()

### Community 219 - "ci.yml (4)"
Cohesion: 0.40
Nodes (5): on, pull_request, push, workflow_dispatch, branches

### Community 222 - "instant"
Cohesion: 0.17
Nodes (12): amd_cppc_highest(), cppc_ranks(), intel_hwp_highest(), intel_limits_decode(), intel_pkg_limits(), Limit, ranks(), Composition (+4 more)

### Community 224 - "small_text_parsers"
Cohesion: 0.43
Nodes (7): Counters, NET_DEV, parse_proc_net_dev(), proc_net_dev(), Rates, rates(), rates_in_megabits()

### Community 228 - "ci.yml (5)"
Cohesion: 0.17
Nodes (12): name, runs-on, steps, headless, with, components, fetch-depth, if-no-files-found (+4 more)

### Community 229 - "LimiterKind"
Cohesion: 0.40
Nodes (5): LimiterKind, Compositor, FpsCap, Menu, VSync

### Community 230 - "macos.rs (4)"
Cohesion: 0.54
Nodes (6): fill(), memory_type(), memory_without_smbios(), partial_and_unavailable_data(), preserves_smbios_and_handles_nested_banks(), probe()

### Community 231 - ".graphics_tab"
Cohesion: 0.33
Nodes (6): DetailTab, Network, Overview, Security, Threads, Windows

### Community 237 - "gui"
Cohesion: 0.08
Nodes (15): BenchView, Cpu, Gpu, History, Latency, Memory, Storage, Stress (+7 more)

### Community 239 - "Monitor"
Cohesion: 0.15
Nodes (3): Monitor, tray_pictures(), Shown

### Community 240 - "Theme"
Cohesion: 0.15
Nodes (14): alerts(), ES, lang_of(), page(), privacy(), PT_BR, row(), system_settings() (+6 more)

### Community 245 - "Step"
Cohesion: 0.12
Nodes (17): App, AutoShots, request(), Shot, Bmp, Clipboard, SHOT, Png (+9 more)

### Community 246 - "page_tests.rs"
Cohesion: 0.14
Nodes (24): Tab, Bottleneck, Live, Overlay, Sessions, about_page(), copy_and_pin(), detail_switch() (+16 more)

### Community 247 - "lib.rs"
Cohesion: 0.50
Nodes (3): DATA_DIR_NAME, DISPLAY_NAME, NAME

### Community 249 - ".mainboard_more"
Cohesion: 0.05
Nodes (39): BASIC, Level, Advanced, .ALL, Basic, Expert, level_of(), levels() (+31 more)

### Community 250 - "Cargo.toml"
Cohesion: 0.67
Nodes (4): cpuzeez, cpuzeez-core, cpuzeez-fuzz, cpuzeez-platform

### Community 269 - "WindowOp"
Cohesion: 0.33
Nodes (6): window_op(), WindowOp, Close, Maximize, Minimize, SwitchTo

### Community 271 - "Impact"
Cohesion: 0.29
Nodes (5): Impact, High, Low, Medium, NotMeasured

## Knowledge Gaps
- **504 isolated node(s):** `Msr`, `I2cDev`, `Idle`, `BelowNormal`, `Normal` (+499 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 1917 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **24 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `Ctx` connect `src/smbios.rs` to `sensors.rs`, `Sub`, `Source`, `System Performance Charts`, `x86.rs`, `Batteries`, `live_sessions.rs`, `CPU Benchmark Runner`, `src/health.rs`, `Memory Benchmark`, `Storage Drives Probe`, `network.rs`, `Ctx`, `CPU Sensors and Power Limits`, `system_users.rs`, `mod.rs`, `tr`, `timers.rs`, `peripherals.rs`, `f`, `diff.rs`, `probe`, `overview.rs`, `mod.rs (2)`, `Telemetry`, `gui/screens.rs`, `bottleneck_ui.rs`, `software.rs`, `mod.rs (3)`, `mod.rs (4)`, `telemetry.rs (2)`, `chipset.rs`, `Reading`, `src/pci/mod.rs`, `App`, `adapters`, `run`, `devices.rs`, `battery.rs`, `usb.rs`, `Monitor`, `Page`, `availability.rs`, `Ctx`, `.mainboard_more`, `.render_table`?**
  _High betweenness centrality (0.074) - this node is a cross-community bridge._
- **Are the 9 inferred relationships involving `Ctx` (e.g. with `cli()` and `no_data()`) actually correct?**
  _`Ctx` has 9 INFERRED edges - model-reasoned connections that need verification._
- **What connects `Msr`, `I2cDev`, `Idle` to the rest of the system?**
  _504 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `sensors.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.05502392344497608 - nodes in this community are weakly interconnected._
- **Why does `tr()` connect `tr` to `sensors.rs`, `Sub`, `intel.rs`, `layout_view.rs`, `System Performance Charts`, `live_sessions.rs`, `.render_table`, `gui/tools.rs`, `system_history.rs`, `.show_ui`, `system.rs`, `system_users.rs`, `data.rs`, `palette.rs`, `system_services.rs`, `diff.rs`, `split.rs`, `overview.rs`, `bottleneck_ui.rs`, `tray.rs (2)`, `live_sessions.rs`, `run`, `live.rs`, `devices.rs`, `pins.rs`, `Monitor`, `Page`, `Theme`, `tray.rs`, `Step`, `.mainboard_more`, `table.rs`, `R`?**
  _High betweenness centrality (0.050) - this node is a cross-community bridge._
- **Are the 150 inferred relationships involving `tr()` (e.g. with `.banner()` and `.frame()`) actually correct?**
  _`tr()` has 150 INFERRED edges - model-reasoned connections that need verification._
- **Should `Sub` be split into smaller, more focused modules?**
  _Cohesion score 0.04634495938843765 - nodes in this community are weakly interconnected._