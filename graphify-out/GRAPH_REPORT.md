# Graph Report - cpuzeez  (2026-11-13)

## Corpus Check
- 291 files · ~410,912 words
- Verdict: corpus is large enough that graph structure adds value.
- Unclassified: 38 file(s) not represented in the graph (top: (none) 22, .csv 5, .toml 4)

## Summary
- 6338 nodes · 14658 edges · 273 communities (223 shown, 50 thin omitted)
- Extraction: 92% EXTRACTED · 8% INFERRED · 0% AMBIGUOUS · INFERRED: 1141 edges (avg confidence: 0.81)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `e980476e`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- sensors.rs
- egui
- x86.rs
- Stage
- System Performance Charts
- pci/macos.rs
- Windows Services Backend
- gui/live.rs
- CPU Benchmark Runner
- SPD Decoding
- Memory Benchmark
- Storage Drives Probe
- core/src/smbios.rs
- system_history.rs
- App
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
- Action
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
- gpubench.rs
- system_users.rs
- fuzz.yml
- intel.rs
- software.rs
- gpubench.rs
- proc_net.rs
- setupapi.rs
- tray.rs (2)
- bottleneck.rs
- mod.rs (3)
- live_sessions.rs
- procs.rs (2)
- arm.rs
- mod.rs (4)
- trayicon.rs
- telemetry.rs (2)
- chipset.rs
- telemetry.rs (3)
- desktop.rs
- display.rs
- gpu_engines.rs
- theme.rs
- executor.rs
- amd_pm.rs
- mod.rs (5)
- users.rs
- large_pages.rs
- i18n.rs
- core/src/robustness.rs
- report.rs
- overlay.rs
- Unit
- soak.py
- nvml.rs
- live.rs
- boxed
- windows.rs (4)
- devices.rs
- pins.rs
- screens.rs
- diskbench.rs
- bench_tab.rs
- session.rs
- zeez-live.md
- ci.yml
- battery.rs
- usb.rs
- startup.rs (2)
- a11y.rs
- nightly.yml
- BenchView
- firmware.rs
- availability.rs
- live.rs (2)
- PresentMon bundled sidecar (ETW)
- clocks.rs
- hashmap
- memory_more.rs
- table.rs
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
- settings_ui.rs
- health.rs
- truncated_and_uuid
- body
- .to
- gui/tools.rs
- identify.rs
- Page
- gpu/macos.rs
- FEATURES.md (CPU-Z 2.15 clean-room spec)
- ServiceAccount
- CPU ZeeZ 1.1.0 plan
- CpuZeeZ 1.0 Plan (hub)
- pkgver.py
- Batteries
- .ensure
- Lib
- raster.py
- Array
- Metric Engine
- direct_io.rs
- instant
- tempdir.rs
- Anchor
- Channel
- dependabot.yml
- ci.yml (2)
- superio.rs (2)
- best_hz
- engines.rs
- windows.rs (5)
- on_cpu
- ci.yml (3)
- update_bar
- memplus.rs
- timers.rs
- gpu/macos.rs
- Units
- net_io.rs
- Bottleneck Detector Design
- sha256_of
- w
- probe
- fixture.rs
- I2cDev
- telemetry/os.rs
- ServiceOp
- diskio.rs
- Units
- Problem
- storage-and-cleanup.md
- of
- ROADMAP.md
- make_icons.py
- superio.rs (4)
- Graphify neste projeto
- bench_tab.rs
- actions.rs (2)
- State
- edid.rs
- layout.rs (2)
- limits.rs
- processor_more.rs
- encryption.rs
- n
- Rng
- RecoveryAction
- CLAUDE.md
- benchkit.rs
- soak.rs
- ServiceState
- ServiceFilter
- db.rs
- collect
- unsafe_ratchet.py
- startup.rs (4)
- Direction
- F
- pages/mod.rs
- shotdiff.py
- clock.rs
- Kind
- Id
- Size
- mod.rs (9)
- ci.yml (4)
- beyond
- super
- netio.rs
- Color32
- gui
- ci.yml (5)
- macos.rs (4)
- mainboard_more.rs
- Monitor
- Settings
- ci.yml (6)
- .take_screenshot
- lib.rs
- gui/mod.rs
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
3. `palette()` - 95 edges
4. `trf()` - 85 edges
5. `()` - 64 edges
6. `Series` - 51 edges
7. `card()` - 51 edges
8. `V` - 40 edges
9. `App` - 39 edges
10. `Unit` - 39 edges

## Surprising Connections (you probably didn't know these)
- `tsc_hz()` --calls--> `rdtsc()`  [INFERRED]
  src/cpu/clocks.rs → crates/platform/src/clock.rs
- `live_rows()` --calls--> `g()`  [INFERRED]
  src/gpu/mod.rs → crates/core/src/report.rs
- `Release notes template` --semantically_similar_to--> `Unsigned releases and SHA256SUMS verification`  [INFERRED] [semantically similar]
  packaging/RELEASE_NOTES.md → README.md
- `Test fixtures README (S0-T04)` --semantically_similar_to--> `CPU-Z DDR5 TeamGroup SPD dump (test data)`  [INFERRED] [semantically similar]
  crates/core/fixtures/README.md → src/spd/testdata/cpuz_ddr5_teamgroup.txt
- `the_services_inspector_fits_at_every_scale_in_every_language()` --calls--> `details()`  [INFERRED]
  src/gui/a11y.rs → crates/platform/src/services.rs

## Import Cycles
- 1-file cycle: `src/gui/tray.rs -> src/gui/tray.rs`
- 1-file cycle: `src/gui/tools.rs -> src/gui/tools.rs`
- 1-file cycle: `src/gui/system_history.rs -> src/gui/system_history.rs`
- 1-file cycle: `src/battery.rs -> src/battery.rs`
- 1-file cycle: `src/usb.rs -> src/usb.rs`
- 1-file cycle: `src/storage.rs -> src/storage.rs`
- 1-file cycle: `crates/platform/src/firmware.rs -> crates/platform/src/firmware.rs`
- 1-file cycle: `src/live_frames.rs -> src/live_frames.rs`
- 1-file cycle: `src/gpu/dl.rs -> src/gpu/dl.rs`
- 1-file cycle: `crates/platform/src/elevation.rs -> crates/platform/src/elevation.rs`
- 1-file cycle: `crates/platform/src/large_pages.rs -> crates/platform/src/large_pages.rs`
- 1-file cycle: `src/npu.rs -> src/npu.rs`
- 1-file cycle: `src/software.rs -> src/software.rs`
- 1-file cycle: `crates/platform/src/procs.rs -> crates/platform/src/procs.rs`
- 1-file cycle: `crates/core/src/layout.rs -> crates/core/src/layout.rs`
- 1-file cycle: `src/peripherals.rs -> src/peripherals.rs`
- 1-file cycle: `crates/platform/src/display.rs -> crates/platform/src/display.rs`
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

## Communities (273 total, 50 thin omitted)

### Community 0 - "sensors.rs"
Cohesion: 0.07
Nodes (59): resolve(), avg_clock(), CHART_MAX_H, CHART_MIN_H, charts(), chipset_temp(), core_clocks(), cpu_temp() (+51 more)

### Community 1 - "egui"
Cohesion: 0.04
Nodes (66): Conditions, Job, median_spread(), merge(), points(), repeat(), scores_and_medians(), Sub (+58 more)

### Community 2 - "x86.rs"
Cohesion: 0.21
Nodes (23): amd_legacy_caches(), bit(), bits(), brand(), CacheDesc, caches(), cpuid(), decodes_this_cpu() (+15 more)

### Community 3 - "Stage"
Cohesion: 0.15
Nodes (18): command(), download_and_verify(), fetch_and_verify(), in_path(), Installer, launch(), open_path(), sha256_file() (+10 more)

### Community 4 - "System Performance Charts"
Cohesion: 0.06
Nodes (43): ago_text(), AXIS_FONT, chart(), Line, LINE_WIDTH, MARGIN_BOTTOM, MARGIN_LEFT, nearest() (+35 more)

### Community 5 - "pci/macos.rs"
Cohesion: 0.22
Nodes (14): b64(), device(), list(), next_tag(), parse_plist(), plist(), Pv, Arr (+6 more)

### Community 6 - "Windows Services Backend"
Cohesion: 0.03
Nodes (14): apply(), config2(), dacl_sddl(), details(), enable_shutdown_privilege(), enrich_config(), list(), multi_sz_to_vec() (+6 more)

### Community 7 - "gui/live.rs"
Cohesion: 0.12
Nodes (16): Mode, Automatic, Imported, Manual, Timed, display_hz(), Capture, ES (+8 more)

### Community 8 - "CPU Benchmark Runner"
Cohesion: 0.07
Nodes (39): Bench, CHECK_10K, CHUNK, cli(), deterministic(), fmix(), hostname(), LANES (+31 more)

### Community 9 - "SPD Decoding"
Cohesion: 0.12
Nodes (48): ascii(), bandwidth(), bcd(), cl_rows(), Core, crc16(), crc_known_value(), cycle() (+40 more)

### Community 10 - "Memory Benchmark"
Cohesion: 0.09
Nodes (38): alloc(), Bandwidth, bandwidth(), BLOCK_LINES, build_chain(), chain_is_one_cycle_over_all_lines(), chase(), CHASE_STEP (+30 more)

### Community 11 - "Storage Drives Probe"
Cohesion: 0.06
Nodes (58): ata(), ata_identify(), ata_string(), BUS, bus_name(), CM_Free_Log_Conf_Handle(), CM_Free_Res_Des_Handle(), CM_Get_Device_IDW() (+50 more)

### Community 12 - "core/src/smbios.rs"
Cohesion: 0.09
Nodes (44): agesa(), array_ecc(), array_location(), array_use(), Bios, bios_info(), bios_of(), bits() (+36 more)

### Community 13 - "system_history.rs"
Cohesion: 0.07
Nodes (34): accumulate(), AggregatedApp, app_history_tab_shows_the_view(), AppDayRecord, apps(), clear(), Cmd, Clear (+26 more)

### Community 14 - "App"
Cohesion: 0.13
Nodes (7): CTRL_C, App, AutoShots, frame_log(), hostname(), MASK_PRIVATE, run()

### Community 15 - "Metric Engine"
Cohesion: 0.09
Nodes (26): a_panicking_source_is_paused_and_reported(), a_stuck_source_does_not_stall_the_others(), Actor, clamp_period(), Config, Cost, Engine, engine_stops_with_its_last_handle() (+18 more)

### Community 16 - "Network Adapters"
Cohesion: 0.07
Nodes (46): Adapter, adapters(), AddrNode, band_from_mhz(), BssList, channel_from_mhz(), Close, collect() (+38 more)

### Community 17 - "Ctx"
Cohesion: 0.13
Nodes (19): adapters(), BATTERIES, Cached, Cached<T>, cpu_os(), CPU_OS, CpuOs, DRIVES (+11 more)

### Community 18 - "with"
Cohesion: 0.05
Nodes (52): container, needs, runs-on, steps, env, CARGO_TERM_COLOR, GH_TOKEN, TAG (+44 more)

### Community 19 - "CPU Sensors and Power Limits"
Cohesion: 0.10
Nodes (33): cluster_cpus(), dimms(), get(), cppc(), intel_limits(), core_types(), Ctx, cli() (+25 more)

### Community 20 - "CPU Model Database"
Cohesion: 0.05
Nodes (49): a(), AM2P, AM3, AM3P, AM4, AM5, ANY, APPLE (+41 more)

### Community 21 - "system.rs"
Cohesion: 0.05
Nodes (55): AffinityModal, ALL_COL_IDS, ConfirmEnd, csv_and_json_export_and_snapshot_diff(), CtxCmd, Affinity, Dump, Eco (+47 more)

### Community 22 - "update.rs"
Cohesion: 0.11
Nodes (31): API, apply(), Asset, channels_by_label(), check(), checksum_listing(), cli(), download_dirs_are_fresh_private_and_unique() (+23 more)

### Community 23 - "Process Snapshot Platform"
Cohesion: 0.07
Nodes (40): mach_timebase(), parse_major_faults(), the_rpc_endpoint_mapper_is_listening(), boot_ms(), command_line(), decoders(), elf_arch(), filetime_conversion() (+32 more)

### Community 24 - "Super I/O Sensor Chips"
Cohesion: 0.06
Nodes (56): ASR_AM4_FANS, ASR_AM4_VOLTS, ASR_AM5_VOLTS, ASR_INTEL_FANS, ASR_INTEL_VOLTS, ASR_X2_VOLTS, ASUS_800_VOLTS, ASUS_800_VTT_VOLTS (+48 more)

### Community 25 - "Action"
Cohesion: 0.08
Nodes (33): Action, InstallPawnIo, LoadKernelModule, Process, Service, Startup, audit_lines_are_json(), AuditEntry (+25 more)

### Community 26 - "Community 26"
Cohesion: 0.06
Nodes (26): KernelModule, I2cDev, Msr, BootSummary, ES, kernel_module(), pawnio_doc(), PT_BR (+18 more)

### Community 27 - "Windows GPU Probe"
Cohesion: 0.07
Nodes (56): Adl, adl(), adl_alloc(), AdlInfo, apply_nvapi(), cm_bdf(), CM_Get_DevNode_Registry_PropertyW(), CM_Get_Parent() (+48 more)

### Community 28 - "CPU Test Suite"
Cohesion: 0.08
Nodes (51): add(), Aes, aes_arm(), AES_BLOCKS, aes_hw(), aes_matches_fips_197(), aes_ni(), aes_sw() (+43 more)

### Community 29 - "Single Instance and SIDs"
Cohesion: 0.08
Nodes (17): WaitForSingleObject(), Claim, claim(), First, Running, H, Instance, key() (+9 more)

### Community 30 - "data.rs"
Cohesion: 0.14
Nodes (21): categories_sizes_and_delete(), Category, .ALL, AppLog, AuditLog, BenchmarkHistory, Diagnostics, Exports (+13 more)

### Community 31 - "mod.rs"
Cohesion: 0.08
Nodes (43): write_fuzz_seeds(), cpu_label(), cpu_temps(), display_name(), fake_sysfs(), put(), read(), readings() (+35 more)

### Community 32 - "palette.rs"
Cohesion: 0.14
Nodes (22): Act, ResetMinMax, SaveHtml, SaveTxt, Stress, ToggleCsv, ToggleMini, ACTIONS (+14 more)

### Community 33 - "elevation.rs"
Cohesion: 0.09
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
Cohesion: 0.06
Nodes (16): Bdf, cfg_path(), I2C_SLAVE_FORCE, I2C_SMBUS, I2C_SMBUS_BYTE, I2C_SMBUS_BYTE_DATA, I2C_SMBUS_READ, I2C_SMBUS_WORD_DATA (+8 more)

### Community 38 - "tr"
Cohesion: 0.08
Nodes (36): conditions(), Access, class_color(), draw_analysis(), evidence_text(), explain(), meter(), spark() (+28 more)

### Community 39 - "drivetest.rs"
Cohesion: 0.12
Nodes (35): ata_kind(), ata_log_parses_newest_first(), ata_logs(), ata_selftest(), ata_status_text(), ctrl(), Entry, is_ata() (+27 more)

### Community 40 - "system_services.rs"
Cohesion: 0.12
Nodes (17): dependency_chain_is_cycle_safe_and_filters_work(), matches_filter(), populate_dependents(), ServiceRow, transitive_dependents(), dispatch(), Finished, a_refused_change_leaves_no_undo_and_resets_the_editor() (+9 more)

### Community 41 - "decode.rs"
Cohesion: 0.17
Nodes (28): bars(), bars_and_ext(), bytes_text(), cap(), cap_name(), cap_node(), caps(), dev() (+20 more)

### Community 42 - "smart.rs"
Cohesion: 0.17
Nodes (18): ata_attr_name(), ata_attributes(), ata_smart(), AtaAttr, entry(), from_smartctl(), from_status(), HealthLevel (+10 more)

### Community 43 - "windows.rs"
Cohesion: 0.10
Nodes (33): driver_installed(), driver_modules_found(), CloseFn, CreateMutexW(), ExecFn, fn_name(), GetProcAddress(), Handle (+25 more)

### Community 44 - "peripherals.rs"
Cohesion: 0.09
Nodes (26): Bars, bars(), bars_of(), batteries(), BATTERY, bluetooth_batteries(), DEVICE, FORMAT (+18 more)

### Community 45 - "f"
Cohesion: 0.09
Nodes (21): diagnostics_bundle(), info(), install_panic_hook(), LOCK, MAX, path(), warn(), write() (+13 more)

### Community 46 - "src/os.rs"
Cohesion: 0.09
Nodes (9): final_path(), FreeConsole(), GetConsoleProcessList(), hide_own_console(), HKLM, nul_std_handles(), RRF_RT_ANY, SetStdHandle() (+1 more)

### Community 47 - "diff.rs"
Cohesion: 0.12
Nodes (30): g(), Change, change(), ChangeKind, Added, Modified, Removed, cli() (+22 more)

### Community 48 - "split.rs"
Cohesion: 0.15
Nodes (21): a_narrow_window_opens_the_panel_over_the_table(), changed_id(), clamp(), drawer(), HANDLE, harness(), MAX_FRAC, MIN (+13 more)

### Community 49 - "frames.rs"
Cohesion: 0.11
Nodes (26): curve_histogram_and_window(), exclusive_fullscreen(), fps(), frame_csv_line(), FRAMES_CSV_HEADER, histogram(), is_stutter(), last_secs() (+18 more)

### Community 50 - "hotkeys_portal.rs"
Cohesion: 0.11
Nodes (19): bind(), create_session(), DEST, IFACE, listen(), no_bus_means_unavailable(), PATH, Portal (+11 more)

### Community 51 - "overview.rs"
Cohesion: 0.15
Nodes (25): drives(), customize_presets_and_restore(), editor(), ES, find(), health(), identity(), join() (+17 more)

### Community 52 - "mod.rs (2)"
Cohesion: 0.06
Nodes (51): available(), call(), conn(), IOConnectCallStructMethod(), IOObjectRelease(), IOServiceGetMatchingService(), IOServiceMatching(), IOServiceOpen() (+43 more)

### Community 53 - "proc_ops.rs"
Cohesion: 0.12
Nodes (25): acts_on_a_child_process(), check(), class(), connections(), dump(), eco(), end(), err() (+17 more)

### Community 54 - "Telemetry"
Cohesion: 0.10
Nodes (7): cli_log(), every_source_keeps_the_contract(), Gpus, lock(), sources(), status(), Telemetry

### Community 55 - "procs.rs"
Cohesion: 0.11
Nodes (23): anti_cheat(), classify(), cpu_percent(), grouping_and_critical(), HIGH_CPU, HISTORY, Kind, App (+15 more)

### Community 56 - "display_drm.rs"
Cohesion: 0.11
Nodes (20): card_modes(), CardRes, CONNECTED, connector_name(), connector_names_match_the_kernels(), Crtc, driver_of(), FLAG_DBLSCAN (+12 more)

### Community 57 - "dxcore.rs"
Cohesion: 0.13
Nodes (23): compute_adapters(), ComputeAdapter, CORE_COMPUTE, Count, Create, CreateList, GENERIC_ML, GetAdapter (+15 more)

### Community 58 - "gpubench.rs"
Cohesion: 0.23
Nodes (8): Ctl, flops(), Gpu, pcie(), raster(), run(), stress(), vram()

### Community 59 - "system_users.rs"
Cohesion: 0.14
Nodes (11): UserSession, aggregate_users(), ES, is_service_account(), Modal, ConfirmAction, SendMessage, PT_BR (+3 more)

### Community 60 - "fuzz.yml"
Cohesion: 0.06
Nodes (32): MINUTES, name, runs-on, steps, strategy, timeout-minutes, minutes, jobs (+24 more)

### Community 61 - "intel.rs"
Cohesion: 0.06
Nodes (56): ADL, ARL, arrow_lake(), BCLK_FREQ, decode(), degrade(), F, f() (+48 more)

### Community 62 - "software.rs"
Cohesion: 0.22
Nodes (12): after(), collect(), created_secs(), CV, dates(), fields(), mdy(), os_line() (+4 more)

### Community 63 - "gpubench.rs"
Cohesion: 0.13
Nodes (8): BIG, block_on(), FLOPS_PER_ITER, FMA, GROUPS, list(), REFERENCE, SCENE

### Community 64 - "proc_net.rs"
Cohesion: 0.10
Nodes (25): begin(), inet_diag_message(), KERNEL_NETWORK, KEYWORDS, LAST, nettop_csv(), on_event(), parse_inet_diag() (+17 more)

### Community 65 - "setupapi.rs"
Cohesion: 0.16
Nodes (23): CM_Get_Parent(), DevInfoData, DevPropKey, DIGCF_ALLCLASSES, DIGCF_PRESENT, Guid, pci_devices(), PCI_PROPS (+15 more)

### Community 66 - "tray.rs (2)"
Cohesion: 0.05
Nodes (38): Click, App, Mini, Action, Lock, Mini, Pause, Quit (+30 more)

### Community 67 - "bottleneck.rs"
Cohesion: 0.05
Nodes (85): a_real_presentmon_capture_tells_cpu_bound_loading_from_gpu_bound_play(), a_session_sums_up_time_per_class(), Analysis, analyze(), analyze_verdicts(), avg(), base(), Class (+77 more)

### Community 68 - "mod.rs (3)"
Cohesion: 0.15
Nodes (23): Adapter, adapter_nodes(), adapters(), apply_nvml(), backend(), collect(), fill_from_db(), gen_speed() (+15 more)

### Community 69 - "live_sessions.rs"
Cohesion: 0.13
Nodes (19): a_written_session_reads_back(), a_mangohud_log_imports_as_a_session(), bars(), COLORS, sessions_dir(), ES, import(), key() (+11 more)

### Community 70 - "procs.rs (2)"
Cohesion: 0.12
Nodes (7): Collector, collector_builds_rows_and_rates(), Meta, Prev, Sampler, Snapshot, worker()

### Community 71 - "arm.rs"
Cohesion: 0.23
Nodes (9): sysctl_str(), sysctl_u64(), build(), cpuinfo_field(), cpuinfo_midr(), implementer(), midr_text(), part() (+1 more)

### Community 72 - "mod.rs (4)"
Cohesion: 0.13
Nodes (24): core_mhz(), amd_pstates(), apic_tree(), arm_free_sysctl(), build(), bus_mhz(), Cache, cache_line() (+16 more)

### Community 73 - "trayicon.rs"
Cohesion: 0.11
Nodes (23): Threshold, Tone, Danger, Good, Normal, Warning, detail_levels_filter_cards(), placeholder() (+15 more)

### Community 75 - "telemetry.rs (2)"
Cohesion: 0.12
Nodes (27): throttle_kind(), AT_LIMIT, c(), cli_once(), CLOCK_DROP, Cond, cond(), Debounce (+19 more)

### Community 76 - "chipset.rs"
Cohesion: 0.14
Nodes (26): amd_chipset(), amd_umc(), chipset_names(), chipset_token(), ChipsetSummary, collect(), decode_umc(), decode_umc_more() (+18 more)

### Community 77 - "telemetry.rs (3)"
Cohesion: 0.12
Nodes (16): batteries(), battery(), cpu_extra(), Disks, disks(), DiskSnap, gpu(), gpu_group() (+8 more)

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
Cohesion: 0.05
Nodes (50): Config, parse(), cpu_suite_modes(), ctx(), memory_and_latency_views(), storage_view_warns_before_writing(), themed(), layout_editor_edits() (+42 more)

### Community 82 - "executor.rs"
Cohesion: 0.15
Nodes (21): audit(), audit_log(), audit_path(), bundled_pawnio(), Done, Cancelled, Failed, Ok (+13 more)

### Community 83 - "amd_pm.rs"
Cohesion: 0.16
Nodes (27): APU, cells(), close(), Core, Cores, cores(), decode(), enabled_slots() (+19 more)

### Community 84 - "mod.rs (5)"
Cohesion: 0.08
Nodes (6): Caps, detect(), Hw, NoHw, none(), PAUSED

### Community 85 - "users.rs"
Cohesion: 0.08
Nodes (6): query_wts_str(), send_message(), sessions(), state_label(), to_wide(), the_users_tab_holds_together_at_every_scale()

### Community 87 - "large_pages.rs"
Cohesion: 0.08
Nodes (4): alloc(), HUGE, lock_memory_privilege(), Region

### Community 88 - "i18n.rs"
Cohesion: 0.17
Nodes (18): bottleneck_texts_are_translated(), current(), CURRENT, every_literal_is_translated(), from_code(), Lang, En, Es (+10 more)

### Community 89 - "core/src/robustness.rs"
Cohesion: 0.18
Nodes (12): edid_blocks(), FIXTURE, hardware_and_mangohud_logs(), layout_files(), n(), output_lines_ignore_chunking(), pci_config_space(), presentmon_csv_files() (+4 more)

### Community 90 - "report.rs"
Cohesion: 0.13
Nodes (29): esc(), fields(), find(), find_section(), groups(), gv(), hex(), hex_rows() (+21 more)

### Community 91 - "overlay.rs"
Cohesion: 0.09
Nodes (4): apply(), each(), Find, Style

### Community 92 - "Unit"
Cohesion: 0.07
Nodes (28): Buf, Csv, csv_header_rows_and_new_columns(), History, history_bounded_by_keep_stats_since_start(), r(), Reading, Series (+20 more)

### Community 93 - "soak.py"
Cohesion: 0.14
Nodes (15): analyze(), frame_trend(), _linux_sampler(), _macos_sampler(), make_sampler(), profile(), report(), run_gui() (+7 more)

### Community 94 - "nvml.rs"
Cohesion: 0.17
Nodes (13): db_apple(), sym!, cstr(), Dev, FanRpm, Info, load(), Memory (+5 more)

### Community 95 - "live.rs"
Cohesion: 0.10
Nodes (11): HotkeyAction, .ALL, Capture, CycleStyle, EditMode, Marker, ToggleOverlay, Hotkeys (+3 more)

### Community 96 - "boxed"
Cohesion: 0.16
Nodes (5): boxed(), shown(), App, gpu_name(), App

### Community 97 - "windows.rs (4)"
Cohesion: 0.16
Nodes (6): exits_config(), GlobalMutex, live_dump(), Module, PawnIo, SioSlot

### Community 98 - "devices.rs"
Cohesion: 0.17
Nodes (20): adapter(), battery(), device_card(), drive(), ES, gb(), Health, health() (+12 more)

### Community 99 - "pins.rs"
Cohesion: 0.22
Nodes (17): cache_id(), card_id(), card_text(), current_card(), enter_card(), ES, is_pinned(), latest() (+9 more)

### Community 100 - "screens.rs"
Cohesion: 0.17
Nodes (20): A, B, C, choose(), choosing_where_to_draw(), locate(), locate_and_place_round_trip(), place_on() (+12 more)

### Community 102 - "diskbench.rs"
Cohesion: 0.16
Nodes (16): write_at(), a_tiny_run_measures_and_cleans_up(), a_tiny_sustained_write_cleans_up(), bench(), phase(), rates(), rates_math(), Run (+8 more)

### Community 103 - "bench_tab.rs"
Cohesion: 0.21
Nodes (5): bar(), ES, PT_BR, save_row(), cpu_temp()

### Community 104 - "session.rs"
Cohesion: 0.15
Nodes (23): windows_from_session(), percentile(), corrupt_sessions_stay_bounded(), long_sessions_are_linear(), Band, ChartLine, comparability(), comparisons_and_repeats() (+15 more)

### Community 105 - "zeez-live.md"
Cohesion: 0.06
Nodes (40): ADR 0006: Dependencies, rusqlite (bundled SQLite), windows-sys, ADR 0013: glow renderer with wgpu fallback, glow (OpenGL) renderer, wgpu fallback (CZ_RENDERER=wgpu, WARP), Now card, live readings, session analysis, Benchmark principles (safe, reproducible, 1000 = reference 7900X) (+32 more)

### Community 106 - "ci.yml"
Cohesion: 0.18
Nodes (23): steps, steps, name, runs-on, steps, headless, visual, steps (+15 more)

### Community 107 - "battery.rs"
Cohesion: 0.13
Nodes (20): Battery, chemistry_name(), collect(), GUID_DEVICE_BATTERY, info(), IOCTL_BATTERY_QUERY_INFORMATION, IOCTL_BATTERY_QUERY_STATUS, IOCTL_BATTERY_QUERY_TAG (+12 more)

### Community 108 - "usb.rs"
Cohesion: 0.16
Nodes (16): CM_Get_Parent(), parent(), class_from_compat(), collect(), devices(), hardware_ids(), hub_info(), id_field() (+8 more)

### Community 109 - "startup.rs (2)"
Cohesion: 0.11
Nodes (20): extract_exe_path(), StartupRow, StartupSource, CommonFolder, HkcuRun, HkcuRunOnce, HklmRun, HklmRun32 (+12 more)

### Community 110 - "a11y.rs"
Cohesion: 0.13
Nodes (16): at_every_zoom(), bottleneck_sample(), every_live_tab_holds_together_at_every_scale(), every_page_holds_together_at_every_scale(), every_tools_tab_holds_together_at_every_scale(), page_at_every_zoom(), sidebar_and_tabs_work_from_the_keyboard(), the_app_history_tab_holds_together_at_every_scale() (+8 more)

### Community 111 - "nightly.yml"
Cohesion: 0.09
Nodes (21): minutes, jobs, soak, default, description, name, on, schedule (+13 more)

### Community 114 - "BenchView"
Cohesion: 0.13
Nodes (15): SHOTS, Step, AboutPt, Bench, Bottleneck, Page, System, Tool (+7 more)

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
Cohesion: 0.15
Nodes (15): active_from_deltas(), active_mhz(), add_chain_mhz(), aperf_mperf(), aperf_mperf_regs(), chain(), live_source(), loaded_mhz() (+7 more)

### Community 121 - "memory_more.rs"
Cohesion: 0.22
Nodes (9): active_profile(), App, edac(), ES, finds_the_running_profile_and_tuning(), Live, num(), PT_BR (+1 more)

### Community 122 - "table.rs"
Cohesion: 0.06
Nodes (58): half_typed_and_odd_input_never_fails(), hit(), matches(), number(), Op, Contains, Eq, Ge (+50 more)

### Community 123 - "memtest.rs"
Cohesion: 0.17
Nodes (9): every_pattern_catches_a_flipped_bit(), fill(), MemTest, PATTERNS, runs_and_stops(), seed(), value(), verify() (+1 more)

### Community 124 - "CHANGELOG.md"
Cohesion: 0.09
Nodes (45): ADR 0009: releases stay unsigned for now, ADR 0013: glow with wgpu fallback, 1.1.0 roadmap (docs/v1.1), Security review 2026-11-09, 1.0 plan (docs/v1), ADR 0009: Releases stay unsigned, SHA256SUMS verification for unsigned releases, ADR 0011: Workspace split by layer (+37 more)

### Community 125 - "live_frames.rs"
Cohesion: 0.07
Nodes (51): Frame, App, apps(), begin_capture(), CAPTURE_MAX, Demo, demo(), due() (+43 more)

### Community 126 - "detail.rs"
Cohesion: 0.14
Nodes (14): BASIC, Level, Advanced, .ALL, Basic, Expert, level_of(), levels() (+6 more)

### Community 127 - "history.rs"
Cohesion: 0.21
Nodes (18): add(), dates(), Entry, entry(), join(), load(), load_from(), memory_line() (+10 more)

### Community 128 - "core/src/services.rs"
Cohesion: 0.17
Nodes (17): small_text_parsers(), a_dacl_that_refuses_administrators(), CHANGE_CONFIG, is_protected(), is_third_party(), parse_systemd_secs(), parse_systemd_show(), protected_and_third_party_classification() (+9 more)

### Community 129 - "superio.rs"
Cohesion: 0.23
Nodes (15): apply(), board_lookup(), board_map(), fintek(), Io, ite(), nct6687_like_the_dump(), nct_ec() (+7 more)

### Community 130 - "Source"
Cohesion: 0.06
Nodes (15): actor(), check_contract(), Class, Fast, Slow, Fake, Source, spawn() (+7 more)

### Community 131 - "layout_view.rs"
Cohesion: 0.09
Nodes (23): ES, KINDS, PT_BR, THROTTLE_DEVICES, ALIASES, aliases_resolve_to_this_machines_sensors(), bar(), ES (+15 more)

### Community 132 - "cpu_os.rs"
Cohesion: 0.15
Nodes (11): CODE_INTEGRITY, KVA_SHADOW, NtQuerySystemInformation(), Power, power(), query(), read(), SPECULATION_CONTROL (+3 more)

### Community 133 - "linux.rs (2)"
Cohesion: 0.14
Nodes (19): raw(), read_trim(), read_u64(), uptime_secs(), adapter(), cards(), dpm(), freq() (+11 more)

### Community 134 - "settings_ui.rs"
Cohesion: 0.22
Nodes (12): alerts(), ES, lang_of(), Minis, page(), privacy(), PT_BR, row() (+4 more)

### Community 135 - "health.rs"
Cohesion: 0.26
Nodes (16): a_healthy_build_has_nothing_to_say(), check(), Display, each_mistake(), Facts, Finding, finding(), good() (+8 more)

### Community 136 - "truncated_and_uuid"
Cohesion: 0.33
Nodes (9): entry_point_version(), ioreg_hex(), synth(), truncated_and_uuid(), uuid_bytes(), apple_table(), fetch(), sysfs_fallback() (+1 more)

### Community 137 - "body"
Cohesion: 0.12
Nodes (15): description, label, placeholder, render, value, body, attributes, id (+7 more)

### Community 138 - ".to"
Cohesion: 0.27
Nodes (8): GetCurrentThread(), GetThreadPriority(), GroupAffinity, Pinned, RegGetValueW(), SetThreadGroupAffinity(), SetThreadPriority(), held_handle_blocks_writes_and_renames()

### Community 139 - "gui/tools.rs"
Cohesion: 0.13
Nodes (15): changes(), CHART_BOTTOM, CHART_HEIGHT, CHART_LEFT, CHART_LINE, diff_as_text(), diff_text(), ES (+7 more)

### Community 140 - "identify.rs"
Cohesion: 0.33
Nodes (11): ascii(), ata(), ata_fields_and_dump(), ata_text(), hex_dump(), kelvin(), le16(), le32() (+3 more)

### Community 141 - "Page"
Cohesion: 0.12
Nodes (16): NAV, NAV_BOTTOM, Page, About, Benchmarks, Devices, Graphics, Live (+8 more)

### Community 142 - "gpu/macos.rs"
Cohesion: 0.26
Nodes (9): cmd(), logs(), adapter(), apple_nm(), hex_id(), ioreg_edids(), metal(), probe() (+1 more)

### Community 143 - "FEATURES.md (CPU-Z 2.15 clean-room spec)"
Cohesion: 0.05
Nodes (42): ADR 0012: no code injection, ever, ADR 0014: no hardware balance bar in 1.0, ADR 0015: Wayland shortcuts through the GlobalShortcuts portal, Anti-cheat safe mode, App history (SQLite store), Bottleneck detector (What limits my FPS?), Build health checks, Per-action elevation (one-shot elevated copy, closed operation list) (+34 more)

### Community 144 - "ServiceAccount"
Cohesion: 0.33
Nodes (5): ServiceAccount, .ALL, LocalService, LocalSystem, NetworkService

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

### Community 149 - ".ensure"
Cohesion: 0.20
Nodes (5): Data, State, Audio, collect(), drives()

### Community 150 - "Lib"
Cohesion: 0.52
Nodes (3): GetProcAddress(), Lib, LoadLibraryW()

### Community 151 - "raster.py"
Cohesion: 0.20
Nodes (8): dist_seg(), in_poly(), in_rrect(), on_arc(), png(), render(), render_png(), sample()

### Community 153 - "Metric Engine"
Cohesion: 0.19
Nodes (11): windows-sys crate, D10 Hand-written FFI, D1 APERF/MPERF clock busy-spin, D2 rdmsr spawns thread per read, D3 SMART probe inside sampler, D5 series() clones history; duplicate GUI sampler, Diagnosis of 0.3.0 (D1-D10), Metric subscriptions and rate classes (+3 more)

### Community 154 - "direct_io.rs"
Cohesion: 0.18
Nodes (7): ALIGN, Buffer, FILE_FLAG_NO_BUFFERING, FILE_FLAG_WRITE_THROUGH, open(), read_at(), uncached_round_trip()

### Community 155 - "instant"
Cohesion: 0.24
Nodes (7): Tour, collect(), cpu_load(), cpu_mhz(), monitors(), Row, sample()

### Community 156 - "tempdir.rs"
Cohesion: 0.32
Nodes (14): base_is_safe(), base_validation_follows_symlinks_in_the_path(), base_validation_rejects_swappable_ancestors(), base_validation_rejects_swappable_symlinks(), base_validation_requires_sticky_when_shared(), create(), create_exclusive_private(), private_and_ours() (+6 more)

### Community 157 - "Anchor"
Cohesion: 0.10
Nodes (24): Anchor, .ALL, Bottom, BottomLeft, BottomRight, Center, Left, Right (+16 more)

### Community 158 - "Channel"
Cohesion: 0.22
Nodes (6): Channel, .ALL, Alpha, Beta, Snapshot, Stable

### Community 159 - "dependabot.yml"
Cohesion: 0.13
Nodes (14): patterns, prefix, patterns, actions, egui, interval, updates, commit-message (+6 more)

### Community 160 - "ci.yml (2)"
Cohesion: 0.14
Nodes (15): container, name, runs-on, strategy, name, os, distros, native (+7 more)

### Community 161 - "superio.rs (2)"
Cohesion: 0.28
Nodes (9): cfg_rd(), cfg_rd16(), cfg_wr(), Chip, detect(), Fake, finish(), probe() (+1 more)

### Community 162 - "best_hz"
Cohesion: 0.36
Nodes (8): best_hz(), fixture(), mode(), ModeInfo, parse_modeinfo(), refresh_from_the_timing(), short_or_garbled_input_never_panics(), the_best_mode_is_at_the_current_resolution()

### Community 163 - "engines.rs"
Cohesion: 0.31
Nodes (11): AdapterEngines, aggregate(), engine_label(), labels(), parse(), per_process(), per_process_is_the_busiest_engine(), per_process_memory() (+3 more)

### Community 164 - "windows.rs (5)"
Cohesion: 0.24
Nodes (8): open_never_panics(), PawnSmbus, Xfer, xfer_args(), ReadByte, ReadWord, SendByte, WriteByte

### Community 165 - "on_cpu"
Cohesion: 0.31
Nodes (8): cpu_groups(), GetActiveProcessorCount(), GetActiveProcessorGroupCount(), logical_cpus(), on_cpu(), on_cpu_restores_affinity(), on_cpu_runs_on_the_calling_thread(), pin()

### Community 166 - "ci.yml (3)"
Cohesion: 0.14
Nodes (14): name, runs-on, name, runs-on, steps, jobs, deb, deny (+6 more)

### Community 167 - "update_bar"
Cohesion: 0.32
Nodes (4): App, failed_text(), update_bar(), Updates

### Community 168 - "memplus.rs"
Cohesion: 0.33
Nodes (13): chain(), chain_is_one_cycle(), gups(), latency(), LINE, pin(), Plan, run() (+5 more)

### Community 169 - "timers.rs"
Cohesion: 0.26
Nodes (13): ACPI_PM_HZ, acpi_timer(), arm_cntfrq(), fadt(), fadt_pm_timer(), hz_text(), measure_pm(), perf_timer() (+5 more)

### Community 170 - "gpu/macos.rs"
Cohesion: 0.24
Nodes (9): measure(), pair(), ROUNDS, statistical(), statistical_mode_measures_unpinned(), summary(), summary_reads_the_upper_triangle(), two_cores_answer() (+1 more)

### Community 171 - "Units"
Cohesion: 0.16
Nodes (9): defaults(), fires_only_when_the_whole_window_is_over(), firing(), pts(), Rule, rules_load_from_older_settings(), Temp, Celsius (+1 more)

### Community 172 - "net_io.rs"
Cohesion: 0.17
Nodes (4): FILTER, HARDWARE, read(), RTM_IFINFO2

### Community 173 - "Bottleneck Detector Design"
Cohesion: 0.12
Nodes (19): Bottleneck detector (measured evidence only), Build health checks on Overview, ADR 0014: No hardware balance bar in 1.0, Configuration checks / Build health (single channel, XMP off, PCIe, ReBAR...), Bottleneck Detector Design, Evidence-tied recommendations (no snake oil), Heuristic fallback without GPU Busy (lower confidence), Limiters (V-Sync, VRR, FPS cap, compositor) (+11 more)

### Community 174 - "sha256_of"
Cohesion: 0.46
Nodes (5): compress(), hex(), Sha256, sha256_fips_vectors(), sha256_of()

### Community 175 - "w"
Cohesion: 0.38
Nodes (7): reg_raw(), reg_str(), reg_subkeys(), reg_u64(), w(), audio(), wave_format()

### Community 176 - "probe"
Cohesion: 0.13
Nodes (23): ascii(), ata_cmd(), ata_cmd_lba(), ata_smart(), ata_smart_nodata(), cstr(), find_key(), Handle (+15 more)

### Community 177 - "fixture.rs"
Cohesion: 0.26
Nodes (10): adapters(), app(), cpu(), ctx(), drives(), every_hardware_page_draws_from_the_fixtures(), every_overlay_style_stays_within_its_drawing_budget(), harness() (+2 more)

### Community 179 - "telemetry/os.rs"
Cohesion: 0.32
Nodes (4): cpu_load(), parse_proc_stat(), throttle_counts(), ticks()

### Community 180 - "ServiceOp"
Cohesion: 0.08
Nodes (23): Risk, Critical, Irreversible, Normal, ServiceOp, Pause, Restart, SetAccount (+15 more)

### Community 181 - "diskio.rs"
Cohesion: 0.30
Nodes (11): a_reset_counter_is_no_activity_not_garbage(), Counters, DISKSTATS, diskstats_keeps_whole_disks_only(), diskstats_units(), is_whole_disk(), parse_diskstats(), Rates (+3 more)

### Community 182 - "Units"
Cohesion: 0.19
Nodes (8): Clock, Auto, Ghz, Mhz, Units, .DEFAULT, CURRENT, set()

### Community 183 - "Problem"
Cohesion: 0.33
Nodes (5): Problem, FromTemp, HighCpu, MemoryGrowing, NotResponding

### Community 184 - "storage-and-cleanup.md"
Cohesion: 0.10
Nodes (19): ADR 0002: Privileges, Restart as administrator button, ADR 0005: Task Manager under one System entry, System hub (Processes, Performance, App history, Startup, Users, Services), BOARDS Super I/O map (src/sensors/superio.rs), Ideas for later versions, Storage audit and safe cleanup (idea), Duplicate file finder (size, partial hash, SHA-256) (+11 more)

### Community 185 - "of"
Cohesion: 0.40
Nodes (5): of(), CM_Get_DevNode_Registry_PropertyW(), CreateFileW(), DeviceIoControl(), SgIoHdr

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
Cohesion: 0.31
Nodes (9): StartupOp, SetEnabled, apply(), enum_reg_strings(), query(), read_approved_map(), read_fw_post_time(), RegKey (+1 more)

### Community 191 - "actions.rs (2)"
Cohesion: 0.18
Nodes (8): Priority, AboveNormal, .ALL, BelowNormal, High, Idle, Normal, Realtime

### Community 192 - "State"
Cohesion: 0.16
Nodes (5): Mode, Cpu, Memory, State, MemBench

### Community 193 - "edid.rs"
Cohesion: 0.31
Nodes (10): decode(), displayid(), displayid_ext(), dtd(), Monitor, parse(), PNP, pnp_name() (+2 more)

### Community 194 - "layout.rs (2)"
Cohesion: 0.18
Nodes (10): Style, .ALL, Bar, Graph, Number, Ring, Theme, Dark (+2 more)

### Community 195 - "limits.rs"
Cohesion: 0.43
Nodes (7): amd_cppc_highest(), cppc_ranks(), intel_hwp_highest(), intel_limits_decode(), intel_pkg_limits(), Limit, ranks()

### Community 196 - "processor_more.rs"
Cohesion: 0.23
Nodes (8): heading(), App, CoreRow, ES, group_of(), instructions(), PT_BR, topology()

### Community 197 - "encryption.rs"
Cohesion: 0.31
Nodes (6): crypt(), DONE, read(), STARTED, Status, status()

### Community 198 - "n"
Cohesion: 0.70
Nodes (4): imported_reports(), n(), settings_from_any_version(), update_answers()

### Community 200 - "RecoveryAction"
Cohesion: 0.25
Nodes (7): RecoveryAction, .EDITABLE, None, Reboot, Restart, RunProgram, action_of()

### Community 203 - "soak.rs"
Cohesion: 0.21
Nodes (9): App, enabled(), frame(), NAME, overlay_frame(), overlay_only(), PID, start() (+1 more)

### Community 204 - "ServiceState"
Cohesion: 0.25
Nodes (7): ServiceState, Paused, Running, StartPending, Stopped, StopPending, Unknown

### Community 205 - "ServiceFilter"
Cohesion: 0.25
Nodes (7): ServiceFilter, All, Disabled, FailedRecently, Running, StoppedAuto, ThirdParty

### Community 206 - "db.rs"
Cohesion: 0.27
Nodes (7): AMD, INTEL, lookup(), NV, s(), Spec, TABLE

### Community 207 - "collect"
Cohesion: 0.18
Nodes (14): find_agesa(), rom_agesa(), section(), uefi(), collect(), collect(), devices(), list() (+6 more)

### Community 208 - "unsafe_ratchet.py"
Cohesion: 0.36
Nodes (4): count(), selftest(), tree(), verdict()

### Community 209 - "startup.rs (4)"
Cohesion: 0.36
Nodes (9): attr_value(), decode_startup_approved(), encode_startup_approved(), filetime_to_ymd(), impact_thresholds_and_wdi_xml_parser(), parse_wdi_startup_xml(), startup_approved_round_trip_and_filetime_decoding(), tag_u64() (+1 more)

### Community 210 - "Direction"
Cohesion: 0.50
Nodes (4): Direction, Grid, Horizontal, Vertical

### Community 211 - "F"
Cohesion: 0.15
Nodes (17): class_desc(), Ids, ids_parse_and_labels(), mcfg(), mcfg_base(), parse_ids(), vendor_label(), vendor_short() (+9 more)

### Community 212 - "pages/mod.rs"
Cohesion: 0.09
Nodes (21): BenchView, Cpu, Gpu, History, Latency, Memory, Storage, Stress (+13 more)

### Community 213 - "shotdiff.py"
Cohesion: 0.31
Nodes (4): compare(), diff(), read_png(), _test()

### Community 215 - "Kind"
Cohesion: 0.22
Nodes (9): Kind, Bar, Clock, Header, LabelValue, Separator, Sparkline, Throttle (+1 more)

### Community 216 - "Id"
Cohesion: 0.20
Nodes (6): egui::InnerResponse<T>, FINTEK, Id, ITE, lookup(), WINBOND_NUVOTON

### Community 217 - "Size"
Cohesion: 0.50
Nodes (4): Size, Decimal, Iec, Windows

### Community 219 - "ci.yml (4)"
Cohesion: 0.22
Nodes (8): env, CARGO_TERM_COLOR, name, on, pull_request, push, workflow_dispatch, branches

### Community 222 - "super"
Cohesion: 0.47
Nodes (4): Composition, meminfo(), MEMINFO, parse_meminfo()

### Community 224 - "netio.rs"
Cohesion: 0.43
Nodes (7): Counters, NET_DEV, parse_proc_net_dev(), proc_net_dev(), Rates, rates(), rates_in_megabits()

### Community 226 - "Color32"
Cohesion: 0.21
Nodes (7): chart_colours_meet_the_graphics_minimum(), class_colours_are_distinct_and_readable(), contrast(), luminance(), surfaces(), text_colours_meet_aa_on_every_surface(), encode()

### Community 227 - "gui"
Cohesion: 0.17
Nodes (3): mainboard_tab(), App, timers_body()

### Community 228 - "ci.yml (5)"
Cohesion: 0.25
Nodes (8): with, components, fetch-depth, if-no-files-found, name, path, tool, toolchain

### Community 230 - "macos.rs (4)"
Cohesion: 0.54
Nodes (6): fill(), memory_type(), memory_without_smbios(), partial_and_unavailable_data(), preserves_smbios_and_handles_nested_banks(), probe()

### Community 237 - "mainboard_more.rs"
Cohesion: 0.18
Nodes (6): App, ES, link_status(), link_text(), PT_BR, tree()

### Community 239 - "Monitor"
Cohesion: 0.14
Nodes (4): Runtime, Monitor, tray_pictures(), Shown

### Community 240 - "Settings"
Cohesion: 0.14
Nodes (5): file(), newer_files_are_never_overwritten_and_bad_ones_are_kept(), Pin, SCHEMA, Settings

### Community 242 - "ci.yml (6)"
Cohesion: 0.33
Nodes (6): msi-releases, name, permissions, runs-on, permissions, contents

### Community 245 - ".take_screenshot"
Cohesion: 0.26
Nodes (5): page_name(), App, request(), SHOT, write_png()

### Community 247 - "lib.rs"
Cohesion: 0.50
Nodes (3): DATA_DIR_NAME, DISPLAY_NAME, NAME

### Community 249 - "gui/mod.rs"
Cohesion: 0.12
Nodes (16): app_icon(), cpuid_dump(), decode_png(), ES, PT_BR, sec(), sget(), sgi() (+8 more)

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
- **502 isolated node(s):** `Overview`, `System`, `Live`, `Processor`, `Mainboard` (+497 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 1920 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **50 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `Ctx` connect `CPU Sensors and Power Limits` to `sensors.rs`, `egui`, `x86.rs`, `layout_view.rs`, `System Performance Charts`, `Source`, `gui/live.rs`, `CPU Benchmark Runner`, `Memory Benchmark`, `Storage Drives Probe`, `App`, `Network Adapters`, `Ctx`, `Batteries`, `.ensure`, `system.rs`, `Community 26`, `mod.rs`, `tr`, `timers.rs`, `f`, `diff.rs`, `probe`, `overview.rs`, `mod.rs (2)`, `Telemetry`, `software.rs`, `State`, `bottleneck.rs`, `mod.rs (3)`, `mod.rs (4)`, `telemetry.rs (2)`, `chipset.rs`, `telemetry.rs (3)`, `collect`, `F`, `devices.rs`, `bench_tab.rs`, `battery.rs`, `usb.rs`, `Monitor`, `availability.rs`, `clocks.rs`, `gui/mod.rs`, `history.rs`?**
  _High betweenness centrality (0.101) - this node is a cross-community bridge._
- **Are the 7 inferred relationships involving `Ctx` (e.g. with `cli()` and `.page()`) actually correct?**
  _`Ctx` has 7 INFERRED edges - model-reasoned connections that need verification._
- **What connects `Overview`, `System`, `Live` to the rest of the system?**
  _502 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `sensors.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.06664198445020363 - nodes in this community are weakly interconnected._
- **Why does `tr()` connect `tr` to `sensors.rs`, `egui`, `layout_view.rs`, `System Performance Charts`, `settings_ui.rs`, `gui/live.rs`, `gui/tools.rs`, `system_history.rs`, `App`, `system.rs`, `Community 26`, `palette.rs`, `update_bar`, `system_services.rs`, `diff.rs`, `split.rs`, `overview.rs`, `system_users.rs`, `intel.rs`, `tray.rs (2)`, `bottleneck.rs`, `processor_more.rs`, `live_sessions.rs`, `i18n.rs`, `live.rs`, `boxed`, `devices.rs`, `gui`, `pins.rs`, `gui/mod.rs`, `bench_tab.rs`, `mainboard_more.rs`, `Monitor`, `.take_screenshot`, `memory_more.rs`, `table.rs`, `detail.rs`?**
  _High betweenness centrality (0.062) - this node is a cross-community bridge._
- **Are the 148 inferred relationships involving `tr()` (e.g. with `.banner()` and `.frame()`) actually correct?**
  _`tr()` has 148 INFERRED edges - model-reasoned connections that need verification._
- **Should `egui` be split into smaller, more focused modules?**
  _Cohesion score 0.04254385964912281 - nodes in this community are weakly interconnected._