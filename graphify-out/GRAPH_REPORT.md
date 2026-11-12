# Graph Report - cpuzeez  (2026-11-12)

## Corpus Check
- 285 files · ~406,140 words
- Verdict: corpus is large enough that graph structure adds value.
- Unclassified: 37 file(s) not represented in the graph (top: (none) 22, .csv 5, .toml 4)

## Summary
- 6187 nodes · 14547 edges · 262 communities (228 shown, 34 thin omitted)
- Extraction: 92% EXTRACTED · 8% INFERRED · 0% AMBIGUOUS · INFERRED: 1140 edges (avg confidence: 0.81)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `23e22ac5`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- Sensors Page and Stress Monitor
- Benchmark Suite Views
- x86.rs
- palette
- System Performance Charts
- Users Tab and Access Banner
- Windows Services Backend
- ZeeZ Live Overlay and Sessions
- CPU Benchmark Runner
- SPD Decoding
- Memory Benchmark
- Storage Drives Probe
- SMBIOS Decoding
- system_history.rs
- GUI App Shell
- Metric Engine
- Network Adapters
- Probe Cache and ACPI
- Release Workflow
- CPU Sensors and Power Limits
- CPU Model Database
- Processes Tab
- update.rs
- Process Snapshot Platform
- Super I/O Sensor Chips
- Elevated Actions
- actor
- Windows GPU Probe
- CPU Test Suite
- Single Instance and SIDs
- data.rs
- mod.rs
- palette.rs
- elevation.rs
- disk_io.rs
- layout.rs
- egui
- linux.rs
- tray.rs
- drivetest.rs
- system_services.rs
- decode.rs
- smart.rs
- windows.rs
- peripherals.rs
- main.rs
- os.rs
- diff.rs
- tools.rs
- frames.rs
- hotkeys_portal.rs
- overview.rs
- mod.rs (2)
- proc_ops.rs
- telemetry.rs
- procs.rs
- display_drm.rs
- settings.rs
- gpubench.rs
- layout_view.rs
- fuzz.yml
- intel.rs
- f
- metrics.rs
- proc_net.rs
- theme.rs
- tray.rs (2)
- bottleneck.rs
- mod.rs (3)
- live_sessions.rs
- procs.rs (2)
- bottleneck_ui.rs
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
- Stage
- robustness.rs
- report.rs
- overlay.rs
- pci/windows.rs
- soak.py
- nvml.rs
- live.rs
- services.rs
- windows.rs (4)
- devices.rs
- pins.rs
- screens.rs
- startup.rs
- diskbench.rs
- update.rs
- session.rs
- zeez-live.md
- ci.yml
- battery.rs
- usb.rs
- startup.rs (2)
- a11y.rs
- nightly.yml
- Ctx
- npu.rs
- Step
- probe
- availability.rs
- live.rs (2)
- PresentMon bundled sidecar (ETW)
- processor_more.rs
- pci/windows.rs
- cstates.rs
- memory.rs
- memtest.rs
- CHANGELOG.md
- live_frames.rs
- detail.rs
- clocks.rs
- table.rs
- superio.rs
- layout_view.rs
- query.rs
- cpu_os.rs
- linux.rs (2)
- history.rs
- health.rs
- live.rs (3)
- dependencies.md
- table.rs (2)
- table.rs (3)
- win.rs
- Page
- Window
- FEATURES.md (CPU-Z 2.15 clean-room spec)
- actions.rs
- CPU ZeeZ 1.1.0 plan
- CpuZeeZ 1.0 Plan (hub)
- pkgver.py
- mod.rs (7)
- applog.rs
- live_mangohud.rs
- raster.py
- i18n.rs
- Period
- direct_io.rs
- .take_screenshot
- tempdir.rs
- firmware.rs
- pci/linux.rs
- dependabot.yml
- ci.yml (2)
- superio.rs (2)
- bottleneck.rs (3)
- engines.rs
- windows.rs (5)
- ServiceFilter
- ci.yml (3)
- gpu/macos.rs
- memplus.rs
- timers.rs
- layout_editor.rs
- units.rs
- net_io.rs
- Bottleneck Detector Design
- start
- cpusuite.rs
- arm.rs
- fixture.rs
- superio.rs (3)
- os.rs (2)
- latency.rs
- diskio.rs
- .show_ui
- units.rs (2)
- storage-and-cleanup.md
- WindowOp
- ROADMAP.md
- make_icons.py
- superio.rs (4)
- Graphify neste projeto
- best_hz
- actions.rs (2)
- Sampler
- edid.rs
- layout.rs (2)
- mangohud.rs
- smbios.rs (2)
- encryption.rs
- arc
- .stage_shot
- analyze
- CLAUDE.md
- benchkit.rs
- soak.rs
- mainboard_more.rs
- smart
- db.rs
- mod.rs (8)
- bench_views_test.rs
- startup.rs (4)
- Status
- Status
- struct
- shotdiff.py
- .tick
- layout.rs (3)
- memory.rs (2)
- mod.rs (9)
- ci.yml (4)
- detector.rs
- alerts.rs
- limits.rs
- netio.rs
- services.rs (5)
- ci.yml (5)
- macos.rs (4)
- dl.rs
- update.rs (2)
- query.rs (2)
- ci.yml (6)
- SysTab
- bottleneck.rs (4)
- layout.rs (4)
- lib.rs
- Cargo.toml
- fetch_pawnio.sh
- ci.yml (7)
- CHANGELOG.md (3)

## God Nodes (most connected - your core abstractions)
1. `Ctx` - 222 edges
2. `tr()` - 187 edges
3. `palette()` - 114 edges
4. `trf()` - 81 edges
5. `card()` - 60 edges
6. `Series` - 51 edges
7. `V` - 40 edges
8. `Telemetry` - 39 edges
9. `pill()` - 38 edges
10. `CpuZeeZ 1.0 Plan (hub)` - 38 edges

## Surprising Connections (you probably didn't know these)
- `live_rows()` --calls--> `g()`  [INFERRED]
  src/gpu/mod.rs → crates/core/src/report.rs
- `Release notes template` --semantically_similar_to--> `Unsigned releases and SHA256SUMS verification`  [INFERRED] [semantically similar]
  packaging/RELEASE_NOTES.md → README.md
- `Test fixtures README (S0-T04)` --semantically_similar_to--> `CPU-Z DDR5 TeamGroup SPD dump (test data)`  [INFERRED] [semantically similar]
  crates/core/fixtures/README.md → src/spd/testdata/cpuz_ddr5_teamgroup.txt
- `the_users_tab_holds_together_at_every_scale()` --calls--> `sessions()`  [INFERRED]
  src/gui/a11y.rs → crates/platform/src/users.rs
- `the_services_inspector_fits_at_every_scale_in_every_language()` --calls--> `details()`  [INFERRED]
  src/gui/a11y.rs → crates/platform/src/services.rs

## Import Cycles
- 1-file cycle: `crates/core/src/layout.rs -> crates/core/src/layout.rs`
- 1-file cycle: `src/gui/tools.rs -> src/gui/tools.rs`
- 1-file cycle: `src/gui/tray.rs -> src/gui/tray.rs`
- 1-file cycle: `crates/platform/src/desktop.rs -> crates/platform/src/desktop.rs`
- 1-file cycle: `src/gui/system_history.rs -> src/gui/system_history.rs`
- 1-file cycle: `src/live_frames.rs -> src/live_frames.rs`
- 1-file cycle: `src/battery.rs -> src/battery.rs`
- 1-file cycle: `src/usb.rs -> src/usb.rs`
- 1-file cycle: `crates/platform/src/elevation.rs -> crates/platform/src/elevation.rs`
- 1-file cycle: `src/storage.rs -> src/storage.rs`
- 1-file cycle: `crates/platform/src/large_pages.rs -> crates/platform/src/large_pages.rs`
- 1-file cycle: `src/npu.rs -> src/npu.rs`
- 1-file cycle: `src/software.rs -> src/software.rs`
- 1-file cycle: `crates/platform/src/procs.rs -> crates/platform/src/procs.rs`
- 1-file cycle: `src/gpu/dl.rs -> src/gpu/dl.rs`
- 1-file cycle: `src/peripherals.rs -> src/peripherals.rs`
- 1-file cycle: `crates/platform/src/display.rs -> crates/platform/src/display.rs`
- 1-file cycle: `crates/platform/src/gpu_engines.rs -> crates/platform/src/gpu_engines.rs`
- 1-file cycle: `crates/platform/src/memory.rs -> crates/platform/src/memory.rs`
- 2-file cycle: `src/gui/sensors.rs -> src/gui/stress.rs -> src/gui/sensors.rs`

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

## Communities (262 total, 34 thin omitted)

### Community 0 - "Sensors Page and Stress Monitor"
Cohesion: 0.06
Nodes (58): conditions(), cpu_temp(), avg_clock(), CHART_MAX_H, CHART_MIN_H, charts(), chipset_temp(), core_clocks() (+50 more)

### Community 1 - "Benchmark Suite Views"
Cohesion: 0.12
Nodes (27): Conditions, Job, median_spread(), merge(), points(), repeat(), scores_and_medians(), Sub (+19 more)

### Community 2 - "x86.rs"
Cohesion: 0.21
Nodes (23): amd_legacy_caches(), bit(), bits(), brand(), CacheDesc, caches(), cpuid(), decodes_this_cpu() (+15 more)

### Community 3 - "palette"
Cohesion: 0.06
Nodes (33): conditions(), BUCKETS, bar(), ES, PT_BR, save_row(), draw_analysis(), evidence_text() (+25 more)

### Community 4 - "System Performance Charts"
Cohesion: 0.07
Nodes (42): ago_text(), AXIS_FONT, chart(), Line, LINE_WIDTH, MARGIN_BOTTOM, MARGIN_LEFT, nearest() (+34 more)

### Community 5 - "Users Tab and Access Banner"
Cohesion: 0.05
Nodes (32): KernelModule, I2cDev, Msr, BootSummary, Access, ES, kernel_module(), pawnio_doc() (+24 more)

### Community 6 - "Windows Services Backend"
Cohesion: 0.03
Nodes (16): cmd(), apply(), config2(), dacl_sddl(), details(), enable_shutdown_privilege(), enrich_config(), list() (+8 more)

### Community 7 - "ZeeZ Live Overlay and Sessions"
Cohesion: 0.08
Nodes (31): Marker, Mode, Automatic, Imported, Manual, Timed, tr(), prefixes() (+23 more)

### Community 8 - "CPU Benchmark Runner"
Cohesion: 0.07
Nodes (39): Bench, CHECK_10K, CHUNK, cli(), deterministic(), fmix(), hostname(), LANES (+31 more)

### Community 9 - "SPD Decoding"
Cohesion: 0.09
Nodes (56): ascii(), bandwidth(), bcd(), cl_rows(), Core, crc16(), crc_known_value(), cycle() (+48 more)

### Community 10 - "Memory Benchmark"
Cohesion: 0.08
Nodes (39): alloc(), Bandwidth, bandwidth(), BLOCK_LINES, build_chain(), chain_is_one_cycle_over_all_lines(), chase(), CHASE_STEP (+31 more)

### Community 11 - "Storage Drives Probe"
Cohesion: 0.06
Nodes (62): of(), BUS, bus_name(), CM_Free_Log_Conf_Handle(), CM_Free_Res_Des_Handle(), CM_Get_Device_IDW(), CM_Get_Device_Interface_List_SizeW(), CM_Get_Device_Interface_ListW() (+54 more)

### Community 12 - "SMBIOS Decoding"
Cohesion: 0.10
Nodes (43): agesa(), array_ecc(), array_location(), array_use(), Bios, bios_info(), bios_of(), bits() (+35 more)

### Community 13 - "system_history.rs"
Cohesion: 0.17
Nodes (20): accumulate(), add(), AggregatedApp, AppDayRecord, apps(), clear(), days(), ES (+12 more)

### Community 14 - "GUI App Shell"
Cohesion: 0.05
Nodes (35): App, app_icon(), AutoShots, boxed(), cpuid_dump(), decode_png(), ES, frame_log() (+27 more)

### Community 15 - "Metric Engine"
Cohesion: 0.08
Nodes (28): a_panicking_source_is_paused_and_reported(), a_stuck_source_does_not_stall_the_others(), Actor, clamp_period(), Config, Cost, Engine, engine_stops_with_its_last_handle() (+20 more)

### Community 16 - "Network Adapters"
Cohesion: 0.07
Nodes (47): Adapter, adapters(), AddrNode, band_from_mhz(), BssList, channel_from_mhz(), Close, collect() (+39 more)

### Community 17 - "Probe Cache and ACPI"
Cohesion: 0.14
Nodes (19): batteries(), BATTERIES, Cached, Cached<T>, cpu_os(), CPU_OS, CpuOs, DRIVES (+11 more)

### Community 18 - "Release Workflow"
Cohesion: 0.05
Nodes (52): container, needs, runs-on, steps, env, CARGO_TERM_COLOR, GH_TOKEN, TAG (+44 more)

### Community 19 - "CPU Sensors and Power Limits"
Cohesion: 0.17
Nodes (22): get(), core_types(), amd(), amd_limits(), amd_rapl(), ccd_base(), core_clocks(), core_threads() (+14 more)

### Community 20 - "CPU Model Database"
Cohesion: 0.05
Nodes (48): a(), AM2P, AM3, AM3P, AM4, AM5, ANY, APPLE (+40 more)

### Community 21 - "Processes Tab"
Cohesion: 0.05
Nodes (54): AffinityModal, ALL_COL_IDS, ConfirmEnd, csv_and_json_export_and_snapshot_diff(), CtxCmd, Affinity, Dump, Eco (+46 more)

### Community 22 - "update.rs"
Cohesion: 0.11
Nodes (30): API, Asset, channels_by_label(), check(), checksum_listing(), download_and_verify(), download_dirs_are_fresh_private_and_unique(), downloads_and_verifies_latest_release() (+22 more)

### Community 23 - "Process Snapshot Platform"
Cohesion: 0.08
Nodes (39): parse_major_faults(), the_rpc_endpoint_mapper_is_listening(), boot_ms(), command_line(), decoders(), elf_arch(), filetime_conversion(), filetime_ms() (+31 more)

### Community 24 - "Super I/O Sensor Chips"
Cohesion: 0.08
Nodes (46): ASR_AM4_FANS, ASR_AM4_VOLTS, ASR_AM5_VOLTS, ASR_INTEL_FANS, ASR_INTEL_VOLTS, ASR_X2_VOLTS, ASUS_800_VOLTS, ASUS_800_VTT_VOLTS (+38 more)

### Community 25 - "Elevated Actions"
Cohesion: 0.07
Nodes (36): Action, InstallPawnIo, LoadKernelModule, Process, Service, Startup, decode(), ELEVATED_FLAG (+28 more)

### Community 26 - "actor"
Cohesion: 0.17
Nodes (9): actor(), check_contract(), Class, Fast, Slow, Fake, spawn(), Stuck (+1 more)

### Community 27 - "Windows GPU Probe"
Cohesion: 0.09
Nodes (41): Link, AdlInfo, cm_bdf(), CM_Get_DevNode_Registry_PropertyW(), CM_Get_Parent(), cm_u32(), CreateDXGIFactory1(), D3DKMTCloseAdapter() (+33 more)

### Community 28 - "CPU Test Suite"
Cohesion: 0.12
Nodes (35): add(), AES_BLOCKS, dot(), gemm(), gemm_any(), gemm_avx2(), gemm_avx512(), gemm_is_right() (+27 more)

### Community 29 - "Single Instance and SIDs"
Cohesion: 0.08
Nodes (17): WaitForSingleObject(), Claim, claim(), First, Running, H, Instance, key() (+9 more)

### Community 30 - "data.rs"
Cohesion: 0.14
Nodes (21): categories_sizes_and_delete(), Category, .ALL, AppLog, AuditLog, BenchmarkHistory, Diagnostics, Exports (+13 more)

### Community 31 - "mod.rs"
Cohesion: 0.08
Nodes (39): write_fuzz_seeds(), cpu_label(), cpu_temps(), display_name(), fake_sysfs(), put(), read(), readings() (+31 more)

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

### Community 36 - "egui"
Cohesion: 0.06
Nodes (30): ES, label(), labels_and_numbers(), num(), poll(), PT_BR, runs_picker(), save() (+22 more)

### Community 37 - "linux.rs"
Cohesion: 0.06
Nodes (17): Bdf, cfg_path(), I2C_SLAVE_FORCE, I2C_SMBUS, I2C_SMBUS_BYTE, I2C_SMBUS_BYTE_DATA, I2C_SMBUS_READ, I2C_SMBUS_WORD_DATA (+9 more)

### Community 38 - "tray.rs"
Cohesion: 0.07
Nodes (24): Click, App, Mini, Action, Lock, Mini, Pause, Quit (+16 more)

### Community 39 - "drivetest.rs"
Cohesion: 0.12
Nodes (35): ata_kind(), ata_log_parses_newest_first(), ata_logs(), ata_selftest(), ata_status_text(), ctrl(), Entry, is_ata() (+27 more)

### Community 40 - "system_services.rs"
Cohesion: 0.10
Nodes (17): dependency_chain_is_cycle_safe_and_filters_work(), matches_filter(), populate_dependents(), ServiceRow, transitive_dependents(), dispatch(), Finished, a_refused_change_leaves_no_undo_and_resets_the_editor() (+9 more)

### Community 41 - "decode.rs"
Cohesion: 0.15
Nodes (33): bars(), bars_and_ext(), bytes_text(), cap(), cap_name(), cap_node(), caps(), dev() (+25 more)

### Community 42 - "smart.rs"
Cohesion: 0.11
Nodes (29): ascii(), ata(), ata_fields_and_dump(), ata_text(), hex_dump(), kelvin(), le16(), le32() (+21 more)

### Community 43 - "windows.rs"
Cohesion: 0.11
Nodes (31): CloseFn, CreateMutexW(), ExecFn, exits_config(), fn_name(), GetProcAddress(), GlobalMutex, Handle (+23 more)

### Community 44 - "peripherals.rs"
Cohesion: 0.08
Nodes (24): Data, State, Audio, Bars, bars(), bars_of(), batteries(), BATTERY (+16 more)

### Community 45 - "main.rs"
Cohesion: 0.13
Nodes (12): heavy_test(), main(), Out, out(), Html, Json, JsonTxt, Txt (+4 more)

### Community 46 - "os.rs"
Cohesion: 0.09
Nodes (30): cpu_groups(), FreeConsole(), GetActiveProcessorCount(), GetActiveProcessorGroupCount(), GetConsoleProcessList(), GetCurrentThread(), GetThreadPriority(), GroupAffinity (+22 more)

### Community 47 - "diff.rs"
Cohesion: 0.13
Nodes (28): Change, change(), ChangeKind, Added, Modified, Removed, cli(), diff() (+20 more)

### Community 48 - "tools.rs"
Cohesion: 0.12
Nodes (18): changes(), CHART_BOTTOM, CHART_HEIGHT, CHART_LEFT, CHART_LINE, Compare, diff_as_text(), diff_text() (+10 more)

### Community 49 - "frames.rs"
Cohesion: 0.11
Nodes (29): curve_histogram_and_window(), exclusive_fullscreen(), fps(), Frame, frame_csv_line(), FRAMES_CSV_HEADER, histogram(), is_stutter() (+21 more)

### Community 50 - "hotkeys_portal.rs"
Cohesion: 0.11
Nodes (19): bind(), create_session(), DEST, IFACE, listen(), no_bus_means_unavailable(), PATH, Portal (+11 more)

### Community 51 - "overview.rs"
Cohesion: 0.12
Nodes (27): drives(), customize_presets_and_restore(), editor(), ES, find(), health(), identity(), join() (+19 more)

### Community 52 - "mod.rs (2)"
Cohesion: 0.13
Nodes (25): collect(), decode_dimm(), Dimm, dimm_temps(), DUMP, dumps(), FakeHub, golden_ddr5_matches_cpuz() (+17 more)

### Community 53 - "proc_ops.rs"
Cohesion: 0.12
Nodes (25): acts_on_a_child_process(), check(), class(), connections(), dump(), eco(), end(), err() (+17 more)

### Community 54 - "telemetry.rs"
Cohesion: 0.09
Nodes (9): cli_log(), every_source_keeps_the_contract(), Gpus, lock(), catch_ctrl_c(), ctrl_c(), sources(), status() (+1 more)

### Community 55 - "procs.rs"
Cohesion: 0.09
Nodes (28): anti_cheat(), classify(), cpu_percent(), grouping_and_critical(), HIGH_CPU, HISTORY, Kind, App (+20 more)

### Community 56 - "display_drm.rs"
Cohesion: 0.11
Nodes (20): card_modes(), CardRes, CONNECTED, connector_name(), connector_names_match_the_kernels(), Crtc, driver_of(), FLAG_DBLSCAN (+12 more)

### Community 57 - "settings.rs"
Cohesion: 0.08
Nodes (27): layout_editor_edits(), a_row_pins_under_its_card(), alerts(), ES, lang_of(), page(), privacy(), PT_BR (+19 more)

### Community 58 - "gpubench.rs"
Cohesion: 0.11
Nodes (16): adapters(), BIG, block_on(), flops(), FLOPS_PER_ITER, FMA, Gpu, GROUPS (+8 more)

### Community 59 - "layout_view.rs"
Cohesion: 0.08
Nodes (18): aliases_resolve_to_this_machines_sensors(), bar(), ES, lbl(), preview(), PT_BR, resolve(), s() (+10 more)

### Community 60 - "fuzz.yml"
Cohesion: 0.06
Nodes (32): MINUTES, name, runs-on, steps, strategy, timeout-minutes, minutes, jobs (+24 more)

### Community 61 - "intel.rs"
Cohesion: 0.07
Nodes (45): ADL, ARL, arrow_lake(), BCLK_FREQ, decode(), degrade(), F, f() (+37 more)

### Community 62 - "f"
Cohesion: 0.12
Nodes (21): find_agesa(), rom_agesa(), section(), uefi(), collect(), f(), collect(), domains() (+13 more)

### Community 63 - "metrics.rs"
Cohesion: 0.07
Nodes (22): Buf, Csv, csv_header_rows_and_new_columns(), History, Series, Snapshot, Stat, Tick (+14 more)

### Community 64 - "proc_net.rs"
Cohesion: 0.13
Nodes (20): begin(), inet_diag_message(), KERNEL_NETWORK, KEYWORDS, LAST, nettop_csv(), on_event(), parse_inet_diag() (+12 more)

### Community 65 - "theme.rs"
Cohesion: 0.10
Nodes (19): Config, parse(), apply(), DARK, fonts(), GAP, LIGHT, metric() (+11 more)

### Community 66 - "tray.rs (2)"
Cohesion: 0.10
Nodes (16): argb(), build(), icon(), Imp, items(), menu(), menu_follows_its_state(), MenuState (+8 more)

### Community 67 - "bottleneck.rs"
Cohesion: 0.24
Nodes (25): a_real_presentmon_capture_tells_cpu_bound_loading_from_gpu_bound_play(), a_session_sums_up_time_per_class(), analyze(), base(), class(), classify(), cpu_bound_needs_a_saturated_thread_and_gpu_headroom(), estimates_are_counted_and_shown() (+17 more)

### Community 68 - "mod.rs (3)"
Cohesion: 0.15
Nodes (22): Adapter, adapter_nodes(), adapters(), apply_nvml(), backend(), collect(), fill_from_db(), gen_speed() (+14 more)

### Community 69 - "live_sessions.rs"
Cohesion: 0.15
Nodes (20): Session, System, a_mangohud_log_imports_as_a_session(), bars(), COLORS, compare(), sessions_dir(), ES (+12 more)

### Community 70 - "procs.rs (2)"
Cohesion: 0.18
Nodes (5): Collector, collector_builds_rows_and_rates(), Meta, Prev, worker()

### Community 71 - "bottleneck_ui.rs"
Cohesion: 0.10
Nodes (23): Analysis, Confidence, High, Low, Medium, Evidence, fill(), Recommendation (+15 more)

### Community 72 - "mod.rs (4)"
Cohesion: 0.15
Nodes (23): amd_pstates(), apic_tree(), arm_free_sysctl(), build(), bus_mhz(), Cache, cache_line(), Cluster (+15 more)

### Community 73 - "trayicon.rs"
Cohesion: 0.11
Nodes (23): Threshold, Tone, Danger, Good, Normal, Warning, detail_levels_filter_cards(), placeholder() (+15 more)

### Community 74 - "windows.rs (2)"
Cohesion: 0.12
Nodes (24): read(), hard_faults_per_sec(), open(), Array, reads_a_wildcard_counter(), array(), collect(), cpu_load() (+16 more)

### Community 75 - "telemetry.rs (2)"
Cohesion: 0.13
Nodes (25): throttle_kind(), AT_LIMIT, c(), CLOCK_DROP, Cond, cond(), Debounce, DEBOUNCE (+17 more)

### Community 76 - "chipset.rs"
Cohesion: 0.15
Nodes (25): amd_chipset(), amd_umc(), chipset_names(), chipset_token(), ChipsetSummary, decode_umc(), decode_umc_more(), dimms() (+17 more)

### Community 77 - "telemetry.rs (3)"
Cohesion: 0.07
Nodes (23): r(), Reading, battery(), Board, cli_once(), cpu_extra(), Dimms, Disks (+15 more)

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
Cohesion: 0.13
Nodes (23): audit_lines_are_json(), AuditEntry, audit(), audit_log(), audit_path(), bundled_pawnio(), Done, Cancelled (+15 more)

### Community 83 - "amd_pm.rs"
Cohesion: 0.16
Nodes (27): APU, cells(), close(), Core, Cores, cores(), decode(), enabled_slots() (+19 more)

### Community 84 - "mod.rs (5)"
Cohesion: 0.08
Nodes (8): Caps, detect(), driver_installed(), driver_modules_found(), Hw, NoHw, none(), PAUSED

### Community 85 - "users.rs"
Cohesion: 0.06
Nodes (16): query_wts_str(), send_message(), sessions(), state_label(), to_wide(), UserSession, aggregate_users(), ES (+8 more)

### Community 86 - "macos.rs"
Cohesion: 0.15
Nodes (24): battery(), call(), conn(), decode(), IOConnectCallStructMethod(), IOObjectRelease(), IOServiceGetMatchingService(), IOServiceMatching() (+16 more)

### Community 87 - "large_pages.rs"
Cohesion: 0.08
Nodes (4): alloc(), HUGE, lock_memory_privilege(), Region

### Community 88 - "Stage"
Cohesion: 0.15
Nodes (17): command(), fetch_and_verify(), in_path(), Installer, launch(), open_path(), sha256_file(), Stage (+9 more)

### Community 89 - "robustness.rs"
Cohesion: 0.12
Nodes (15): edid_blocks(), FIXTURE, hardware_and_mangohud_logs(), layout_files(), n(), output_lines_ignore_chunking(), pci_config_space(), presentmon_csv_files() (+7 more)

### Community 90 - "report.rs"
Cohesion: 0.14
Nodes (25): esc(), fields(), find(), find_section(), groups(), gv(), hex(), hex_rows() (+17 more)

### Community 91 - "overlay.rs"
Cohesion: 0.09
Nodes (4): apply(), each(), Find, Style

### Community 92 - "pci/windows.rs"
Cohesion: 0.06
Nodes (50): description, label, placeholder, render, value, body, attributes, id (+42 more)

### Community 93 - "soak.py"
Cohesion: 0.23
Nodes (9): _linux_sampler(), _macos_sampler(), make_sampler(), profile(), report(), run_gui(), run_sampler(), _windows_sampler() (+1 more)

### Community 94 - "nvml.rs"
Cohesion: 0.17
Nodes (13): db_apple(), cstr(), Dev, FanRpm, get(), Info, load(), Memory (+5 more)

### Community 95 - "live.rs"
Cohesion: 0.10
Nodes (11): HotkeyAction, .ALL, Capture, CycleStyle, EditMode, Marker, ToggleOverlay, Hotkeys (+3 more)

### Community 96 - "services.rs"
Cohesion: 0.11
Nodes (24): small_text_parsers(), a_dacl_that_refuses_administrators(), CHANGE_CONFIG, is_protected(), is_third_party(), parse_systemd_secs(), parse_systemd_show(), protected_and_third_party_classification() (+16 more)

### Community 97 - "windows.rs (4)"
Cohesion: 0.14
Nodes (7): live_dump(), Module, PawnIo, pch_decode(), pch_raw(), SioSlot, on_cpu()

### Community 98 - "devices.rs"
Cohesion: 0.17
Nodes (20): adapter(), battery(), device_card(), drive(), ES, gb(), Health, health() (+12 more)

### Community 99 - "pins.rs"
Cohesion: 0.16
Nodes (22): cache_id(), card_id(), card_text(), current_card(), enter_card(), ES, is_pinned(), latest() (+14 more)

### Community 100 - "screens.rs"
Cohesion: 0.16
Nodes (21): nearest_anchor(), A, B, C, choose(), choosing_where_to_draw(), locate(), locate_and_place_round_trip() (+13 more)

### Community 101 - "startup.rs"
Cohesion: 0.08
Nodes (9): StartupOp, SetEnabled, apply(), enum_reg_strings(), query(), read_approved_map(), read_fw_post_time(), RegKey (+1 more)

### Community 102 - "diskbench.rs"
Cohesion: 0.14
Nodes (16): write_at(), a_tiny_run_measures_and_cleans_up(), a_tiny_sustained_write_cleans_up(), bench(), phase(), rates(), rates_math(), Run (+8 more)

### Community 103 - "update.rs"
Cohesion: 0.20
Nodes (7): Channel, .ALL, Alpha, Beta, Snapshot, Stable, cli()

### Community 104 - "session.rs"
Cohesion: 0.19
Nodes (18): windows_from_session(), corrupt_sessions_stay_bounded(), long_sessions_are_linear(), Band, ChartLine, comparability(), comparisons_and_repeats(), esc() (+10 more)

### Community 105 - "zeez-live.md"
Cohesion: 0.11
Nodes (21): Now card, live readings, session analysis, glow renderer with automatic wgpu fallback (ADR 0013), MangoHud runtime component (detect + pkexec install), PresentMon sidecar bundled, Decisions Q1-Q10, Anti-cheat and privacy rule (no strong handles), System hub navigation (Q5): Details as view mode, PawnIO x kernel anti-cheat compatibility list (SP7) (+13 more)

### Community 106 - "ci.yml"
Cohesion: 0.18
Nodes (23): steps, steps, name, runs-on, steps, headless, visual, steps (+15 more)

### Community 107 - "battery.rs"
Cohesion: 0.15
Nodes (20): batteries(), Battery, chemistry_name(), collect(), GUID_DEVICE_BATTERY, info(), IOCTL_BATTERY_QUERY_INFORMATION, IOCTL_BATTERY_QUERY_STATUS (+12 more)

### Community 108 - "usb.rs"
Cohesion: 0.13
Nodes (21): CM_Get_Parent(), controller(), parent(), reg_prop(), reg_str(), reg_strs(), reg_u32(), class_from_compat() (+13 more)

### Community 109 - "startup.rs (2)"
Cohesion: 0.11
Nodes (20): extract_exe_path(), StartupRow, StartupSource, CommonFolder, HkcuRun, HkcuRunOnce, HklmRun, HklmRun32 (+12 more)

### Community 110 - "a11y.rs"
Cohesion: 0.10
Nodes (25): at_every_zoom(), beyond(), bottleneck_sample(), chart_colours_meet_the_graphics_minimum(), class_colours_are_distinct_and_readable(), collisions(), contrast(), every_live_tab_holds_together_at_every_scale() (+17 more)

### Community 111 - "nightly.yml"
Cohesion: 0.09
Nodes (21): minutes, jobs, soak, default, description, name, on, schedule (+13 more)

### Community 112 - "Ctx"
Cohesion: 0.09
Nodes (23): cluster_cpus(), cppc(), intel_limits(), Mode, Cpu, Memory, State, join() (+15 more)

### Community 113 - "npu.rs"
Cohesion: 0.13
Nodes (17): release(), CORE_COMPUTE, Count, CreateList, devices(), GENERIC_ML, GetAdapter, GetProp (+9 more)

### Community 114 - "Step"
Cohesion: 0.09
Nodes (22): BenchView, Cpu, Gpu, History, Latency, Memory, Storage, Stress (+14 more)

### Community 115 - "probe"
Cohesion: 0.12
Nodes (25): imported_reports(), n(), settings_from_any_version(), update_answers(), ascii(), ata(), ata_cmd(), ata_cmd_lba() (+17 more)

### Community 116 - "availability.rs"
Cohesion: 0.06
Nodes (47): Capability, .ALL, BoardSensors, ChipsetTemp, CpuSensors, MemoryTimings, Spd, caps() (+39 more)

### Community 117 - "live.rs (2)"
Cohesion: 0.19
Nodes (13): EDGE, exe_name(), hotkey_conflicts(), hotkeys_normalize_and_conflict(), normalize_keys(), Overlay, portal_request_path(), portal_trigger() (+5 more)

### Community 118 - "PresentMon bundled sidecar (ETW)"
Cohesion: 0.14
Nodes (13): ADR 0001: ZeeZ Live is the brand, ZeeZ Live (Overlay, Capture, Reports), ADR 0003: FPS on Windows from PresentMon, PresentMon bundled sidecar (ETW), ADR 0004: FPS on Linux from MangoHud, MangoHud integration (detect, install, config, logs), SetLayeredWindowAttributes click-through fix, global-hotkey crate (RegisterHotKey/Carbon/XGrabKey) (+5 more)

### Community 119 - "processor_more.rs"
Cohesion: 0.21
Nodes (7): CoreRow, ES, group_of(), PT_BR, Cell, Num, Text

### Community 120 - "pci/windows.rs"
Cohesion: 0.13
Nodes (24): acpi_table(), CM_Get_Parent(), DevInfoData, DevPropKey, DIGCF_ALLCLASSES, DIGCF_PRESENT, Guid, hwid_field() (+16 more)

### Community 121 - "cstates.rs"
Cohesion: 0.16
Nodes (7): array(), LAST, PREV, Query, QUERY, Raw, Residency

### Community 122 - "memory.rs"
Cohesion: 0.14
Nodes (8): composition(), Handles, LAST, lists(), MB, PATHS, Query, QUERY

### Community 123 - "memtest.rs"
Cohesion: 0.17
Nodes (9): every_pattern_catches_a_flipped_bit(), fill(), MemTest, PATTERNS, runs_and_stops(), seed(), value(), verify() (+1 more)

### Community 124 - "CHANGELOG.md"
Cohesion: 0.09
Nodes (45): ADR 0009: releases stay unsigned for now, ADR 0013: glow with wgpu fallback, 1.1.0 roadmap (docs/v1.1), Security review 2026-11-09, 1.0 plan (docs/v1), ADR 0009: Releases stay unsigned, SHA256SUMS verification for unsigned releases, ADR 0011: Workspace split by layer (+37 more)

### Community 125 - "live_frames.rs"
Cohesion: 0.12
Nodes (24): App, apps(), begin_capture(), CAPTURE_MAX, Demo, due(), end_capture(), Hub (+16 more)

### Community 126 - "detail.rs"
Cohesion: 0.25
Nodes (9): BASIC, Level, Advanced, .ALL, Basic, Expert, level_of(), levels() (+1 more)

### Community 127 - "clocks.rs"
Cohesion: 0.16
Nodes (17): active_from_deltas(), active_mhz(), add_chain_mhz(), adds(), aperf_mperf(), aperf_mperf_regs(), chain(), core_mhz() (+9 more)

### Community 128 - "table.rs"
Cohesion: 0.23
Nodes (18): cols(), columns_move_and_keep_their_order(), ES, filter_sort_and_group(), header_resizes_moves_and_sorts_with_the_mouse(), MIN_W, move_column(), P (+10 more)

### Community 129 - "superio.rs"
Cohesion: 0.28
Nodes (12): apply(), fintek(), Io, ite(), nct6687_like_the_dump(), nct_ec(), nct_new(), nct_old() (+4 more)

### Community 130 - "layout_view.rs"
Cohesion: 0.07
Nodes (6): Source, Batteries, Cpu, Drives, GpuEngines, MeasuredClock

### Community 131 - "query.rs"
Cohesion: 0.26
Nodes (15): half_typed_and_odd_input_never_fails(), hit(), matches(), number(), parse(), plain(), row(), syntax() (+7 more)

### Community 132 - "cpu_os.rs"
Cohesion: 0.14
Nodes (11): CODE_INTEGRITY, KVA_SHADOW, NtQuerySystemInformation(), Power, power(), query(), read(), SPECULATION_CONTROL (+3 more)

### Community 133 - "linux.rs (2)"
Cohesion: 0.14
Nodes (19): raw(), read_trim(), read_u64(), adapter(), cards(), dpm(), freq(), hwmon_live() (+11 more)

### Community 134 - "history.rs"
Cohesion: 0.27
Nodes (15): add(), dates(), Entry, entry(), load(), load_from(), now_rfc3339(), path() (+7 more)

### Community 135 - "health.rs"
Cohesion: 0.26
Nodes (16): a_healthy_build_has_nothing_to_say(), check(), Display, each_mistake(), Facts, Finding, finding(), good() (+8 more)

### Community 136 - "live.rs (3)"
Cohesion: 0.16
Nodes (14): Anchor, .ALL, Bottom, BottomLeft, BottomRight, Center, Left, Right (+6 more)

### Community 137 - "dependencies.md"
Cohesion: 0.09
Nodes (19): ADR 0006: Dependencies, rusqlite (bundled SQLite), windows-sys, ADR 0013: glow renderer with wgpu fallback, glow (OpenGL) renderer, wgpu fallback (CZ_RENDERER=wgpu, WARP), Benchmark principles (safe, reproducible, 1000 = reference 7900X), Benchmarks (CPU suite, core-to-core latency, memory, GPU, storage, stability) (+11 more)

### Community 138 - "table.rs (2)"
Cohesion: 0.18
Nodes (8): R, cell_text(), Column, Column<'a, R>, fit(), fit_width(), Header, Table

### Community 139 - "table.rs (3)"
Cohesion: 0.33
Nodes (7): Item, Group, Row, shown(), State, Table<'a, R>, to_tsv()

### Community 140 - "win.rs"
Cohesion: 0.15
Nodes (17): Adl, adl(), adl_alloc(), apply_nvapi(), live(), nv_gpu(), NvApi, nvapi() (+9 more)

### Community 141 - "Page"
Cohesion: 0.12
Nodes (16): NAV, NAV_BOTTOM, Page, About, Benchmarks, Devices, Graphics, Live (+8 more)

### Community 142 - "Window"
Cohesion: 0.30
Nodes (19): analyze_verdicts(), avg(), cpu_saturated(), Finding, gpu_headroom(), gpu_saturated(), n0(), pct() (+11 more)

### Community 143 - "FEATURES.md (CPU-Z 2.15 clean-room spec)"
Cohesion: 0.05
Nodes (42): ADR 0012: no code injection, ever, ADR 0014: no hardware balance bar in 1.0, ADR 0015: Wayland shortcuts through the GlobalShortcuts portal, Anti-cheat safe mode, App history (SQLite store), Bottleneck detector (What limits my FPS?), Build health checks, Per-action elevation (one-shot elevated copy, closed operation list) (+34 more)

### Community 144 - "actions.rs"
Cohesion: 0.08
Nodes (23): ServiceOp, Pause, Restart, SetAccount, SetRecovery, SetStartup, Start, StartWithParams (+15 more)

### Community 145 - "CPU ZeeZ 1.1.0 plan"
Cohesion: 0.06
Nodes (39): AccessKit names, states and values for painted widgets, Accessibility (S3-03), Keyboard navigation and table shortcuts, Scale test 75-200% (at_every_zoom), Screen readers cannot read painted tables and charts, WCAG AA contrast check from theme palette, Read-only helper service (after 1.0), ADR 0007: One 1.0.0 release in gated stages (+31 more)

### Community 146 - "CpuZeeZ 1.0 Plan (hub)"
Cohesion: 0.04
Nodes (51): ADR 0008: Planning documents are in English, Command palette (Ctrl+K), Shared page patterns (detail levels, Ctrl+K palette, banners, pin, units), rusqlite (bundled SQLite), windows-sys crate, ADRs in docs/adr, Contract test suites and fakes, core crate (pure logic, no OS/GUI) (+43 more)

### Community 147 - "pkgver.py"
Cohesion: 0.18
Nodes (3): Strict, _test(), versions()

### Community 148 - "mod.rs (7)"
Cohesion: 0.24
Nodes (7): collect(), devices(), ecam_base(), enumerate(), ids(), ids_name(), register_spaces()

### Community 149 - "applog.rs"
Cohesion: 0.17
Nodes (11): diagnostics_bundle(), info(), install_panic_hook(), LOCK, MAX, path(), warn(), write() (+3 more)

### Community 150 - "live_mangohud.rs"
Cohesion: 0.18
Nodes (15): set_running(), wanted(), config_home(), config_path(), configure(), env_path(), home(), Install (+7 more)

### Community 151 - "raster.py"
Cohesion: 0.20
Nodes (8): dist_seg(), in_poly(), in_rrect(), on_arc(), png(), render(), render_png(), sample()

### Community 152 - "i18n.rs"
Cohesion: 0.18
Nodes (17): bottleneck_texts_are_translated(), current(), CURRENT, every_literal_is_translated(), from_code(), Lang, En, Es (+9 more)

### Community 153 - "Period"
Cohesion: 0.12
Nodes (11): Cmd, Clear, Flush, Query, Period, All, Days30, Days7 (+3 more)

### Community 154 - "direct_io.rs"
Cohesion: 0.18
Nodes (7): ALIGN, Buffer, FILE_FLAG_NO_BUFFERING, FILE_FLAG_WRITE_THROUGH, open(), read_at(), uncached_round_trip()

### Community 156 - "tempdir.rs"
Cohesion: 0.32
Nodes (14): base_is_safe(), base_validation_follows_symlinks_in_the_path(), base_validation_rejects_swappable_ancestors(), base_validation_rejects_swappable_symlinks(), base_validation_requires_sticky_when_shared(), create(), create_exclusive_private(), private_and_ours() (+6 more)

### Community 157 - "firmware.rs"
Cohesion: 0.19
Nodes (7): Header, header(), text(), acpi(), Firmware, read(), SECURE_BOOT

### Community 158 - "pci/linux.rs"
Cohesion: 0.20
Nodes (10): Ctl, every_test_runs(), p_vs_e(), pinned(), rate(), reference(), scaling(), st_cpu() (+2 more)

### Community 159 - "dependabot.yml"
Cohesion: 0.13
Nodes (14): patterns, prefix, patterns, actions, egui, interval, updates, commit-message (+6 more)

### Community 160 - "ci.yml (2)"
Cohesion: 0.14
Nodes (15): container, name, runs-on, strategy, name, os, distros, native (+7 more)

### Community 161 - "superio.rs (2)"
Cohesion: 0.26
Nodes (10): chips(), cfg_rd(), cfg_rd16(), cfg_wr(), Chip, detect(), Fake, finish() (+2 more)

### Community 162 - "bottleneck.rs (3)"
Cohesion: 0.14
Nodes (12): Class, .ALL, Background, Cpu, Disk, Gpu, Inconclusive, Limiter (+4 more)

### Community 163 - "engines.rs"
Cohesion: 0.31
Nodes (11): AdapterEngines, aggregate(), engine_label(), labels(), parse(), per_process(), per_process_is_the_busiest_engine(), per_process_memory() (+3 more)

### Community 164 - "windows.rs (5)"
Cohesion: 0.24
Nodes (8): open_never_panics(), PawnSmbus, Xfer, xfer_args(), ReadByte, ReadWord, SendByte, WriteByte

### Community 165 - "ServiceFilter"
Cohesion: 0.25
Nodes (7): ServiceFilter, All, Disabled, FailedRecently, Running, StoppedAuto, ThirdParty

### Community 166 - "ci.yml (3)"
Cohesion: 0.14
Nodes (14): name, runs-on, name, runs-on, steps, jobs, deb, deny (+6 more)

### Community 167 - "gpu/macos.rs"
Cohesion: 0.25
Nodes (7): adapter(), apple_nm(), hex_id(), ioreg_edids(), metal(), probe(), s()

### Community 168 - "memplus.rs"
Cohesion: 0.33
Nodes (13): chain(), chain_is_one_cycle(), gups(), latency(), LINE, pin(), Plan, run() (+5 more)

### Community 169 - "timers.rs"
Cohesion: 0.26
Nodes (13): ACPI_PM_HZ, acpi_timer(), arm_cntfrq(), fadt(), fadt_pm_timer(), hz_text(), measure_pm(), perf_timer() (+5 more)

### Community 170 - "layout_editor.rs"
Cohesion: 0.20
Nodes (6): editor(), ES, KINDS, PT_BR, THROTTLE_DEVICES, ALIASES

### Community 171 - "units.rs"
Cohesion: 0.12
Nodes (11): Clock, Auto, Ghz, Mhz, Size, Decimal, Iec, Windows (+3 more)

### Community 172 - "net_io.rs"
Cohesion: 0.17
Nodes (4): FILTER, HARDWARE, read(), RTM_IFINFO2

### Community 173 - "Bottleneck Detector Design"
Cohesion: 0.12
Nodes (19): Bottleneck detector (measured evidence only), Build health checks on Overview, ADR 0014: No hardware balance bar in 1.0, Configuration checks / Build health (single channel, XMP off, PCIe, ReBAR...), Bottleneck Detector Design, Evidence-tied recommendations (no snake oil), Heuristic fallback without GPU Busy (lower confidence), Limiters (V-Sync, VRR, FPS cap, compositor) (+11 more)

### Community 174 - "start"
Cohesion: 0.24
Nodes (11): end_session(), final_path_of(), held(), hold(), run(), set_status(), sidecar(), start() (+3 more)

### Community 175 - "cpusuite.rs"
Cohesion: 0.26
Nodes (8): Aes, aes_arm(), aes_hw(), aes_matches_fips_197(), aes_ni(), aes_sw(), counters(), sbox()

### Community 176 - "arm.rs"
Cohesion: 0.36
Nodes (8): build(), cpuinfo_field(), cpuinfo_midr(), implementer(), midr_text(), part(), sysctl_str(), sysctl_u64()

### Community 177 - "fixture.rs"
Cohesion: 0.29
Nodes (10): adapters(), app(), cpu(), ctx(), drives(), every_hardware_page_draws_from_the_fixtures(), every_overlay_style_stays_within_its_drawing_budget(), harness() (+2 more)

### Community 178 - "superio.rs (3)"
Cohesion: 0.17
Nodes (13): b(), Board, board_lookup(), board_map(), BOARDS, EC_GENERIC, ITE_GENERIC, ITE_OLD_GENERIC (+5 more)

### Community 179 - "os.rs (2)"
Cohesion: 0.23
Nodes (10): cpu_load(), host_processor_info(), mach_host_self(), mach_task_self_, parse_proc_stat(), PROCESSOR_CPU_LOAD_INFO, STOP, throttle_counts() (+2 more)

### Community 180 - "latency.rs"
Cohesion: 0.31
Nodes (9): measure(), pair(), ROUNDS, statistical(), statistical_mode_measures_unpinned(), summary(), summary_reads_the_upper_triangle(), two_cores_answer() (+1 more)

### Community 181 - "diskio.rs"
Cohesion: 0.30
Nodes (11): a_reset_counter_is_no_activity_not_garbage(), Counters, DISKSTATS, diskstats_keeps_whole_disks_only(), diskstats_units(), is_whole_disk(), parse_diskstats(), Rates (+3 more)

### Community 182 - ".show_ui"
Cohesion: 0.31
Nodes (4): app_history_tab_shows_the_view(), format_duration_hms(), render_daily_chart(), State

### Community 183 - "units.rs (2)"
Cohesion: 0.29
Nodes (4): Units, .DEFAULT, CURRENT, set()

### Community 184 - "storage-and-cleanup.md"
Cohesion: 0.10
Nodes (19): ADR 0002: Privileges, Restart as administrator button, ADR 0005: Task Manager under one System entry, System hub (Processes, Performance, App history, Startup, Users, Services), BOARDS Super I/O map (src/sensors/superio.rs), Ideas for later versions, Storage audit and safe cleanup (idea), Duplicate file finder (size, partial hash, SHA-256) (+11 more)

### Community 185 - "WindowOp"
Cohesion: 0.33
Nodes (6): window_op(), WindowOp, Close, Maximize, Minimize, SwitchTo

### Community 186 - "ROADMAP.md"
Cohesion: 0.18
Nodes (15): Anti-cheat match test (spike X08), 1.0.0-beta.1 manual test checklist, macOS validation checklist (S0), CHANGELOG 1.0.0 Consolidated (pointer), 1.0 release criteria, CpuZeeZ 1.0 Detailed Roadmap, Sampler measurements on Ryzen 9 7900X, S0 Foundation (+7 more)

### Community 187 - "make_icons.py"
Cohesion: 0.29
Nodes (4): icns(), ico(), main(), render()

### Community 188 - "superio.rs (4)"
Cohesion: 0.17
Nodes (12): Family, Fintek, Ite, NctEc, NctNew, NctOld, MV10_6, MV11 (+4 more)

### Community 189 - "Graphify neste projeto"
Cohesion: 0.33
Nodes (5): Graphify neste projeto, `.graphifyignore`, Notas, Setup (uma vez por projeto), Uso no dia a dia

### Community 190 - "best_hz"
Cohesion: 0.36
Nodes (8): best_hz(), fixture(), mode(), ModeInfo, parse_modeinfo(), refresh_from_the_timing(), short_or_garbled_input_never_panics(), the_best_mode_is_at_the_current_resolution()

### Community 191 - "actions.rs (2)"
Cohesion: 0.18
Nodes (8): Priority, AboveNormal, .ALL, BelowNormal, High, Idle, Normal, Realtime

### Community 193 - "edid.rs"
Cohesion: 0.31
Nodes (10): decode(), displayid(), displayid_ext(), dtd(), Monitor, parse(), PNP, pnp_name() (+2 more)

### Community 194 - "layout.rs (2)"
Cohesion: 0.18
Nodes (10): Style, .ALL, Bar, Graph, Number, Ring, Theme, Dark (+2 more)

### Community 195 - "mangohud.rs"
Cohesion: 0.27
Nodes (9): config(), CONFIG_FILE, config_follows_the_layout(), ENV_FILE, env_file(), FLATPAK_ID, install_command(), LAUNCH_OPTION (+1 more)

### Community 196 - "smbios.rs (2)"
Cohesion: 0.23
Nodes (12): apple(), entry_point_version(), ioreg_hex(), synth(), truncated_and_uuid(), uuid_bytes(), uuid_text(), GetSystemFirmwareTable() (+4 more)

### Community 197 - "encryption.rs"
Cohesion: 0.27
Nodes (6): crypt(), DONE, read(), STARTED, Status, status()

### Community 198 - "arc"
Cohesion: 0.25
Nodes (3): game_centre(), label(), points()

### Community 199 - ".stage_shot"
Cohesion: 0.29
Nodes (3): App, write_png(), demo()

### Community 200 - "analyze"
Cohesion: 0.32
Nodes (4): analyze(), frame_trend(), selftest(), slope()

### Community 203 - "soak.rs"
Cohesion: 0.17
Nodes (9): App, enabled(), frame(), NAME, overlay_frame(), overlay_only(), PID, start() (+1 more)

### Community 204 - "mainboard_more.rs"
Cohesion: 0.29
Nodes (4): ES, link_status(), link_text(), PT_BR

### Community 205 - "smart"
Cohesion: 0.33
Nodes (5): nvme_log(), sat_smart(), sat_smart_cmd(), smart(), smartctl()

### Community 206 - "db.rs"
Cohesion: 0.27
Nodes (7): AMD, INTEL, lookup(), NV, s(), Spec, TABLE

### Community 207 - "mod.rs (8)"
Cohesion: 0.38
Nodes (8): class_desc(), Ids, ids_parse_and_labels(), mcfg(), mcfg_base(), parse_ids(), vendor_label(), vendor_short()

### Community 208 - "bench_views_test.rs"
Cohesion: 0.57
Nodes (5): cpu_suite_modes(), ctx(), memory_and_latency_views(), storage_view_warns_before_writing(), themed()

### Community 209 - "startup.rs (4)"
Cohesion: 0.17
Nodes (14): attr_value(), decode_startup_approved(), encode_startup_approved(), filetime_to_ymd(), Impact, High, Low, Medium (+6 more)

### Community 210 - "Status"
Cohesion: 0.29
Nodes (7): Status, BadChecksum, Missing, NeedsAdmin, Off, Running, Starting

### Community 211 - "Status"
Cohesion: 0.33
Nodes (5): STATE, Status, NeedsAdmin, Running, Unavailable

### Community 213 - "shotdiff.py"
Cohesion: 0.29
Nodes (4): compare(), diff(), read_png(), _test()

### Community 214 - ".tick"
Cohesion: 0.33
Nodes (3): display_hz(), frame_age(), recent_ms()

### Community 215 - "layout.rs (3)"
Cohesion: 0.22
Nodes (9): Kind, Bar, Clock, Header, LabelValue, Separator, Sparkline, Throttle (+1 more)

### Community 216 - "memory.rs (2)"
Cohesion: 0.28
Nodes (5): Composition, meminfo(), MEMINFO, parse_meminfo(), Tour

### Community 219 - "ci.yml (4)"
Cohesion: 0.22
Nodes (8): env, CARGO_TERM_COLOR, name, on, pull_request, push, workflow_dispatch, branches

### Community 222 - "alerts.rs"
Cohesion: 0.43
Nodes (6): defaults(), fires_only_when_the_whole_window_is_over(), firing(), pts(), Rule, rules_load_from_older_settings()

### Community 223 - "limits.rs"
Cohesion: 0.43
Nodes (7): amd_cppc_highest(), cppc_ranks(), intel_hwp_highest(), intel_limits_decode(), intel_pkg_limits(), Limit, ranks()

### Community 224 - "netio.rs"
Cohesion: 0.43
Nodes (7): Counters, NET_DEV, parse_proc_net_dev(), proc_net_dev(), Rates, rates(), rates_in_megabits()

### Community 226 - "services.rs (5)"
Cohesion: 0.25
Nodes (7): ServiceState, Paused, Running, StartPending, Stopped, StopPending, Unknown

### Community 228 - "ci.yml (5)"
Cohesion: 0.25
Nodes (8): with, components, fetch-depth, if-no-files-found, name, path, tool, toolchain

### Community 230 - "macos.rs (4)"
Cohesion: 0.54
Nodes (6): fill(), memory_type(), memory_without_smbios(), partial_and_unavailable_data(), preserves_smbios_and_handles_nested_banks(), probe()

### Community 231 - "dl.rs"
Cohesion: 0.43
Nodes (4): GetProcAddress(), Lib, LoadLibraryW(), sym!

### Community 234 - "update.rs (2)"
Cohesion: 0.46
Nodes (5): compress(), hex(), Sha256, sha256_fips_vectors(), sha256_of()

### Community 238 - "query.rs (2)"
Cohesion: 0.29
Nodes (7): Op, Contains, Eq, Ge, Gt, Le, Lt

### Community 242 - "ci.yml (6)"
Cohesion: 0.33
Nodes (6): msi-releases, name, permissions, runs-on, permissions, contents

### Community 243 - "SysTab"
Cohesion: 0.29
Nodes (7): SysTab, AppHistory, Performance, Processes, Services, Startup, Users

### Community 244 - "bottleneck.rs (4)"
Cohesion: 0.40
Nodes (5): LimiterKind, Compositor, FpsCap, Menu, VSync

### Community 246 - "layout.rs (4)"
Cohesion: 0.50
Nodes (4): Direction, Grid, Horizontal, Vertical

### Community 247 - "lib.rs"
Cohesion: 0.50
Nodes (3): DATA_DIR_NAME, DISPLAY_NAME, NAME

### Community 250 - "Cargo.toml"
Cohesion: 0.67
Nodes (4): cpuzeez, cpuzeez-core, cpuzeez-fuzz, cpuzeez-platform

## Knowledge Gaps
- **502 isolated node(s):** `CpuSensors`, `BoardSensors`, `Spd`, `MemoryTimings`, `ChipsetTemp` (+497 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 1786 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **34 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `Ctx` connect `Ctx` to `Sensors Page and Stress Monitor`, `Benchmark Suite Views`, `x86.rs`, `palette`, `System Performance Charts`, `Users Tab and Access Banner`, `history.rs`, `layout_view.rs`, `CPU Benchmark Runner`, `Memory Benchmark`, `Storage Drives Probe`, `GUI App Shell`, `Network Adapters`, `Probe Cache and ACPI`, `CPU Sensors and Power Limits`, `mod.rs (7)`, `Processes Tab`, `mod.rs`, `superio.rs (2)`, `egui`, `timers.rs`, `peripherals.rs`, `main.rs`, `diff.rs`, `overview.rs`, `mod.rs (2)`, `telemetry.rs`, `layout_view.rs`, `f`, `mod.rs (3)`, `arc`, `bottleneck_ui.rs`, `mod.rs (4)`, `telemetry.rs (2)`, `chipset.rs`, `telemetry.rs (3)`, `.tick`, `devices.rs`, `battery.rs`, `usb.rs`, `npu.rs`, `probe`, `availability.rs`, `clocks.rs`?**
  _High betweenness centrality (0.091) - this node is a cross-community bridge._
- **Are the 7 inferred relationships involving `Ctx` (e.g. with `cli()` and `.page()`) actually correct?**
  _`Ctx` has 7 INFERRED edges - model-reasoned connections that need verification._
- **What connects `CpuSensors`, `BoardSensors`, `Spd` to the rest of the system?**
  _502 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `Sensors Page and Stress Monitor` be split into smaller, more focused modules?**
  _Cohesion score 0.05859969558599695 - nodes in this community are weakly interconnected._
- **Why does `tr()` connect `ZeeZ Live Overlay and Sessions` to `Sensors Page and Stress Monitor`, `Benchmark Suite Views`, `table.rs`, `palette`, `System Performance Charts`, `Users Tab and Access Banner`, `table.rs (3)`, `system_history.rs`, `GUI App Shell`, `Processes Tab`, `i18n.rs`, `palette.rs`, `egui`, `tray.rs`, `system_services.rs`, `layout_editor.rs`, `tools.rs`, `overview.rs`, `.show_ui`, `settings.rs`, `layout_view.rs`, `intel.rs`, `tray.rs (2)`, `live_sessions.rs`, `bottleneck_ui.rs`, `.stage_shot`, `users.rs`, `live.rs`, `devices.rs`, `pins.rs`?**
  _High betweenness centrality (0.041) - this node is a cross-community bridge._
- **Are the 148 inferred relationships involving `tr()` (e.g. with `.banner()` and `.frame()`) actually correct?**
  _`tr()` has 148 INFERRED edges - model-reasoned connections that need verification._
- **Should `Benchmark Suite Views` be split into smaller, more focused modules?**
  _Cohesion score 0.12436974789915967 - nodes in this community are weakly interconnected._