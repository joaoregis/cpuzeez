# Graph Report - cpuzeez  (2026-11-13)

## Corpus Check
- 288 files · ~410,346 words
- Verdict: corpus is large enough that graph structure adds value.
- Unclassified: 38 file(s) not represented in the graph (top: (none) 22, .csv 5, .toml 4)

## Summary
- 6315 nodes · 14784 edges · 261 communities (228 shown, 33 thin omitted)
- Extraction: 92% EXTRACTED · 8% INFERRED · 0% AMBIGUOUS · INFERRED: 1158 edges (avg confidence: 0.81)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `cc7ec993`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- sensors.rs
- egui
- x86.rs
- details
- System Performance Charts
- Row
- palette
- CPU Benchmark Runner
- SPD Decoding
- Memory Benchmark
- Storage Drives Probe
- core/src/smbios.rs
- system_history.rs
- gui/mod.rs
- Metric Engine
- Network Adapters
- Ctx
- with
- CPU Sensors and Power Limits
- CPU Model Database
- system.rs
- update.rs
- Process Snapshot Platform
- Super I/O Sensor Chips
- actions.rs
- Community 26
- Windows GPU Probe
- CPU Test Suite
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
- os.rs
- diff.rs
- split.rs
- frames.rs
- hotkeys_portal.rs
- overview.rs
- mod.rs (2)
- proc_ops.rs
- intel.rs
- procs.rs
- display_drm.rs
- Action
- gpubench.rs
- welcome.rs
- fuzz.yml
- mini.rs
- software.rs
- Unit
- proc_net.rs
- Window
- tray.rs (2)
- bottleneck.rs
- mod.rs (3)
- live_sessions.rs
- procs.rs (2)
- trf
- mod.rs (4)
- trayicon.rs
- windows.rs (2)
- telemetry.rs (2)
- chipset.rs
- telemetry.rs (3)
- desktop.rs
- display.rs
- gpu_engines.rs
- page_tests.rs
- executor.rs
- amd_pm.rs
- mod.rs (5)
- users.rs
- macos.rs
- large_pages.rs
- i18n.rs
- core/src/robustness.rs
- report.rs
- overlay.rs
- split.rs
- soak.py
- nvml.rs
- live.rs
- serialize
- windows.rs (4)
- devices.rs
- pins.rs
- screens.rs
- diskbench.rs
- live_mangohud.rs
- session.rs
- zeez-live.md
- ci.yml
- battery.rs
- usb.rs
- startup.rs (2)
- a11y.rs
- nightly.yml
- npu.rs
- BenchView
- firmware.rs
- availability.rs
- live.rs (2)
- PresentMon bundled sidecar (ETW)
- clocks.rs
- pci/windows.rs
- cstates.rs
- memory.rs
- memtest.rs
- CHANGELOG.md
- live_frames.rs
- detail.rs
- history.rs
- core/src/services.rs
- superio.rs
- Source
- layout_view.rs
- cpu_os.rs
- linux.rs (2)
- State
- health.rs
- src/smbios.rs
- body
- table.rs
- gui/tools.rs
- .render_table
- Page
- gpu/macos.rs
- FEATURES.md (CPU-Z 2.15 clean-room spec)
- actions.rs
- CPU ZeeZ 1.1.0 plan
- CpuZeeZ 1.0 Plan (hub)
- pkgver.py
- Batteries
- Map
- Lib
- raster.py
- PdhCollectQueryData
- Metric Engine
- direct_io.rs
- hwmon.rs
- tempdir.rs
- Anchor
- Aes
- dependabot.yml
- ci.yml (2)
- superio.rs (2)
- bottleneck.rs (3)
- engines.rs
- windows.rs (5)
- arm.rs
- ci.yml (3)
- start
- memplus.rs
- timers.rs
- gpu/macos.rs
- Units
- net_io.rs
- Bottleneck Detector Design
- rate
- .ensure
- Handle
- fixture.rs
- I2cDev
- telemetry/os.rs
- StartupType
- diskio.rs
- .tick
- best_hz
- storage-and-cleanup.md
- StartupRow
- ROADMAP.md
- make_icons.py
- superio.rs (4)
- Graphify neste projeto
- bench_tab.rs
- actions.rs (2)
- node
- edid.rs
- layout.rs (2)
- limits.rs
- ProcessOp
- encryption.rs
- Preset
- n
- RecoveryAction
- CLAUDE.md
- benchkit.rs
- soak.rs
- ServiceState
- ServiceFilter
- db.rs
- f
- unsafe_ratchet.py
- startup.rs (4)
- queryable
- gui/screens.rs
- Status
- shotdiff.py
- Status
- Kind
- .take_widths
- of
- mod.rs (9)
- ci.yml (4)
- DetailTab
- detector.rs
- Gear
- Act
- netio.rs
- .tick
- ci.yml (5)
- macos.rs (4)
- Monitor
- Theme
- ci.yml (6)
- SysTab
- bottleneck.rs (4)
- .take_screenshot
- lib.rs
- Cargo.toml
- fetch_pawnio.sh
- ci.yml (7)
- CHANGELOG.md (3)
- WindowOp
- lint_workflows.py
- Impact

## God Nodes (most connected - your core abstractions)
1. `Ctx` - 222 edges
2. `tr()` - 188 edges
3. `palette()` - 114 edges
4. `trf()` - 86 edges
5. `()` - 64 edges
6. `card()` - 63 edges
7. `Series` - 51 edges
8. `V` - 40 edges
9. `Unit` - 39 edges
10. `Telemetry` - 38 edges

## Surprising Connections (you probably didn't know these)
- `Release notes template` --semantically_similar_to--> `Unsigned releases and SHA256SUMS verification`  [INFERRED] [semantically similar]
  packaging/RELEASE_NOTES.md → README.md
- `Test fixtures README (S0-T04)` --semantically_similar_to--> `CPU-Z DDR5 TeamGroup SPD dump (test data)`  [INFERRED] [semantically similar]
  crates/core/fixtures/README.md → src/spd/testdata/cpuz_ddr5_teamgroup.txt
- `tsc_hz()` --calls--> `rdtsc()`  [INFERRED]
  src/cpu/clocks.rs → crates/platform/src/clock.rs
- `adapters()` --calls--> `read_trim()`  [INFERRED]
  src/network.rs → crates/platform/src/os.rs
- `audio()` --calls--> `read_trim()`  [INFERRED]
  src/peripherals.rs → crates/platform/src/os.rs

## Import Cycles
- 1-file cycle: `crates/platform/src/firmware.rs -> crates/platform/src/firmware.rs`
- 1-file cycle: `src/live_frames.rs -> src/live_frames.rs`
- 1-file cycle: `src/gui/tools.rs -> src/gui/tools.rs`
- 1-file cycle: `src/gui/system_history.rs -> src/gui/system_history.rs`
- 1-file cycle: `src/battery.rs -> src/battery.rs`
- 1-file cycle: `src/usb.rs -> src/usb.rs`
- 1-file cycle: `src/storage.rs -> src/storage.rs`
- 1-file cycle: `src/npu.rs -> src/npu.rs`
- 1-file cycle: `src/software.rs -> src/software.rs`
- 1-file cycle: `src/gpu/dl.rs -> src/gpu/dl.rs`
- 1-file cycle: `crates/platform/src/elevation.rs -> crates/platform/src/elevation.rs`
- 1-file cycle: `crates/platform/src/large_pages.rs -> crates/platform/src/large_pages.rs`
- 1-file cycle: `crates/platform/src/procs.rs -> crates/platform/src/procs.rs`
- 1-file cycle: `crates/core/src/layout.rs -> crates/core/src/layout.rs`
- 1-file cycle: `src/peripherals.rs -> src/peripherals.rs`
- 1-file cycle: `crates/platform/src/display.rs -> crates/platform/src/display.rs`
- 1-file cycle: `src/gui/tray.rs -> src/gui/tray.rs`
- 1-file cycle: `crates/platform/src/desktop.rs -> crates/platform/src/desktop.rs`
- 1-file cycle: `crates/platform/src/gpu_engines.rs -> crates/platform/src/gpu_engines.rs`
- 1-file cycle: `crates/platform/src/memory.rs -> crates/platform/src/memory.rs`

## Hyperedges (group relationships)
- **Consumers of the shared Metric Engine / Widget Engine** — docs_v1_readme_metric_engine, docs_v1_zeez_live_overlay, docs_v1_core_pages_mini_monitor, docs_v1_core_pages_tray_icon, docs_v1_core_pages_overview, docs_v1_bottleneck_detector_now_card [EXTRACTED 1.00]
- **Security controls (elevation, updater, threat model, policy)** — changelog_elevated_helper, changelog_one_click_update, changelog_tempdir_module, readme_threat_model, security_vulnerability_priorities [INFERRED 0.75]
- **Frame timing without touching the game (anti-cheat safety)** — docs_adr_0003_fps_on_windows_comes_from_presentmon_bundled_presentmon, docs_adr_0004_fps_on_linux_comes_from_mangohud_mangohud, docs_testing_macos_metal_hud_experiments, docs_adr_0012_no_code_injection_ever_own_overlay_window [INFERRED 0.85]
- **Anti-cheat-safe design (no injection, limited handles, PawnIO gate)** — docs_v1_readme_p4_safety_first, docs_v1_zeez_live_injection_rejected, docs_v1_task_manager_anticheat_privacy_rule, docs_v1_zeez_live_anticheat_compat_list, docs_v1_zeez_live_etw_presentmon [INFERRED 0.85]
- **Bundled third-party binaries pinned by SHA-256 and re-checked** — packaging_pins, third_party, readme_pawnio, readme_presentmon, third_party_pawnio_modules [INFERRED 0.85]
- **Per-OS frame sources feeding ZeeZ Live and the bottleneck detector** — docs_v1_zeez_live_etw_presentmon, docs_v1_zeez_live_mangohud_linux, docs_v1_zeez_live_macos_frames, docs_v1_bottleneck_detector_gpu_busy_signal, docs_v1_bottleneck_detector_heuristic_fallback [INFERRED 0.85]
- **Overlay CPU 2% budget: measure, miss, plan** — docs_performance_p6_budget, docs_performance_overlay_cpu_not_met, docs_testing_soak_overlay_only_mode, docs_v1_1_readme_overlay_cpu_goal, docs_v1_1_readme_overlay_frame_budget [INFERRED 0.85]
- **Controls against paths handed to a more privileged process** — docs_security_threat_model_private_tempdir_rule, docs_security_threat_model_held_presentmon_handle, docs_security_review_2026_11_09_r1_elevated_dump, docs_windows_pawnio_install_pawn_io_action, docs_audit_readme_bug_classes [INFERRED 0.85]
- **ZeeZ Live frame-timing pipeline** — changelog_zeez_live, readme_presentmon, readme_mangohud, changelog_session_capture, changelog_bottleneck_detector [INFERRED 0.85]

## Communities (261 total, 33 thin omitted)

### Community 0 - "sensors.rs"
Cohesion: 0.06
Nodes (63): resolve(), value_text(), tile_text(), avg_clock(), CHART_MAX_H, CHART_MIN_H, charts(), chipset_temp() (+55 more)

### Community 1 - "egui"
Cohesion: 0.09
Nodes (32): Conditions, Job, median_spread(), merge(), points(), repeat(), scores_and_medians(), Sub (+24 more)

### Community 2 - "x86.rs"
Cohesion: 0.21
Nodes (23): amd_legacy_caches(), bit(), bits(), brand(), CacheDesc, caches(), cpuid(), decodes_this_cpu() (+15 more)

### Community 3 - "details"
Cohesion: 0.19
Nodes (17): is_protected(), is_third_party(), protected_and_third_party_classification(), apply(), config2(), dacl_sddl(), details(), enable_shutdown_privilege() (+9 more)

### Community 4 - "System Performance Charts"
Cohesion: 0.07
Nodes (43): ago_text(), AXIS_FONT, chart(), Line, LINE_WIDTH, MARGIN_BOTTOM, MARGIN_LEFT, nearest() (+35 more)

### Community 5 - "Row"
Cohesion: 0.10
Nodes (19): Config, parse(), apply(), DARK, fonts(), GAP, LIGHT, metric() (+11 more)

### Community 7 - "palette"
Cohesion: 0.07
Nodes (33): Access, meter(), spark(), adapter_picker(), prefixes(), publish_throttling(), a_written_session_reads_back(), Capture (+25 more)

### Community 8 - "CPU Benchmark Runner"
Cohesion: 0.07
Nodes (39): Bench, CHECK_10K, CHUNK, cli(), deterministic(), fmix(), hostname(), LANES (+31 more)

### Community 9 - "SPD Decoding"
Cohesion: 0.09
Nodes (56): ascii(), bandwidth(), bcd(), cl_rows(), Core, crc16(), crc_known_value(), cycle() (+48 more)

### Community 10 - "Memory Benchmark"
Cohesion: 0.08
Nodes (42): alloc(), Bandwidth, bandwidth(), BLOCK_LINES, build_chain(), chain_is_one_cycle_over_all_lines(), chase(), CHASE_STEP (+34 more)

### Community 11 - "Storage Drives Probe"
Cohesion: 0.08
Nodes (49): ata(), ata_identify(), ata_string(), BUS, CM_Free_Log_Conf_Handle(), CM_Free_Res_Des_Handle(), CM_Get_Device_IDW(), CM_Get_Device_Interface_List_SizeW() (+41 more)

### Community 12 - "core/src/smbios.rs"
Cohesion: 0.09
Nodes (47): smbios_tables(), agesa(), array_ecc(), array_location(), array_use(), Bios, bios_info(), bios_of() (+39 more)

### Community 13 - "system_history.rs"
Cohesion: 0.08
Nodes (35): accumulate(), add(), AggregatedApp, app_history_tab_shows_the_view(), AppDayRecord, apps(), clear(), Cmd (+27 more)

### Community 14 - "gui/mod.rs"
Cohesion: 0.04
Nodes (41): ES, get(), PT_BR, set(), switch(), App, app_icon(), AutoShots (+33 more)

### Community 15 - "Metric Engine"
Cohesion: 0.07
Nodes (32): a_panicking_source_is_paused_and_reported(), a_stuck_source_does_not_stall_the_others(), Actor, clamp_period(), Config, Cost, Csv, csv_header_rows_and_new_columns() (+24 more)

### Community 16 - "Network Adapters"
Cohesion: 0.07
Nodes (46): Adapter, adapters(), AddrNode, band_from_mhz(), BssList, channel_from_mhz(), Close, collect() (+38 more)

### Community 17 - "Ctx"
Cohesion: 0.08
Nodes (28): cluster_cpus(), cppc(), intel_limits(), Mode, Cpu, Memory, State, batteries() (+20 more)

### Community 18 - "with"
Cohesion: 0.05
Nodes (52): container, needs, runs-on, steps, env, CARGO_TERM_COLOR, GH_TOKEN, TAG (+44 more)

### Community 19 - "CPU Sensors and Power Limits"
Cohesion: 0.17
Nodes (23): get(), core_types(), amd(), amd_limits(), amd_rapl(), ccd_base(), core_clocks(), core_threads() (+15 more)

### Community 20 - "CPU Model Database"
Cohesion: 0.05
Nodes (48): a(), AM2P, AM3, AM3P, AM4, AM5, ANY, APPLE (+40 more)

### Community 21 - "system.rs"
Cohesion: 0.11
Nodes (22): AffinityModal, ALL_COL_IDS, ConfirmEnd, csv_and_json_export_and_snapshot_diff(), diff_snapshots(), DiffEntry, DiffKind, Added (+14 more)

### Community 22 - "update.rs"
Cohesion: 0.06
Nodes (60): API, apply(), Asset, Channel, .ALL, Alpha, Beta, Snapshot (+52 more)

### Community 23 - "Process Snapshot Platform"
Cohesion: 0.07
Nodes (40): mach_timebase(), parse_major_faults(), the_rpc_endpoint_mapper_is_listening(), boot_ms(), command_line(), decoders(), elf_arch(), filetime_conversion() (+32 more)

### Community 24 - "Super I/O Sensor Chips"
Cohesion: 0.08
Nodes (46): ASR_AM4_FANS, ASR_AM4_VOLTS, ASR_AM5_VOLTS, ASR_INTEL_FANS, ASR_INTEL_VOLTS, ASR_X2_VOLTS, ASUS_800_VOLTS, ASUS_800_VTT_VOLTS (+38 more)

### Community 25 - "actions.rs"
Cohesion: 0.13
Nodes (19): decode(), ELEVATED_FLAG, encode(), hex(), MAX_ENCODED, rejects_anything_else(), Risk, Critical (+11 more)

### Community 26 - "Community 26"
Cohesion: 0.06
Nodes (30): KernelModule, I2cDev, Msr, BootSummary, ES, kernel_module(), pawnio_doc(), PT_BR (+22 more)

### Community 27 - "Windows GPU Probe"
Cohesion: 0.06
Nodes (57): Adl, adl(), adl_alloc(), AdlInfo, apply_nvapi(), cm_bdf(), CM_Get_DevNode_Registry_PropertyW(), CM_Get_Parent() (+49 more)

### Community 28 - "CPU Test Suite"
Cohesion: 0.13
Nodes (33): add(), AES_BLOCKS, dot(), gemm(), gemm_any(), gemm_avx2(), gemm_avx512(), gemm_is_right() (+25 more)

### Community 29 - "Single Instance and SIDs"
Cohesion: 0.08
Nodes (17): WaitForSingleObject(), Claim, claim(), First, Running, H, Instance, key() (+9 more)

### Community 30 - "data.rs"
Cohesion: 0.14
Nodes (21): categories_sizes_and_delete(), Category, .ALL, AppLog, AuditLog, BenchmarkHistory, Diagnostics, Exports (+13 more)

### Community 31 - "mod.rs"
Cohesion: 0.10
Nodes (31): board_ident(), board_model(), board_monitors(), chips(), chipset(), chipset_group(), collect(), cpu_monitors() (+23 more)

### Community 32 - "palette.rs"
Cohesion: 0.22
Nodes (14): Entry, ES, finds_fields_and_the_page_that_shows_them(), index(), page_of(), Palette, PT_BR, ranks_title_prefix_first_and_needs_every_word() (+6 more)

### Community 33 - "elevation.rs"
Cohesion: 0.09
Nodes (24): can_relaunch(), elevation_query_does_not_crash(), handoff_marker(), handoff_marker_reports_started(), is_elevated(), Outcome, Cancelled, Failed (+16 more)

### Community 34 - "disk_io.rs"
Cohesion: 0.09
Nodes (21): CFDictionaryGetValue(), CFNumberGetValue(), CFRelease(), cfstr(), CFStringCreateWithCString(), CFTypeRef, GAP, IOIteratorNext() (+13 more)

### Community 35 - "layout.rs"
Cohesion: 0.10
Nodes (30): all(), benchmark(), big(), compact(), detailed(), Direction, Grid, Horizontal (+22 more)

### Community 36 - "icon.rs"
Cohesion: 0.02
Nodes (106): ALL, ARROW_CIRCLE_UP, ARROW_CLOCKWISE, ARROW_COUNTER_CLOCKWISE, ARROW_DOWN, ARROW_ELBOW_DOWN_RIGHT, ARROW_RIGHT, ARROW_UP (+98 more)

### Community 37 - "linux.rs"
Cohesion: 0.07
Nodes (16): Bdf, cfg_path(), I2C_SLAVE_FORCE, I2C_SMBUS, I2C_SMBUS_BYTE, I2C_SMBUS_BYTE_DATA, I2C_SMBUS_READ, I2C_SMBUS_WORD_DATA (+8 more)

### Community 38 - "tr"
Cohesion: 0.05
Nodes (39): conditions(), BUCKETS, bar(), bytes_text(), ES, PT_BR, save_row(), conditions() (+31 more)

### Community 39 - "drivetest.rs"
Cohesion: 0.12
Nodes (35): ata_kind(), ata_log_parses_newest_first(), ata_logs(), ata_selftest(), ata_status_text(), ctrl(), Entry, is_ata() (+27 more)

### Community 40 - "system_services.rs"
Cohesion: 0.12
Nodes (11): Finished, a_refused_change_leaves_no_undo_and_resets_the_editor(), combo(), ES, hash_name(), link_button(), Modal, ConfirmStop (+3 more)

### Community 41 - "decode.rs"
Cohesion: 0.15
Nodes (29): bars(), bars_and_ext(), bytes_text(), cap(), cap_name(), cap_node(), caps(), dev() (+21 more)

### Community 42 - "smart.rs"
Cohesion: 0.10
Nodes (29): ascii(), ata(), ata_fields_and_dump(), ata_text(), hex_dump(), kelvin(), le16(), le32() (+21 more)

### Community 43 - "windows.rs"
Cohesion: 0.10
Nodes (30): driver_installed(), driver_modules_found(), CloseFn, CreateMutexW(), ExecFn, GetProcAddress(), Handle, Held (+22 more)

### Community 44 - "peripherals.rs"
Cohesion: 0.10
Nodes (24): Bars, bars(), bars_of(), batteries(), BATTERY, bluetooth_batteries(), DEVICE, FORMAT (+16 more)

### Community 45 - "f"
Cohesion: 0.08
Nodes (22): diagnostics_bundle(), info(), install_panic_hook(), LOCK, MAX, path(), warn(), write() (+14 more)

### Community 46 - "os.rs"
Cohesion: 0.06
Nodes (34): cpu_groups(), CTRL_C, final_path(), FreeConsole(), GetActiveProcessorCount(), GetActiveProcessorGroupCount(), GetConsoleProcessList(), GetCurrentThread() (+26 more)

### Community 47 - "diff.rs"
Cohesion: 0.11
Nodes (31): g(), Change, change(), ChangeKind, Added, Modified, Removed, cli() (+23 more)

### Community 48 - "split.rs"
Cohesion: 0.15
Nodes (17): a_narrow_window_opens_the_panel_over_the_table(), clamp(), drawer(), HANDLE, harness(), MAX_FRAC, MIN, show() (+9 more)

### Community 49 - "frames.rs"
Cohesion: 0.11
Nodes (28): o(), curve_histogram_and_window(), exclusive_fullscreen(), fps(), frame_csv_line(), FRAMES_CSV_HEADER, histogram(), is_stutter() (+20 more)

### Community 50 - "hotkeys_portal.rs"
Cohesion: 0.11
Nodes (19): bind(), create_session(), DEST, IFACE, listen(), no_bus_means_unavailable(), PATH, Portal (+11 more)

### Community 51 - "overview.rs"
Cohesion: 0.16
Nodes (23): drives(), customize_presets_and_restore(), editor(), ES, find(), health(), identity(), join() (+15 more)

### Community 52 - "mod.rs (2)"
Cohesion: 0.13
Nodes (25): collect(), decode_dimm(), Dimm, dimm_temps(), DUMP, dumps(), FakeHub, golden_ddr5_matches_cpuz() (+17 more)

### Community 53 - "proc_ops.rs"
Cohesion: 0.11
Nodes (26): handoff_dirs_are_fresh_private_and_unique(), acts_on_a_child_process(), check(), class(), connections(), dump(), eco(), end() (+18 more)

### Community 54 - "intel.rs"
Cohesion: 0.18
Nodes (21): ADL, ARL, arrow_lake(), BCLK_FREQ, decode(), degrade(), F, f() (+13 more)

### Community 55 - "procs.rs"
Cohesion: 0.09
Nodes (28): anti_cheat(), classify(), cpu_percent(), grouping_and_critical(), HIGH_CPU, HISTORY, Kind, App (+20 more)

### Community 56 - "display_drm.rs"
Cohesion: 0.11
Nodes (20): card_modes(), CardRes, CONNECTED, connector_name(), connector_names_match_the_kernels(), Crtc, driver_of(), FLAG_DBLSCAN (+12 more)

### Community 57 - "Action"
Cohesion: 0.11
Nodes (17): Action, InstallPawnIo, LoadKernelModule, Process, Service, Startup, Privilege, Admin (+9 more)

### Community 58 - "gpubench.rs"
Cohesion: 0.10
Nodes (17): Ctl, adapters(), BIG, block_on(), flops(), FLOPS_PER_ITER, FMA, Gpu (+9 more)

### Community 59 - "welcome.rs"
Cohesion: 0.27
Nodes (9): continuing_without_ticking_means_no_update_check(), ES, harness(), page(), PRIVACY_VERSION, PT_BR, St, ticking_the_box_opts_in() (+1 more)

### Community 60 - "fuzz.yml"
Cohesion: 0.06
Nodes (32): MINUTES, name, runs-on, steps, strategy, timeout-minutes, minutes, jobs (+24 more)

### Community 61 - "mini.rs"
Cohesion: 0.16
Nodes (17): apply(), Change, Close, Edit, Layout, New, Pos, Scale (+9 more)

### Community 62 - "software.rs"
Cohesion: 0.20
Nodes (12): after(), collect(), created_secs(), CV, dates(), fields(), mdy(), os_line() (+4 more)

### Community 63 - "Unit"
Cohesion: 0.08
Nodes (20): Buf, History, Series, Snapshot, Stat, Tick, Unit, Ampere (+12 more)

### Community 64 - "proc_net.rs"
Cohesion: 0.10
Nodes (25): begin(), inet_diag_message(), KERNEL_NETWORK, KEYWORDS, LAST, nettop_csv(), on_event(), parse_inet_diag() (+17 more)

### Community 65 - "Window"
Cohesion: 0.30
Nodes (19): analyze_verdicts(), avg(), cpu_saturated(), Finding, gpu_headroom(), gpu_saturated(), n0(), pct() (+11 more)

### Community 66 - "tray.rs (2)"
Cohesion: 0.05
Nodes (36): Action, Lock, Mini, Pause, Quit, Sensors, Speed, Toggle (+28 more)

### Community 67 - "bottleneck.rs"
Cohesion: 0.20
Nodes (26): a_real_presentmon_capture_tells_cpu_bound_loading_from_gpu_bound_play(), a_session_sums_up_time_per_class(), Analysis, analyze(), base(), class(), classify(), cpu_bound_needs_a_saturated_thread_and_gpu_headroom() (+18 more)

### Community 68 - "mod.rs (3)"
Cohesion: 0.17
Nodes (21): Adapter, adapter_nodes(), adapters(), apply_nvml(), backend(), collect(), fill_from_db(), gen_speed() (+13 more)

### Community 69 - "live_sessions.rs"
Cohesion: 0.15
Nodes (18): a_mangohud_log_imports_as_a_session(), bars(), COLORS, compare(), sessions_dir(), ES, import(), key() (+10 more)

### Community 70 - "procs.rs (2)"
Cohesion: 0.11
Nodes (8): Window, Collector, collector_builds_rows_and_rates(), Meta, Prev, Sampler, Snapshot, worker()

### Community 71 - "trf"
Cohesion: 0.11
Nodes (20): Confidence, High, Low, Medium, Verdict, a_session_result_lists_shares_evidence_and_advice(), class_color(), Detector (+12 more)

### Community 72 - "mod.rs (4)"
Cohesion: 0.13
Nodes (24): core_mhz(), amd_pstates(), apic_tree(), arm_free_sysctl(), build(), bus_mhz(), Cache, cache_line() (+16 more)

### Community 73 - "trayicon.rs"
Cohesion: 0.11
Nodes (23): Threshold, Tone, Danger, Good, Normal, Warning, detail_levels_filter_cards(), placeholder() (+15 more)

### Community 74 - "windows.rs (2)"
Cohesion: 0.18
Nodes (16): array(), collect(), cpu_load(), cpu_mhz(), FmtValue, GetSystemPowerStatus(), Item, monitors() (+8 more)

### Community 75 - "telemetry.rs (2)"
Cohesion: 0.06
Nodes (33): throttle_kind(), AT_LIMIT, c(), cli_log(), CLOCK_DROP, Cond, cond(), Debounce (+25 more)

### Community 76 - "chipset.rs"
Cohesion: 0.13
Nodes (28): amd_chipset(), amd_umc(), chipset_names(), chipset_token(), ChipsetSummary, collect(), decode_umc(), decode_umc_more() (+20 more)

### Community 77 - "telemetry.rs (3)"
Cohesion: 0.15
Nodes (15): Reading, batteries(), battery(), cpu_extra(), disks(), DiskSnap, gpu(), gpu_group() (+7 more)

### Community 78 - "desktop.rs"
Cohesion: 0.09
Nodes (3): each(), Window, windows()

### Community 79 - "display.rs"
Cohesion: 0.10
Nodes (10): advanced_colour(), Colour, device(), each_monitor(), mode(), modes(), POWER_SAVER, ScreenRect (+2 more)

### Community 80 - "gpu_engines.rs"
Cohesion: 0.14
Nodes (20): CFDictionaryGetValue(), CFNumberGetValue(), CFRelease(), CFStringCreateWithCString(), CFTypeRef, IOIteratorNext(), IOObjectRelease(), IORegistryEntryCreateCFProperty() (+12 more)

### Community 81 - "page_tests.rs"
Cohesion: 0.19
Nodes (19): about_page(), copy_and_pin(), detail_switch(), devices_page(), every_page_and_tab_has_an_interaction_test(), graphics_page(), live_tab(), live_test() (+11 more)

### Community 82 - "executor.rs"
Cohesion: 0.16
Nodes (17): audit_lines_are_json(), AuditEntry, audit_log(), audit_path(), bundled_pawnio(), elevated_main(), install_pawnio(), load_kernel_module() (+9 more)

### Community 83 - "amd_pm.rs"
Cohesion: 0.16
Nodes (27): APU, cells(), close(), Core, Cores, cores(), decode(), enabled_slots() (+19 more)

### Community 84 - "mod.rs (5)"
Cohesion: 0.08
Nodes (6): Caps, detect(), Hw, NoHw, none(), PAUSED

### Community 85 - "users.rs"
Cohesion: 0.06
Nodes (16): query_wts_str(), send_message(), sessions(), state_label(), to_wide(), UserSession, aggregate_users(), ES (+8 more)

### Community 86 - "macos.rs"
Cohesion: 0.15
Nodes (24): battery(), call(), conn(), decode(), IOConnectCallStructMethod(), IOObjectRelease(), IOServiceGetMatchingService(), IOServiceMatching() (+16 more)

### Community 87 - "large_pages.rs"
Cohesion: 0.08
Nodes (4): alloc(), HUGE, lock_memory_privilege(), Region

### Community 88 - "i18n.rs"
Cohesion: 0.17
Nodes (18): bottleneck_texts_are_translated(), current(), CURRENT, every_literal_is_translated(), from_code(), Lang, En, Es (+10 more)

### Community 89 - "core/src/robustness.rs"
Cohesion: 0.11
Nodes (16): smooth(), detector_rules_on_random_windows(), edid_blocks(), FIXTURE, hardware_and_mangohud_logs(), layout_files(), n(), output_lines_ignore_chunking() (+8 more)

### Community 90 - "report.rs"
Cohesion: 0.14
Nodes (28): esc(), find(), find_section(), groups(), gv(), hex(), hex_rows(), html_nodes() (+20 more)

### Community 91 - "overlay.rs"
Cohesion: 0.09
Nodes (4): apply(), each(), Find, Style

### Community 92 - "split.rs"
Cohesion: 0.20
Nodes (14): b64(), device(), list(), next_tag(), parse_plist(), plist(), Pv, Arr (+6 more)

### Community 93 - "soak.py"
Cohesion: 0.14
Nodes (15): analyze(), frame_trend(), _linux_sampler(), _macos_sampler(), make_sampler(), profile(), report(), run_gui() (+7 more)

### Community 94 - "nvml.rs"
Cohesion: 0.16
Nodes (14): db_apple(), cstr(), Dev, FanRpm, get(), Info, load(), Memory (+6 more)

### Community 95 - "live.rs"
Cohesion: 0.10
Nodes (11): HotkeyAction, .ALL, Capture, CycleStyle, EditMode, Marker, ToggleOverlay, Hotkeys (+3 more)

### Community 96 - "serialize"
Cohesion: 0.36
Nodes (6): defaults(), fires_only_when_the_whole_window_is_over(), firing(), pts(), Rule, rules_load_from_older_settings()

### Community 97 - "windows.rs (4)"
Cohesion: 0.14
Nodes (9): exits_config(), fn_name(), GlobalMutex, live_dump(), packing(), PawnIo, pch_decode(), pch_raw() (+1 more)

### Community 98 - "devices.rs"
Cohesion: 0.17
Nodes (20): adapter(), battery(), device_card(), drive(), ES, gb(), Health, health() (+12 more)

### Community 99 - "pins.rs"
Cohesion: 0.13
Nodes (22): cache_id(), card_id(), card_text(), current_card(), enter_card(), ES, is_pinned(), latest() (+14 more)

### Community 100 - "screens.rs"
Cohesion: 0.14
Nodes (24): nearest_anchor(), nine_anchors_and_offsets(), offset_for(), place(), A, B, C, choose() (+16 more)

### Community 102 - "diskbench.rs"
Cohesion: 0.16
Nodes (16): write_at(), a_tiny_run_measures_and_cleans_up(), a_tiny_sustained_write_cleans_up(), bench(), phase(), rates(), rates_math(), Run (+8 more)

### Community 103 - "live_mangohud.rs"
Cohesion: 0.21
Nodes (13): config_home(), config_path(), configure(), env_path(), home(), Install, Distro, Flatpak (+5 more)

### Community 104 - "session.rs"
Cohesion: 0.12
Nodes (28): recorded_session_windows(), windows_from_session(), corrupt_sessions_stay_bounded(), long_sessions_are_linear(), Band, ChartLine, comparability(), comparisons_and_repeats() (+20 more)

### Community 105 - "zeez-live.md"
Cohesion: 0.06
Nodes (40): ADR 0006: Dependencies, rusqlite (bundled SQLite), windows-sys, ADR 0013: glow renderer with wgpu fallback, glow (OpenGL) renderer, wgpu fallback (CZ_RENDERER=wgpu, WARP), Now card, live readings, session analysis, Benchmark principles (safe, reproducible, 1000 = reference 7900X) (+32 more)

### Community 106 - "ci.yml"
Cohesion: 0.18
Nodes (23): steps, steps, name, runs-on, steps, headless, visual, steps (+15 more)

### Community 107 - "battery.rs"
Cohesion: 0.15
Nodes (19): Battery, chemistry_name(), collect(), GUID_DEVICE_BATTERY, info(), IOCTL_BATTERY_QUERY_INFORMATION, IOCTL_BATTERY_QUERY_STATUS, IOCTL_BATTERY_QUERY_TAG (+11 more)

### Community 108 - "usb.rs"
Cohesion: 0.16
Nodes (16): CM_Get_Parent(), parent(), class_from_compat(), collect(), devices(), hardware_ids(), hub_info(), id_field() (+8 more)

### Community 109 - "startup.rs (2)"
Cohesion: 0.12
Nodes (15): StartupOp, SetEnabled, StartupSource, CommonFolder, HkcuRun, HkcuRunOnce, HklmRun, HklmRun32 (+7 more)

### Community 110 - "a11y.rs"
Cohesion: 0.10
Nodes (26): at_every_zoom(), beyond(), bottleneck_sample(), chart_colours_meet_the_graphics_minimum(), charts_and_bars_say_what_they_show(), class_colours_are_distinct_and_readable(), collisions(), contrast() (+18 more)

### Community 111 - "nightly.yml"
Cohesion: 0.09
Nodes (21): minutes, jobs, soak, default, description, name, on, schedule (+13 more)

### Community 113 - "npu.rs"
Cohesion: 0.12
Nodes (20): f(), release(), the_demo_replays_in_a_loop(), collect(), CORE_COMPUTE, Count, CreateList, devices() (+12 more)

### Community 114 - "BenchView"
Cohesion: 0.14
Nodes (14): SHOTS, Step, AboutPt, Bench, Bottleneck, Page, System, Tool (+6 more)

### Community 115 - "firmware.rs"
Cohesion: 0.17
Nodes (7): Header, header(), text(), acpi(), Firmware, read(), SECURE_BOOT

### Community 116 - "availability.rs"
Cohesion: 0.06
Nodes (47): Capability, .ALL, BoardSensors, ChipsetTemp, CpuSensors, MemoryTimings, Spd, caps() (+39 more)

### Community 117 - "live.rs (2)"
Cohesion: 0.19
Nodes (13): EDGE, exe_name(), hotkey_conflicts(), hotkeys_normalize_and_conflict(), normalize_keys(), Overlay, portal_request_path(), portal_trigger() (+5 more)

### Community 118 - "PresentMon bundled sidecar (ETW)"
Cohesion: 0.14
Nodes (13): ADR 0001: ZeeZ Live is the brand, ZeeZ Live (Overlay, Capture, Reports), ADR 0003: FPS on Windows from PresentMon, PresentMon bundled sidecar (ETW), ADR 0004: FPS on Linux from MangoHud, MangoHud integration (detect, install, config, logs), SetLayeredWindowAttributes click-through fix, global-hotkey crate (RegisterHotKey/Carbon/XGrabKey) (+5 more)

### Community 119 - "clocks.rs"
Cohesion: 0.08
Nodes (16): rdtsc(), active_from_deltas(), active_mhz(), add_chain_mhz(), aperf_mperf(), aperf_mperf_regs(), chain(), live_source() (+8 more)

### Community 120 - "pci/windows.rs"
Cohesion: 0.14
Nodes (23): CM_Get_Parent(), DevInfoData, DevPropKey, DIGCF_ALLCLASSES, DIGCF_PRESENT, Guid, hwid_field(), list() (+15 more)

### Community 121 - "cstates.rs"
Cohesion: 0.16
Nodes (8): array(), LAST, PREV, Query, QUERY, Raw, read(), Residency

### Community 122 - "memory.rs"
Cohesion: 0.16
Nodes (6): Handles, LAST, MB, PATHS, Query, QUERY

### Community 123 - "memtest.rs"
Cohesion: 0.17
Nodes (9): every_pattern_catches_a_flipped_bit(), fill(), MemTest, PATTERNS, runs_and_stops(), seed(), value(), verify() (+1 more)

### Community 124 - "CHANGELOG.md"
Cohesion: 0.09
Nodes (45): ADR 0009: releases stay unsigned for now, ADR 0013: glow with wgpu fallback, 1.1.0 roadmap (docs/v1.1), Security review 2026-11-09, 1.0 plan (docs/v1), ADR 0009: Releases stay unsigned, SHA256SUMS verification for unsigned releases, ADR 0011: Workspace split by layer (+37 more)

### Community 125 - "live_frames.rs"
Cohesion: 0.15
Nodes (24): Frame, App, apps(), begin_capture(), CAPTURE_MAX, Demo, demo(), due() (+16 more)

### Community 126 - "detail.rs"
Cohesion: 0.25
Nodes (9): BASIC, Level, Advanced, .ALL, Basic, Expert, level_of(), levels() (+1 more)

### Community 127 - "history.rs"
Cohesion: 0.21
Nodes (18): add(), dates(), Entry, entry(), join(), load(), load_from(), memory_line() (+10 more)

### Community 128 - "core/src/services.rs"
Cohesion: 0.18
Nodes (19): small_text_parsers(), a_dacl_that_refuses_administrators(), CHANGE_CONFIG, dependency_chain_is_cycle_safe_and_filters_work(), matches_filter(), parse_systemd_secs(), parse_systemd_show(), populate_dependents() (+11 more)

### Community 129 - "superio.rs"
Cohesion: 0.25
Nodes (12): apply(), fintek(), Io, ite(), nct6687_like_the_dump(), nct_ec(), nct_new(), nct_old() (+4 more)

### Community 130 - "Source"
Cohesion: 0.06
Nodes (17): actor(), check_contract(), Class, Fast, Slow, Fake, Source, spawn() (+9 more)

### Community 131 - "layout_view.rs"
Cohesion: 0.08
Nodes (23): editor(), ES, KINDS, PT_BR, THROTTLE_DEVICES, ALIASES, aliases_resolve_to_this_machines_sensors(), bar() (+15 more)

### Community 132 - "cpu_os.rs"
Cohesion: 0.14
Nodes (11): CODE_INTEGRITY, KVA_SHADOW, NtQuerySystemInformation(), Power, power(), query(), read(), SPECULATION_CONTROL (+3 more)

### Community 133 - "linux.rs (2)"
Cohesion: 0.16
Nodes (18): raw(), read_trim(), read_u64(), adapter(), cards(), dpm(), freq(), hwmon_live() (+10 more)

### Community 135 - "health.rs"
Cohesion: 0.26
Nodes (16): a_healthy_build_has_nothing_to_say(), check(), Display, each_mistake(), Facts, Finding, finding(), good() (+8 more)

### Community 136 - "src/smbios.rs"
Cohesion: 0.16
Nodes (17): ioreg_hex(), synth(), uuid_bytes(), find_agesa(), rom_agesa(), section(), uefi(), apple_table() (+9 more)

### Community 137 - "body"
Cohesion: 0.12
Nodes (15): description, label, placeholder, render, value, body, attributes, id (+7 more)

### Community 138 - "table.rs"
Cohesion: 0.05
Nodes (65): half_typed_and_odd_input_never_fails(), hit(), matches(), number(), Op, Contains, Eq, Ge (+57 more)

### Community 139 - "gui/tools.rs"
Cohesion: 0.12
Nodes (19): changes(), CHART_BOTTOM, CHART_HEIGHT, CHART_LEFT, CHART_LINE, Compare, diff_as_text(), diff_text() (+11 more)

### Community 140 - ".render_table"
Cohesion: 0.19
Nodes (15): CtxCmd, Affinity, Dump, Eco, End, Inspect, Priority, RestartShell (+7 more)

### Community 141 - "Page"
Cohesion: 0.12
Nodes (16): NAV, NAV_BOTTOM, Page, About, Benchmarks, Devices, Graphics, Live (+8 more)

### Community 142 - "gpu/macos.rs"
Cohesion: 0.24
Nodes (9): cmd(), logs(), adapter(), apple_nm(), hex_id(), ioreg_edids(), metal(), probe() (+1 more)

### Community 143 - "FEATURES.md (CPU-Z 2.15 clean-room spec)"
Cohesion: 0.05
Nodes (42): ADR 0012: no code injection, ever, ADR 0014: no hardware balance bar in 1.0, ADR 0015: Wayland shortcuts through the GlobalShortcuts portal, Anti-cheat safe mode, App history (SQLite store), Bottleneck detector (What limits my FPS?), Build health checks, Per-action elevation (one-shot elevated copy, closed operation list) (+34 more)

### Community 144 - "actions.rs"
Cohesion: 0.13
Nodes (14): ServiceOp, Pause, Restart, SetAccount, SetRecovery, SetStartup, Start, StartWithParams (+6 more)

### Community 145 - "CPU ZeeZ 1.1.0 plan"
Cohesion: 0.06
Nodes (39): AccessKit names, states and values for painted widgets, Accessibility (S3-03), Keyboard navigation and table shortcuts, Scale test 75-200% (at_every_zoom), Screen readers cannot read painted tables and charts, WCAG AA contrast check from theme palette, Read-only helper service (after 1.0), ADR 0007: One 1.0.0 release in gated stages (+31 more)

### Community 146 - "CpuZeeZ 1.0 Plan (hub)"
Cohesion: 0.05
Nodes (40): ADR 0008: Planning documents are in English, Command palette (Ctrl+K), Shared page patterns (detail levels, Ctrl+K palette, banners, pin, units), rusqlite (bundled SQLite), ADRs in docs/adr, Contract test suites and fakes, core crate (pure logic, no OS/GUI), Definition of done (+32 more)

### Community 147 - "pkgver.py"
Cohesion: 0.23
Nodes (3): walk(), _test(), versions()

### Community 148 - "Batteries"
Cohesion: 0.12
Nodes (3): Batteries, Drives, MeasuredClock

### Community 149 - "Map"
Cohesion: 0.15
Nodes (13): b(), Board, board_lookup(), board_map(), BOARDS, EC_GENERIC, ITE_GENERIC, ITE_OLD_GENERIC (+5 more)

### Community 150 - "Lib"
Cohesion: 0.43
Nodes (4): GetProcAddress(), Lib, LoadLibraryW(), sym!

### Community 151 - "raster.py"
Cohesion: 0.20
Nodes (8): dist_seg(), in_poly(), in_rrect(), on_arc(), png(), render(), render_png(), sample()

### Community 152 - "PdhCollectQueryData"
Cohesion: 0.23
Nodes (9): composition(), hard_faults_per_sec(), lists(), open(), Array, reads_a_wildcard_counter(), PdhAddEnglishCounterW(), PdhCollectQueryData() (+1 more)

### Community 153 - "Metric Engine"
Cohesion: 0.19
Nodes (11): windows-sys crate, D10 Hand-written FFI, D1 APERF/MPERF clock busy-spin, D2 rdmsr spawns thread per read, D3 SMART probe inside sampler, D5 series() clones history; duplicate GUI sampler, Diagnosis of 0.3.0 (D1-D10), Metric subscriptions and rate classes (+3 more)

### Community 154 - "direct_io.rs"
Cohesion: 0.18
Nodes (7): ALIGN, Buffer, FILE_FLAG_NO_BUFFERING, FILE_FLAG_WRITE_THROUGH, open(), read_at(), uncached_round_trip()

### Community 155 - "hwmon.rs"
Cohesion: 0.31
Nodes (10): cpu_label(), cpu_temps(), display_name(), fake_sysfs(), put(), read(), readings(), ROOT (+2 more)

### Community 156 - "tempdir.rs"
Cohesion: 0.32
Nodes (14): base_is_safe(), base_validation_follows_symlinks_in_the_path(), base_validation_rejects_swappable_ancestors(), base_validation_rejects_swappable_symlinks(), base_validation_requires_sticky_when_shared(), create(), create_exclusive_private(), private_and_ours() (+6 more)

### Community 157 - "Anchor"
Cohesion: 0.11
Nodes (20): Anchor, .ALL, Bottom, BottomLeft, BottomRight, Center, Left, Right (+12 more)

### Community 158 - "Aes"
Cohesion: 0.27
Nodes (8): Aes, aes_arm(), aes_hw(), aes_matches_fips_197(), aes_ni(), aes_sw(), counters(), sbox()

### Community 159 - "dependabot.yml"
Cohesion: 0.13
Nodes (14): patterns, prefix, patterns, actions, egui, interval, updates, commit-message (+6 more)

### Community 160 - "ci.yml (2)"
Cohesion: 0.14
Nodes (15): container, name, runs-on, strategy, name, os, distros, native (+7 more)

### Community 161 - "superio.rs (2)"
Cohesion: 0.27
Nodes (10): cfg_rd(), cfg_rd16(), cfg_wr(), Chip, detect(), Fake, finish(), lookup() (+2 more)

### Community 162 - "bottleneck.rs (3)"
Cohesion: 0.14
Nodes (12): Class, .ALL, Background, Cpu, Disk, Gpu, Inconclusive, Limiter (+4 more)

### Community 163 - "engines.rs"
Cohesion: 0.27
Nodes (12): AdapterEngines, aggregate(), engine_label(), labels(), parse(), per_process(), per_process_is_the_busiest_engine(), per_process_memory() (+4 more)

### Community 164 - "windows.rs (5)"
Cohesion: 0.24
Nodes (8): open_never_panics(), PawnSmbus, Xfer, xfer_args(), ReadByte, ReadWord, SendByte, WriteByte

### Community 165 - "arm.rs"
Cohesion: 0.26
Nodes (8): sysctl_str(), sysctl_u64(), build(), cpuinfo_field(), cpuinfo_midr(), implementer(), midr_text(), part()

### Community 166 - "ci.yml (3)"
Cohesion: 0.14
Nodes (14): name, runs-on, name, runs-on, steps, jobs, deb, deny (+6 more)

### Community 167 - "start"
Cohesion: 0.22
Nodes (12): end_session(), final_path_of(), held(), hold(), run(), set_running(), set_status(), sidecar() (+4 more)

### Community 168 - "memplus.rs"
Cohesion: 0.30
Nodes (14): chain(), chain_is_one_cycle(), domains(), gups(), latency(), LINE, pin(), Plan (+6 more)

### Community 169 - "timers.rs"
Cohesion: 0.26
Nodes (13): ACPI_PM_HZ, acpi_timer(), arm_cntfrq(), fadt(), fadt_pm_timer(), hz_text(), measure_pm(), perf_timer() (+5 more)

### Community 170 - "gpu/macos.rs"
Cohesion: 0.24
Nodes (9): measure(), pair(), ROUNDS, statistical(), statistical_mode_measures_unpinned(), summary(), summary_reads_the_upper_triangle(), two_cores_answer() (+1 more)

### Community 171 - "Units"
Cohesion: 0.11
Nodes (14): Clock, Auto, Ghz, Mhz, Size, Decimal, Iec, Windows (+6 more)

### Community 172 - "net_io.rs"
Cohesion: 0.17
Nodes (4): FILTER, HARDWARE, read(), RTM_IFINFO2

### Community 173 - "Bottleneck Detector Design"
Cohesion: 0.12
Nodes (19): Bottleneck detector (measured evidence only), Build health checks on Overview, ADR 0014: No hardware balance bar in 1.0, Configuration checks / Build health (single channel, XMP off, PCIe, ReBAR...), Bottleneck Detector Design, Evidence-tied recommendations (no snake oil), Heuristic fallback without GPU Busy (lower confidence), Limiters (V-Sync, VRR, FPS cap, compositor) (+11 more)

### Community 174 - "rate"
Cohesion: 0.26
Nodes (11): every_test_runs(), p_vs_e(), pinned(), rate(), reference(), scaling(), st_cpu(), suite() (+3 more)

### Community 175 - ".ensure"
Cohesion: 0.20
Nodes (5): Data, State, Audio, collect(), drives()

### Community 176 - "Handle"
Cohesion: 0.13
Nodes (23): ascii(), ata_cmd(), ata_cmd_lba(), ata_smart(), ata_smart_nodata(), cstr(), find_key(), Handle (+15 more)

### Community 177 - "fixture.rs"
Cohesion: 0.26
Nodes (10): adapters(), app(), cpu(), ctx(), drives(), every_hardware_page_draws_from_the_fixtures(), every_overlay_style_stays_within_its_drawing_budget(), harness() (+2 more)

### Community 179 - "telemetry/os.rs"
Cohesion: 0.32
Nodes (4): cpu_load(), parse_proc_stat(), throttle_counts(), ticks()

### Community 180 - "StartupType"
Cohesion: 0.20
Nodes (9): StartupType, Automatic, AutomaticDelayed, Disabled, .EDITABLE, Manual, Masked, Static (+1 more)

### Community 181 - "diskio.rs"
Cohesion: 0.30
Nodes (11): a_reset_counter_is_no_activity_not_garbage(), Counters, DISKSTATS, diskstats_keeps_whole_disks_only(), diskstats_units(), is_whole_disk(), parse_diskstats(), Rates (+3 more)

### Community 182 - ".tick"
Cohesion: 0.22
Nodes (5): frame_age(), readings(), recent(), recent_ms(), target()

### Community 183 - "best_hz"
Cohesion: 0.36
Nodes (8): best_hz(), fixture(), mode(), ModeInfo, parse_modeinfo(), refresh_from_the_timing(), short_or_garbled_input_never_panics(), the_best_mode_is_at_the_current_resolution()

### Community 184 - "storage-and-cleanup.md"
Cohesion: 0.10
Nodes (19): ADR 0002: Privileges, Restart as administrator button, ADR 0005: Task Manager under one System entry, System hub (Processes, Performance, App history, Startup, Users, Services), BOARDS Super I/O map (src/sensors/superio.rs), Ideas for later versions, Storage audit and safe cleanup (idea), Duplicate file finder (size, partial hash, SHA-256) (+11 more)

### Community 185 - "StartupRow"
Cohesion: 0.33
Nodes (7): extract_exe_path(), StartupRow, build_row(), scan_agents(), scan_startup_folder(), scan_xdg_dir(), row_key()

### Community 186 - "ROADMAP.md"
Cohesion: 0.18
Nodes (15): Anti-cheat match test (spike X08), 1.0.0-beta.1 manual test checklist, macOS validation checklist (S0), CHANGELOG 1.0.0 Consolidated (pointer), 1.0 release criteria, CpuZeeZ 1.0 Detailed Roadmap, Sampler measurements on Ryzen 9 7900X, S0 Foundation (+7 more)

### Community 187 - "make_icons.py"
Cohesion: 0.29
Nodes (5): imports(), icns(), ico(), main(), render()

### Community 188 - "superio.rs (4)"
Cohesion: 0.17
Nodes (12): Family, Fintek, Ite, NctEc, NctNew, NctOld, MV10_6, MV11 (+4 more)

### Community 189 - "Graphify neste projeto"
Cohesion: 0.33
Nodes (5): Graphify neste projeto, `.graphifyignore`, Notas, Setup (uma vez por projeto), Uso no dia a dia

### Community 190 - "bench_tab.rs"
Cohesion: 0.42
Nodes (7): apply(), enum_reg_strings(), query(), read_approved_map(), read_fw_post_time(), RegKey, to_wide()

### Community 191 - "actions.rs (2)"
Cohesion: 0.18
Nodes (8): Priority, AboveNormal, .ALL, BelowNormal, High, Idle, Normal, Realtime

### Community 192 - "node"
Cohesion: 0.29
Nodes (9): bus_name(), controller_class(), controller_line(), formats(), gib(), link_line(), node(), pci_controller() (+1 more)

### Community 193 - "edid.rs"
Cohesion: 0.31
Nodes (10): decode(), displayid(), displayid_ext(), dtd(), Monitor, parse(), PNP, pnp_name() (+2 more)

### Community 194 - "layout.rs (2)"
Cohesion: 0.18
Nodes (10): Style, .ALL, Bar, Graph, Number, Ring, Theme, Dark (+2 more)

### Community 195 - "limits.rs"
Cohesion: 0.22
Nodes (11): amd_cppc_highest(), cppc_ranks(), intel_hwp_highest(), intel_limits_decode(), intel_pkg_limits(), Limit, ranks(), Composition (+3 more)

### Community 196 - "ProcessOp"
Cohesion: 0.22
Nodes (9): ProcessOp, Affinity, Dump, Eco, End, Priority, RestartShell, Suspend (+1 more)

### Community 197 - "encryption.rs"
Cohesion: 0.27
Nodes (6): crypt(), DONE, read(), STARTED, Status, status()

### Community 198 - "Preset"
Cohesion: 0.22
Nodes (7): Preset, .ALL, Essential, Everything, Gaming, LeakHunting, Security

### Community 199 - "n"
Cohesion: 0.70
Nodes (4): imported_reports(), n(), settings_from_any_version(), update_answers()

### Community 200 - "RecoveryAction"
Cohesion: 0.25
Nodes (7): RecoveryAction, .EDITABLE, None, Reboot, Restart, RunProgram, action_of()

### Community 203 - "soak.rs"
Cohesion: 0.15
Nodes (10): App, Tour, enabled(), frame(), NAME, overlay_frame(), overlay_only(), PID (+2 more)

### Community 204 - "ServiceState"
Cohesion: 0.25
Nodes (7): ServiceState, Paused, Running, StartPending, Stopped, StopPending, Unknown

### Community 205 - "ServiceFilter"
Cohesion: 0.25
Nodes (7): ServiceFilter, All, Disabled, FailedRecently, Running, StoppedAuto, ThirdParty

### Community 206 - "db.rs"
Cohesion: 0.27
Nodes (7): AMD, INTEL, lookup(), NV, s(), Spec, TABLE

### Community 207 - "f"
Cohesion: 0.13
Nodes (17): class_desc(), Ids, ids_parse_and_labels(), mcfg(), mcfg_base(), parse_ids(), vendor_label(), vendor_short() (+9 more)

### Community 208 - "unsafe_ratchet.py"
Cohesion: 0.36
Nodes (4): count(), selftest(), tree(), verdict()

### Community 209 - "startup.rs (4)"
Cohesion: 0.36
Nodes (9): attr_value(), decode_startup_approved(), encode_startup_approved(), filetime_to_ymd(), impact_thresholds_and_wdi_xml_parser(), parse_wdi_startup_xml(), startup_approved_round_trip_and_filetime_decoding(), tag_u64() (+1 more)

### Community 210 - "queryable"
Cohesion: 0.46
Nodes (5): cpu_suite_modes(), ctx(), memory_and_latency_views(), storage_view_warns_before_writing(), themed()

### Community 211 - "gui/screens.rs"
Cohesion: 0.25
Nodes (3): game_centre(), label(), points()

### Community 212 - "Status"
Cohesion: 0.25
Nodes (8): BenchView, Cpu, Gpu, History, Latency, Memory, Storage, Stress

### Community 213 - "shotdiff.py"
Cohesion: 0.31
Nodes (4): compare(), diff(), read_png(), _test()

### Community 214 - "Status"
Cohesion: 0.29
Nodes (7): Status, BadChecksum, Missing, NeedsAdmin, Off, Running, Starting

### Community 215 - "Kind"
Cohesion: 0.22
Nodes (9): Kind, Bar, Clock, Header, LabelValue, Separator, Sparkline, Throttle (+1 more)

### Community 216 - ".take_widths"
Cohesion: 0.53
Nodes (4): changed_id(), publish(), take_changes(), widths_id()

### Community 217 - "of"
Cohesion: 0.29
Nodes (7): of(), CM_Get_DevNode_Registry_PropertyW(), CreateFileW(), DeviceIoControl(), reg_prop(), reg_u32(), SgIoHdr

### Community 219 - "ci.yml (4)"
Cohesion: 0.22
Nodes (8): env, CARGO_TERM_COLOR, name, on, pull_request, push, workflow_dispatch, branches

### Community 220 - "DetailTab"
Cohesion: 0.33
Nodes (6): DetailTab, Network, Overview, Security, Threads, Windows

### Community 222 - "Gear"
Cohesion: 0.40
Nodes (5): Gear, G2, G2G4, Mode, None

### Community 223 - "Act"
Cohesion: 0.25
Nodes (8): Act, ResetMinMax, SaveHtml, SaveTxt, Stress, ToggleCsv, ToggleMini, ACTIONS

### Community 224 - "netio.rs"
Cohesion: 0.43
Nodes (7): Counters, NET_DEV, parse_proc_net_dev(), proc_net_dev(), Rates, rates(), rates_in_megabits()

### Community 228 - "ci.yml (5)"
Cohesion: 0.25
Nodes (8): with, components, fetch-depth, if-no-files-found, name, path, tool, toolchain

### Community 230 - "macos.rs (4)"
Cohesion: 0.54
Nodes (6): fill(), memory_type(), memory_without_smbios(), partial_and_unavailable_data(), preserves_smbios_and_handles_nested_banks(), probe()

### Community 239 - "Monitor"
Cohesion: 0.13
Nodes (6): Click, App, Mini, Runtime, Monitor, tray_pictures()

### Community 240 - "Theme"
Cohesion: 0.08
Nodes (26): layout_editor_edits(), a_row_pins_under_its_card(), alerts(), ES, lang_of(), page(), privacy(), PT_BR (+18 more)

### Community 242 - "ci.yml (6)"
Cohesion: 0.33
Nodes (6): msi-releases, name, permissions, runs-on, permissions, contents

### Community 243 - "SysTab"
Cohesion: 0.29
Nodes (7): SysTab, AppHistory, Performance, Processes, Services, Startup, Users

### Community 244 - "bottleneck.rs (4)"
Cohesion: 0.40
Nodes (5): LimiterKind, Compositor, FpsCap, Menu, VSync

### Community 245 - ".take_screenshot"
Cohesion: 0.26
Nodes (5): page_name(), App, request(), SHOT, write_png()

### Community 247 - "lib.rs"
Cohesion: 0.50
Nodes (3): DATA_DIR_NAME, DISPLAY_NAME, NAME

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
- **502 isolated node(s):** `AperfMperf`, `Os`, `AddChain`, `Off`, `Starting` (+497 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 1897 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **33 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `Ctx` connect `Ctx` to `sensors.rs`, `egui`, `x86.rs`, `Source`, `System Performance Charts`, `State`, `palette`, `CPU Benchmark Runner`, `src/smbios.rs`, `Memory Benchmark`, `Storage Drives Probe`, `gui/mod.rs`, `Network Adapters`, `CPU Sensors and Power Limits`, `Batteries`, `Community 26`, `mod.rs`, `tr`, `memplus.rs`, `timers.rs`, `f`, `diff.rs`, `.ensure`, `Handle`, `overview.rs`, `mod.rs (2)`, `software.rs`, `mod.rs (3)`, `trf`, `mod.rs (4)`, `telemetry.rs (2)`, `chipset.rs`, `telemetry.rs (3)`, `f`, `gui/screens.rs`, `devices.rs`, `battery.rs`, `usb.rs`, `Monitor`, `npu.rs`, `availability.rs`, `clocks.rs`, `history.rs`?**
  _High betweenness centrality (0.084) - this node is a cross-community bridge._
- **Are the 7 inferred relationships involving `Ctx` (e.g. with `cli()` and `.page()`) actually correct?**
  _`Ctx` has 7 INFERRED edges - model-reasoned connections that need verification._
- **What connects `AperfMperf`, `Os`, `AddChain` to the rest of the system?**
  _502 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `sensors.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.06139240506329114 - nodes in this community are weakly interconnected._
- **Why does `tr()` connect `tr` to `sensors.rs`, `egui`, `layout_view.rs`, `System Performance Charts`, `State`, `palette`, `table.rs`, `gui/tools.rs`, `.render_table`, `system_history.rs`, `gui/mod.rs`, `system.rs`, `Community 26`, `palette.rs`, `system_services.rs`, `diff.rs`, `split.rs`, `overview.rs`, `welcome.rs`, `mini.rs`, `tray.rs (2)`, `live_sessions.rs`, `trf`, `users.rs`, `i18n.rs`, `live.rs`, `devices.rs`, `pins.rs`, `Monitor`, `Theme`, `.take_screenshot`?**
  _High betweenness centrality (0.069) - this node is a cross-community bridge._
- **Are the 148 inferred relationships involving `tr()` (e.g. with `.banner()` and `.frame()`) actually correct?**
  _`tr()` has 148 INFERRED edges - model-reasoned connections that need verification._
- **Should `egui` be split into smaller, more focused modules?**
  _Cohesion score 0.08973172987974098 - nodes in this community are weakly interconnected._