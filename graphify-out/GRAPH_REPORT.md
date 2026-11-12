# Graph Report - cpuzeez  (2026-11-12)

## Corpus Check
- 286 files · ~407,329 words
- Verdict: corpus is large enough that graph structure adds value.
- Unclassified: 38 file(s) not represented in the graph (top: (none) 22, .csv 5, .toml 4)

## Summary
- 6278 nodes · 14722 edges · 255 communities (229 shown, 26 thin omitted)
- Extraction: 92% EXTRACTED · 8% INFERRED · 0% AMBIGUOUS · INFERRED: 1147 edges (avg confidence: 0.82)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `2bceea38`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- Sensors Page and Stress Monitor
- egui
- x86.rs
- trf
- System Performance Charts
- widgets.rs
- tr
- CPU Benchmark Runner
- SPD Decoding
- Memory Benchmark
- Storage Drives Probe
- SMBIOS Decoding
- system_history.rs
- gui/mod.rs
- Metric Engine
- Network Adapters
- Probe Cache and ACPI
- Release Workflow
- CPU Sensors and Power Limits
- CPU Model Database
- system.rs
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
- icon.rs
- linux.rs
- Action
- drivetest.rs
- system_services.rs
- decode.rs
- smart.rs
- windows.rs
- peripherals.rs
- main.rs
- os.rs
- diff.rs
- details
- frames.rs
- hotkeys_portal.rs
- overview.rs
- mod.rs (2)
- proc_ops.rs
- telemetry.rs
- procs.rs
- display_drm.rs
- welcome.rs
- gpubench.rs
- layout_view.rs
- fuzz.yml
- intel.rs
- software.rs
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
- system_users.rs
- availability.rs
- live.rs (2)
- PresentMon bundled sidecar (ETW)
- Cell
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
- gui/tools.rs
- win.rs
- Page
- .render_table
- FEATURES.md (CPU-Z 2.15 clean-room spec)
- actions.rs
- CPU ZeeZ 1.1.0 plan
- CpuZeeZ 1.0 Plan (hub)
- pkgver.py
- boxed
- applog.rs
- live_mangohud.rs
- raster.py
- Sub
- identify.rs
- direct_io.rs
- bench_tab.rs
- tempdir.rs
- firmware.rs
- StartupType
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
- node
- units.rs
- net_io.rs
- Bottleneck Detector Design
- History
- probe
- arm.rs
- fixture.rs
- superio.rs (3)
- os.rs (2)
- .memory_more
- diskio.rs
- interfaces
- RecoveryAction
- storage-and-cleanup.md
- WindowOp
- ROADMAP.md
- make_icons.py
- superio.rs (4)
- Graphify neste projeto
- .mainboard_more
- actions.rs (2)
- best_hz
- edid.rs
- layout.rs (2)
- Impact
- smbios.rs (2)
- encryption.rs
- I2cDev
- .stage_shot
- analyze
- CLAUDE.md
- benchkit.rs
- soak.rs
- jep106.rs
- contrast
- db.rs
- mod.rs (8)
- .risk
- startup.rs (4)
- .parse
- wddm
- struct
- shotdiff.py
- n
- layout.rs (3)
- memory.rs (2)
- Gen
- mod.rs (9)
- ci.yml (4)
- Clock
- detector.rs
- alerts.rs
- limits.rs
- netio.rs
- Size
- bmp.rs
- ci.yml (5)
- macos.rs (4)
- dl.rs
- update.rs (2)
- query.rs (2)
- tr
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
2. `tr()` - 188 edges
3. `palette()` - 114 edges
4. `trf()` - 86 edges
5. `card()` - 63 edges
6. `Series` - 51 edges
7. `V` - 40 edges
8. `Unit` - 40 edges
9. `get()` - 39 edges
10. `Telemetry` - 38 edges

## Surprising Connections (you probably didn't know these)
- `Release notes template` --semantically_similar_to--> `Unsigned releases and SHA256SUMS verification`  [INFERRED] [semantically similar]
  packaging/RELEASE_NOTES.md → README.md
- `Test fixtures README (S0-T04)` --semantically_similar_to--> `CPU-Z DDR5 TeamGroup SPD dump (test data)`  [INFERRED] [semantically similar]
  crates/core/fixtures/README.md → src/spd/testdata/cpuz_ddr5_teamgroup.txt
- `Monitor` --references--> `Click`  [INFERRED]
  src/gui/sensors.rs → crates/core/src/layout.rs
- `Settings` --references--> `Click`  [INFERRED]
  src/settings.rs → crates/core/src/layout.rs
- `readings()` --references--> `Unit`  [INFERRED]
  src/live_frames.rs → crates/core/src/metrics.rs

## Import Cycles
- 1-file cycle: `src/gui/tools.rs -> src/gui/tools.rs`
- 1-file cycle: `src/gui/system_history.rs -> src/gui/system_history.rs`
- 1-file cycle: `src/battery.rs -> src/battery.rs`
- 1-file cycle: `src/usb.rs -> src/usb.rs`
- 1-file cycle: `src/storage.rs -> src/storage.rs`
- 1-file cycle: `src/npu.rs -> src/npu.rs`
- 1-file cycle: `src/software.rs -> src/software.rs`
- 1-file cycle: `crates/platform/src/elevation.rs -> crates/platform/src/elevation.rs`
- 1-file cycle: `crates/platform/src/large_pages.rs -> crates/platform/src/large_pages.rs`
- 1-file cycle: `src/live_frames.rs -> src/live_frames.rs`
- 1-file cycle: `crates/platform/src/procs.rs -> crates/platform/src/procs.rs`
- 1-file cycle: `src/gpu/dl.rs -> src/gpu/dl.rs`
- 1-file cycle: `crates/core/src/layout.rs -> crates/core/src/layout.rs`
- 1-file cycle: `src/peripherals.rs -> src/peripherals.rs`
- 1-file cycle: `crates/platform/src/display.rs -> crates/platform/src/display.rs`
- 1-file cycle: `src/gui/tray.rs -> src/gui/tray.rs`
- 1-file cycle: `crates/platform/src/desktop.rs -> crates/platform/src/desktop.rs`
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

## Communities (255 total, 26 thin omitted)

### Community 0 - "Sensors Page and Stress Monitor"
Cohesion: 0.04
Nodes (61): cpu_temp(), Runtime, avg_clock(), CHART_MAX_H, CHART_MIN_H, charts(), chipset_temp(), core_clocks() (+53 more)

### Community 1 - "egui"
Cohesion: 0.05
Nodes (46): Conditions, conditions(), Job, conditions(), ES, label(), labels_and_numbers(), num() (+38 more)

### Community 2 - "x86.rs"
Cohesion: 0.21
Nodes (23): amd_legacy_caches(), bit(), bits(), brand(), CacheDesc, caches(), cpuid(), decodes_this_cpu() (+15 more)

### Community 3 - "trf"
Cohesion: 0.10
Nodes (12): State, codes_and_fallback(), trf(), ES, heat_map(), Matrix, PT_BR, State (+4 more)

### Community 4 - "System Performance Charts"
Cohesion: 0.07
Nodes (44): ago_text(), AXIS_FONT, chart(), Line, LINE_WIDTH, MARGIN_BOTTOM, MARGIN_LEFT, nearest() (+36 more)

### Community 5 - "widgets.rs"
Cohesion: 0.08
Nodes (22): BootSummary, failure_text(), ES, PT_BR, startup_tab_renders_bios_time_and_timeline(), State, empty_state(), kv() (+14 more)

### Community 7 - "tr"
Cohesion: 0.06
Nodes (37): class_color(), draw_analysis(), evidence_text(), explain(), meter(), spark(), prefixes(), publish_throttling() (+29 more)

### Community 8 - "CPU Benchmark Runner"
Cohesion: 0.07
Nodes (39): Bench, CHECK_10K, CHUNK, cli(), deterministic(), fmix(), hostname(), LANES (+31 more)

### Community 9 - "SPD Decoding"
Cohesion: 0.18
Nodes (37): ascii(), bandwidth(), bcd(), cl_rows(), Core, crc16(), crc_known_value(), cycle() (+29 more)

### Community 10 - "Memory Benchmark"
Cohesion: 0.08
Nodes (41): alloc(), Bandwidth, bandwidth(), BLOCK_LINES, build_chain(), chain_is_one_cycle_over_all_lines(), chase(), CHASE_STEP (+33 more)

### Community 11 - "Storage Drives Probe"
Cohesion: 0.06
Nodes (61): of(), ascii(), ata(), ata_cmd(), ata_cmd_lba(), ata_identify(), ata_smart(), ata_smart_nodata() (+53 more)

### Community 12 - "SMBIOS Decoding"
Cohesion: 0.09
Nodes (47): smbios_tables(), agesa(), array_ecc(), array_location(), array_use(), Bios, bios_info(), bios_of() (+39 more)

### Community 13 - "system_history.rs"
Cohesion: 0.08
Nodes (35): accumulate(), add(), AggregatedApp, app_history_tab_shows_the_view(), AppDayRecord, apps(), clear(), Cmd (+27 more)

### Community 14 - "gui/mod.rs"
Cohesion: 0.09
Nodes (23): App, app_icon(), AutoShots, cpuid_dump(), decode_png(), ES, frame_log(), heading() (+15 more)

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
Nodes (22): bus_mhz(), get(), amd(), amd_limits(), amd_rapl(), ccd_base(), core_clocks(), core_threads() (+14 more)

### Community 20 - "CPU Model Database"
Cohesion: 0.05
Nodes (49): a(), AM2P, AM3, AM3P, AM4, AM5, ANY, APPLE (+41 more)

### Community 21 - "system.rs"
Cohesion: 0.07
Nodes (33): AffinityModal, ALL_COL_IDS, ConfirmEnd, csv_and_json_export_and_snapshot_diff(), diff_snapshots(), DiffEntry, DiffKind, Added (+25 more)

### Community 22 - "update.rs"
Cohesion: 0.12
Nodes (24): API, Asset, channels_by_label(), checksum_listing(), download_dirs_are_fresh_private_and_unique(), each_channel_gets_its_newest(), FAILED, hostile_versions_stay_in_one_component() (+16 more)

### Community 23 - "Process Snapshot Platform"
Cohesion: 0.08
Nodes (39): parse_major_faults(), the_rpc_endpoint_mapper_is_listening(), boot_ms(), command_line(), decoders(), elf_arch(), filetime_conversion(), filetime_ms() (+31 more)

### Community 24 - "Super I/O Sensor Chips"
Cohesion: 0.08
Nodes (46): ASR_AM4_FANS, ASR_AM4_VOLTS, ASR_AM5_VOLTS, ASR_INTEL_FANS, ASR_INTEL_VOLTS, ASR_X2_VOLTS, ASUS_800_VOLTS, ASUS_800_VTT_VOLTS (+38 more)

### Community 25 - "Elevated Actions"
Cohesion: 0.07
Nodes (36): Action, InstallPawnIo, LoadKernelModule, Process, Service, Startup, audit_lines_are_json(), AuditEntry (+28 more)

### Community 26 - "actor"
Cohesion: 0.17
Nodes (9): actor(), check_contract(), Class, Fast, Slow, Fake, spawn(), Stuck (+1 more)

### Community 27 - "Windows GPU Probe"
Cohesion: 0.12
Nodes (26): AdlInfo, cm_bdf(), CM_Get_DevNode_Registry_PropertyW(), CM_Get_Parent(), cm_u32(), CreateDXGIFactory1(), Desc1, dev_prop() (+18 more)

### Community 28 - "CPU Test Suite"
Cohesion: 0.10
Nodes (41): add(), Aes, aes_arm(), AES_BLOCKS, aes_hw(), aes_matches_fips_197(), aes_ni(), aes_sw() (+33 more)

### Community 29 - "Single Instance and SIDs"
Cohesion: 0.07
Nodes (17): Claim, claim(), First, Running, H, Instance, key(), Objects (+9 more)

### Community 30 - "data.rs"
Cohesion: 0.08
Nodes (39): categories_sizes_and_delete(), Category, .ALL, AppLog, AuditLog, BenchmarkHistory, Diagnostics, Exports (+31 more)

### Community 31 - "mod.rs"
Cohesion: 0.08
Nodes (40): write_fuzz_seeds(), cpu_label(), cpu_temps(), display_name(), fake_sysfs(), put(), read(), readings() (+32 more)

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

### Community 38 - "Action"
Cohesion: 0.16
Nodes (13): Action, Lock, Mini, Pause, Quit, Sensors, Speed, Toggle (+5 more)

### Community 39 - "drivetest.rs"
Cohesion: 0.11
Nodes (36): ata_kind(), ata_log_parses_newest_first(), ata_logs(), ata_selftest(), ata_status_text(), BUCKETS, ctrl(), Entry (+28 more)

### Community 40 - "system_services.rs"
Cohesion: 0.12
Nodes (12): dispatch(), Finished, a_refused_change_leaves_no_undo_and_resets_the_editor(), combo(), ES, hash_name(), link_button(), Modal (+4 more)

### Community 41 - "decode.rs"
Cohesion: 0.15
Nodes (32): bars(), bars_and_ext(), bytes_text(), cap(), cap_name(), cap_node(), caps(), dev() (+24 more)

### Community 42 - "smart.rs"
Cohesion: 0.17
Nodes (18): ata_attr_name(), ata_attributes(), ata_smart(), AtaAttr, entry(), from_smartctl(), from_status(), HealthLevel (+10 more)

### Community 43 - "windows.rs"
Cohesion: 0.10
Nodes (34): driver_installed(), driver_modules_found(), CloseFn, CreateMutexW(), ExecFn, fn_name(), GetProcAddress(), Handle (+26 more)

### Community 44 - "peripherals.rs"
Cohesion: 0.09
Nodes (23): Data, Audio, Bars, bars(), bars_of(), batteries(), BATTERY, bluetooth_batteries() (+15 more)

### Community 45 - "main.rs"
Cohesion: 0.13
Nodes (12): heavy_test(), main(), Out, out(), Html, Json, JsonTxt, Txt (+4 more)

### Community 46 - "os.rs"
Cohesion: 0.09
Nodes (31): cpu_groups(), FreeConsole(), GetActiveProcessorCount(), GetActiveProcessorGroupCount(), GetConsoleProcessList(), GetCurrentThread(), GetThreadPriority(), GroupAffinity (+23 more)

### Community 47 - "diff.rs"
Cohesion: 0.11
Nodes (31): g(), Change, change(), ChangeKind, Added, Modified, Removed, cli() (+23 more)

### Community 48 - "details"
Cohesion: 0.19
Nodes (17): is_protected(), is_third_party(), protected_and_third_party_classification(), action_of(), apply(), config2(), dacl_sddl(), details() (+9 more)

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
Cohesion: 0.13
Nodes (25): collect(), decode_dimm(), Dimm, dimm_temps(), DUMP, dumps(), FakeHub, golden_ddr5_matches_cpuz() (+17 more)

### Community 53 - "proc_ops.rs"
Cohesion: 0.12
Nodes (25): acts_on_a_child_process(), check(), class(), connections(), dump(), eco(), end(), err() (+17 more)

### Community 54 - "telemetry.rs"
Cohesion: 0.09
Nodes (10): cli_log(), every_source_keeps_the_contract(), Gpus, lock(), catch_ctrl_c(), ctrl_c(), sources(), status() (+2 more)

### Community 55 - "procs.rs"
Cohesion: 0.09
Nodes (27): anti_cheat(), classify(), cpu_percent(), grouping_and_critical(), HIGH_CPU, HISTORY, Kind, App (+19 more)

### Community 56 - "display_drm.rs"
Cohesion: 0.11
Nodes (20): card_modes(), CardRes, CONNECTED, connector_name(), connector_names_match_the_kernels(), Crtc, driver_of(), FLAG_DBLSCAN (+12 more)

### Community 57 - "welcome.rs"
Cohesion: 0.15
Nodes (15): cpu_suite_modes(), ctx(), memory_and_latency_views(), storage_view_warns_before_writing(), themed(), continuing_without_ticking_means_no_update_check(), ES, harness() (+7 more)

### Community 58 - "gpubench.rs"
Cohesion: 0.11
Nodes (17): Ctl, adapters(), BIG, block_on(), flops(), FLOPS_PER_ITER, FMA, Gpu (+9 more)

### Community 59 - "layout_view.rs"
Cohesion: 0.10
Nodes (22): editor(), ES, KINDS, PT_BR, THROTTLE_DEVICES, ALIASES, aliases_resolve_to_this_machines_sensors(), bar() (+14 more)

### Community 60 - "fuzz.yml"
Cohesion: 0.06
Nodes (32): MINUTES, name, runs-on, steps, strategy, timeout-minutes, minutes, jobs (+24 more)

### Community 61 - "intel.rs"
Cohesion: 0.07
Nodes (45): ADL, ARL, arrow_lake(), BCLK_FREQ, decode(), degrade(), F, f() (+37 more)

### Community 62 - "software.rs"
Cohesion: 0.24
Nodes (12): after(), collect(), created_secs(), CV, dates(), fields(), mdy(), os_line() (+4 more)

### Community 63 - "metrics.rs"
Cohesion: 0.10
Nodes (17): Csv, csv_header_rows_and_new_columns(), r(), Unit, Ampere, Celsius, Count, Mbps (+9 more)

### Community 64 - "proc_net.rs"
Cohesion: 0.10
Nodes (25): begin(), inet_diag_message(), KERNEL_NETWORK, KEYWORDS, LAST, nettop_csv(), on_event(), parse_inet_diag() (+17 more)

### Community 65 - "theme.rs"
Cohesion: 0.10
Nodes (22): layout_editor_edits(), a_row_pins_under_its_card(), system_task_manager_settings(), tray_icons_settings(), apply(), DARK, fonts(), GAP (+14 more)

### Community 66 - "tray.rs (2)"
Cohesion: 0.06
Nodes (26): Click, App, Mini, argb(), build(), CLICK, ES, EVENTS (+18 more)

### Community 67 - "bottleneck.rs"
Cohesion: 0.16
Nodes (44): a_real_presentmon_capture_tells_cpu_bound_loading_from_gpu_bound_play(), a_session_sums_up_time_per_class(), analyze(), analyze_verdicts(), avg(), base(), class(), classify() (+36 more)

### Community 68 - "mod.rs (3)"
Cohesion: 0.16
Nodes (22): Adapter, adapter_nodes(), adapters(), apply_nvml(), backend(), collect(), fill_from_db(), gen_speed() (+14 more)

### Community 69 - "live_sessions.rs"
Cohesion: 0.15
Nodes (18): a_mangohud_log_imports_as_a_session(), bars(), COLORS, compare(), sessions_dir(), ES, import(), key() (+10 more)

### Community 70 - "procs.rs (2)"
Cohesion: 0.12
Nodes (8): Totals, Collector, collector_builds_rows_and_rates(), Meta, Prev, Sampler, Snapshot, worker()

### Community 71 - "bottleneck_ui.rs"
Cohesion: 0.09
Nodes (23): Analysis, Confidence, High, Low, Medium, Evidence, fill(), Recommendation (+15 more)

### Community 72 - "mod.rs (4)"
Cohesion: 0.15
Nodes (20): amd_pstates(), apic_tree(), arm_free_sysctl(), Cache, cache_line(), Cluster, collect(), Cpu (+12 more)

### Community 73 - "trayicon.rs"
Cohesion: 0.11
Nodes (23): Threshold, Tone, Danger, Good, Normal, Warning, detail_levels_filter_cards(), placeholder() (+15 more)

### Community 74 - "windows.rs (2)"
Cohesion: 0.12
Nodes (23): hard_faults_per_sec(), open(), Array, reads_a_wildcard_counter(), array(), collect(), cpu_load(), cpu_mhz() (+15 more)

### Community 75 - "telemetry.rs (2)"
Cohesion: 0.11
Nodes (27): throttle_kind(), AT_LIMIT, c(), cli_once(), CLOCK_DROP, Cond, cond(), Debounce (+19 more)

### Community 76 - "chipset.rs"
Cohesion: 0.14
Nodes (26): amd_chipset(), amd_umc(), chipset_names(), chipset_token(), ChipsetSummary, collect(), decode_umc(), decode_umc_more() (+18 more)

### Community 77 - "telemetry.rs (3)"
Cohesion: 0.16
Nodes (14): Reading, battery(), cpu_extra(), disks(), DiskSnap, gpu(), gpu_group(), ids() (+6 more)

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
Cohesion: 0.14
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

### Community 86 - "macos.rs"
Cohesion: 0.15
Nodes (24): battery(), call(), conn(), decode(), IOConnectCallStructMethod(), IOObjectRelease(), IOServiceGetMatchingService(), IOServiceMatching() (+16 more)

### Community 87 - "large_pages.rs"
Cohesion: 0.08
Nodes (4): alloc(), HUGE, lock_memory_privilege(), Region

### Community 88 - "Stage"
Cohesion: 0.14
Nodes (19): command(), download_and_verify(), downloads_and_verifies_latest_release(), fetch_and_verify(), in_path(), Installer, launch(), open_path() (+11 more)

### Community 89 - "robustness.rs"
Cohesion: 0.12
Nodes (14): edid_blocks(), FIXTURE, hardware_and_mangohud_logs(), layout_files(), n(), output_lines_ignore_chunking(), pci_config_space(), presentmon_csv_files() (+6 more)

### Community 90 - "report.rs"
Cohesion: 0.14
Nodes (24): esc(), fields(), find(), find_section(), groups(), gv(), hex(), hex_rows() (+16 more)

### Community 91 - "overlay.rs"
Cohesion: 0.09
Nodes (4): apply(), each(), Find, Style

### Community 92 - "pci/windows.rs"
Cohesion: 0.05
Nodes (50): description, label, placeholder, render, value, body, attributes, id (+42 more)

### Community 93 - "soak.py"
Cohesion: 0.23
Nodes (9): _linux_sampler(), _macos_sampler(), make_sampler(), profile(), report(), run_gui(), run_sampler(), _windows_sampler() (+1 more)

### Community 94 - "nvml.rs"
Cohesion: 0.17
Nodes (13): db_apple(), cstr(), Dev, FanRpm, get(), Info, load(), Memory (+5 more)

### Community 95 - "live.rs"
Cohesion: 0.11
Nodes (11): HotkeyAction, .ALL, Capture, CycleStyle, EditMode, Marker, ToggleOverlay, Hotkeys (+3 more)

### Community 96 - "services.rs"
Cohesion: 0.11
Nodes (26): small_text_parsers(), a_dacl_that_refuses_administrators(), CHANGE_CONFIG, dependency_chain_is_cycle_safe_and_filters_work(), matches_filter(), parse_systemd_secs(), parse_systemd_show(), populate_dependents() (+18 more)

### Community 97 - "windows.rs (4)"
Cohesion: 0.16
Nodes (6): exits_config(), GlobalMutex, live_dump(), Module, PawnIo, SioSlot

### Community 98 - "devices.rs"
Cohesion: 0.16
Nodes (21): adapter(), battery(), device_card(), drive(), ES, gb(), Health, health() (+13 more)

### Community 99 - "pins.rs"
Cohesion: 0.15
Nodes (22): cache_id(), card_id(), card_text(), current_card(), enter_card(), ES, is_pinned(), latest() (+14 more)

### Community 100 - "screens.rs"
Cohesion: 0.14
Nodes (24): nearest_anchor(), nine_anchors_and_offsets(), offset_for(), place(), A, B, C, choose() (+16 more)

### Community 101 - "startup.rs"
Cohesion: 0.08
Nodes (8): StartupOp, SetEnabled, apply(), enum_reg_strings(), read_approved_map(), read_fw_post_time(), RegKey, to_wide()

### Community 102 - "diskbench.rs"
Cohesion: 0.14
Nodes (15): a_tiny_run_measures_and_cleans_up(), a_tiny_sustained_write_cleans_up(), bench(), phase(), rates(), rates_math(), Run, Score (+7 more)

### Community 103 - "update.rs"
Cohesion: 0.19
Nodes (11): Channel, .ALL, Alpha, Beta, Snapshot, Stable, check(), cli() (+3 more)

### Community 104 - "session.rs"
Cohesion: 0.12
Nodes (29): windows_from_session(), Summary, corrupt_sessions_stay_bounded(), long_sessions_are_linear(), Band, ChartLine, comparability(), comparisons_and_repeats() (+21 more)

### Community 105 - "zeez-live.md"
Cohesion: 0.11
Nodes (21): Now card, live readings, session analysis, glow renderer with automatic wgpu fallback (ADR 0013), MangoHud runtime component (detect + pkexec install), PresentMon sidecar bundled, Decisions Q1-Q10, Anti-cheat and privacy rule (no strong handles), System hub navigation (Q5): Details as view mode, PawnIO x kernel anti-cheat compatibility list (SP7) (+13 more)

### Community 106 - "ci.yml"
Cohesion: 0.18
Nodes (23): steps, steps, name, runs-on, steps, headless, visual, steps (+15 more)

### Community 107 - "battery.rs"
Cohesion: 0.16
Nodes (17): Battery, chemistry_name(), GUID_DEVICE_BATTERY, info(), IOCTL_BATTERY_QUERY_INFORMATION, IOCTL_BATTERY_QUERY_STATUS, IOCTL_BATTERY_QUERY_TAG, ioreg_plist() (+9 more)

### Community 108 - "usb.rs"
Cohesion: 0.11
Nodes (24): CM_Get_Device_IDW(), CM_Get_Parent(), controller(), dev_id(), multi_sz(), parent(), reg_prop(), reg_str() (+16 more)

### Community 109 - "startup.rs (2)"
Cohesion: 0.11
Nodes (22): extract_exe_path(), StartupRow, StartupSource, CommonFolder, HkcuRun, HkcuRunOnce, HklmRun, HklmRun32 (+14 more)

### Community 110 - "a11y.rs"
Cohesion: 0.12
Nodes (19): at_every_zoom(), beyond(), bottleneck_sample(), charts_and_bars_say_what_they_show(), collisions(), every_live_tab_holds_together_at_every_scale(), every_page_holds_together_at_every_scale(), every_tools_tab_holds_together_at_every_scale() (+11 more)

### Community 111 - "nightly.yml"
Cohesion: 0.09
Nodes (21): minutes, jobs, soak, default, description, name, on, schedule (+13 more)

### Community 112 - "Ctx"
Cohesion: 0.08
Nodes (31): batteries(), collect(), opt(), cluster_cpus(), section(), uefi(), build(), cppc() (+23 more)

### Community 113 - "npu.rs"
Cohesion: 0.15
Nodes (16): CORE_COMPUTE, Count, CreateList, devices(), GENERIC_ML, GetAdapter, GetProp, GRAPHICS (+8 more)

### Community 114 - "Step"
Cohesion: 0.09
Nodes (22): BenchView, Cpu, Gpu, History, Latency, Memory, Storage, Stress (+14 more)

### Community 115 - "system_users.rs"
Cohesion: 0.08
Nodes (17): UserSession, Access, ES, kernel_module(), pawnio_doc(), PT_BR, relaunch_elevated(), aggregate_users() (+9 more)

### Community 116 - "availability.rs"
Cohesion: 0.06
Nodes (47): Capability, .ALL, BoardSensors, ChipsetTemp, CpuSensors, MemoryTimings, Spd, caps() (+39 more)

### Community 117 - "live.rs (2)"
Cohesion: 0.17
Nodes (13): EDGE, exe_name(), hotkey_conflicts(), hotkeys_normalize_and_conflict(), normalize_keys(), Overlay, portal_request_path(), portal_trigger() (+5 more)

### Community 118 - "PresentMon bundled sidecar (ETW)"
Cohesion: 0.14
Nodes (13): ADR 0001: ZeeZ Live is the brand, ZeeZ Live (Overlay, Capture, Reports), ADR 0003: FPS on Windows from PresentMon, PresentMon bundled sidecar (ETW), ADR 0004: FPS on Linux from MangoHud, MangoHud integration (detect, install, config, logs), SetLayeredWindowAttributes click-through fix, global-hotkey crate (RegisterHotKey/Carbon/XGrabKey) (+5 more)

### Community 119 - "Cell"
Cohesion: 0.19
Nodes (7): CoreRow, ES, group_of(), PT_BR, Cell, Num, Text

### Community 120 - "pci/windows.rs"
Cohesion: 0.13
Nodes (24): acpi_table(), CM_Get_Parent(), DevInfoData, DevPropKey, DIGCF_ALLCLASSES, DIGCF_PRESENT, Guid, hwid_field() (+16 more)

### Community 121 - "cstates.rs"
Cohesion: 0.16
Nodes (9): array(), LAST, PREV, Query, QUERY, Raw, raw(), read() (+1 more)

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
Cohesion: 0.08
Nodes (50): Frame, App, apps(), begin_capture(), CAPTURE_MAX, Demo, demo(), due() (+42 more)

### Community 126 - "detail.rs"
Cohesion: 0.14
Nodes (14): BASIC, Level, Advanced, .ALL, Basic, Expert, level_of(), levels() (+6 more)

### Community 127 - "clocks.rs"
Cohesion: 0.14
Nodes (18): active_from_deltas(), active_mhz(), add_chain_mhz(), adds(), aperf_mperf(), aperf_mperf_regs(), chain(), core_mhz() (+10 more)

### Community 128 - "table.rs"
Cohesion: 0.19
Nodes (21): cols(), columns_move_and_keep_their_order(), ES, filter_sort_and_group(), header_resizes_moves_and_sorts_with_the_mouse(), Item, Group, Row (+13 more)

### Community 129 - "superio.rs"
Cohesion: 0.28
Nodes (12): apply(), fintek(), Io, ite(), nct6687_like_the_dump(), nct_ec(), nct_new(), nct_old() (+4 more)

### Community 130 - "layout_view.rs"
Cohesion: 0.05
Nodes (12): Source, Batteries, Board, Cpu, Dimms, Disks, Drives, Frames (+4 more)

### Community 131 - "query.rs"
Cohesion: 0.26
Nodes (15): half_typed_and_odd_input_never_fails(), hit(), matches(), number(), parse(), plain(), row(), syntax() (+7 more)

### Community 132 - "cpu_os.rs"
Cohesion: 0.14
Nodes (11): CODE_INTEGRITY, KVA_SHADOW, NtQuerySystemInformation(), Power, power(), query(), read(), SPECULATION_CONTROL (+3 more)

### Community 133 - "linux.rs (2)"
Cohesion: 0.14
Nodes (18): read_trim(), read_u64(), adapter(), cards(), dpm(), freq(), hwmon_live(), link() (+10 more)

### Community 134 - "history.rs"
Cohesion: 0.21
Nodes (18): add(), dates(), Entry, entry(), join(), load(), load_from(), memory_line() (+10 more)

### Community 135 - "health.rs"
Cohesion: 0.26
Nodes (16): a_healthy_build_has_nothing_to_say(), check(), Display, each_mistake(), Facts, Finding, finding(), good() (+8 more)

### Community 136 - "live.rs (3)"
Cohesion: 0.11
Nodes (20): Anchor, .ALL, Bottom, BottomLeft, BottomRight, Center, Left, Right (+12 more)

### Community 137 - "dependencies.md"
Cohesion: 0.09
Nodes (19): ADR 0006: Dependencies, rusqlite (bundled SQLite), windows-sys, ADR 0013: glow renderer with wgpu fallback, glow (OpenGL) renderer, wgpu fallback (CZ_RENDERER=wgpu, WARP), Benchmark principles (safe, reproducible, 1000 = reference 7900X), Benchmarks (CPU suite, core-to-core latency, memory, GPU, storage, stability) (+11 more)

### Community 138 - "table.rs (2)"
Cohesion: 0.16
Nodes (12): R, cell_text(), Column, Column<'a, R>, fit(), fit_width(), Header, shown() (+4 more)

### Community 139 - "gui/tools.rs"
Cohesion: 0.12
Nodes (19): changes(), CHART_BOTTOM, CHART_HEIGHT, CHART_LEFT, CHART_LINE, Compare, diff_as_text(), diff_text() (+11 more)

### Community 140 - "win.rs"
Cohesion: 0.24
Nodes (10): adl_alloc(), apply_nvapi(), nv_gpu(), NvApi, nvapi(), NvClocks, NvGpu, NvThermal (+2 more)

### Community 141 - "Page"
Cohesion: 0.12
Nodes (16): NAV, NAV_BOTTOM, Page, About, Benchmarks, Devices, Graphics, Live (+8 more)

### Community 142 - ".render_table"
Cohesion: 0.22
Nodes (14): CtxCmd, Affinity, Dump, Eco, End, Inspect, Priority, RestartShell (+6 more)

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
Cohesion: 0.04
Nodes (51): ADR 0008: Planning documents are in English, Command palette (Ctrl+K), Shared page patterns (detail levels, Ctrl+K palette, banners, pin, units), rusqlite (bundled SQLite), windows-sys crate, ADRs in docs/adr, Contract test suites and fakes, core crate (pure logic, no OS/GUI) (+43 more)

### Community 147 - "pkgver.py"
Cohesion: 0.18
Nodes (3): Strict, _test(), versions()

### Community 148 - "boxed"
Cohesion: 0.09
Nodes (8): boxed(), shown(), App, gpu_name(), mainboard_tab(), App, memory_tab(), App

### Community 149 - "applog.rs"
Cohesion: 0.22
Nodes (10): diagnostics_bundle(), info(), install_panic_hook(), LOCK, MAX, path(), warn(), write() (+2 more)

### Community 150 - "live_mangohud.rs"
Cohesion: 0.22
Nodes (12): config_home(), config_path(), configure(), env_path(), home(), Install, Distro, Flatpak (+4 more)

### Community 151 - "raster.py"
Cohesion: 0.20
Nodes (8): dist_seg(), in_poly(), in_rrect(), on_arc(), png(), render(), render_png(), sample()

### Community 152 - "Sub"
Cohesion: 0.17
Nodes (18): median_spread(), merge(), points(), repeat(), scores_and_medians(), Sub, total(), every_test_runs() (+10 more)

### Community 153 - "identify.rs"
Cohesion: 0.33
Nodes (11): ascii(), ata(), ata_fields_and_dump(), ata_text(), hex_dump(), kelvin(), le16(), le32() (+3 more)

### Community 154 - "direct_io.rs"
Cohesion: 0.20
Nodes (8): ALIGN, Buffer, FILE_FLAG_NO_BUFFERING, FILE_FLAG_WRITE_THROUGH, open(), read_at(), uncached_round_trip(), write_at()

### Community 155 - "bench_tab.rs"
Cohesion: 0.17
Nodes (9): bar(), bytes_text(), ES, Mode, Cpu, Memory, PT_BR, save_row() (+1 more)

### Community 156 - "tempdir.rs"
Cohesion: 0.32
Nodes (14): base_is_safe(), base_validation_follows_symlinks_in_the_path(), base_validation_rejects_swappable_ancestors(), base_validation_rejects_swappable_symlinks(), base_validation_requires_sticky_when_shared(), create(), create_exclusive_private(), private_and_ours() (+6 more)

### Community 157 - "firmware.rs"
Cohesion: 0.19
Nodes (7): Header, header(), text(), acpi(), Firmware, read(), SECURE_BOOT

### Community 158 - "StartupType"
Cohesion: 0.20
Nodes (9): StartupType, Automatic, AutomaticDelayed, Disabled, .EDITABLE, Manual, Masked, Static (+1 more)

### Community 159 - "dependabot.yml"
Cohesion: 0.13
Nodes (14): patterns, prefix, patterns, actions, egui, interval, updates, commit-message (+6 more)

### Community 160 - "ci.yml (2)"
Cohesion: 0.14
Nodes (15): container, name, runs-on, strategy, name, os, distros, native (+7 more)

### Community 161 - "superio.rs (2)"
Cohesion: 0.28
Nodes (9): cfg_rd(), cfg_rd16(), cfg_wr(), Chip, detect(), Fake, finish(), probe() (+1 more)

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
Cohesion: 0.24
Nodes (9): cmd(), logs(), adapter(), apple_nm(), hex_id(), ioreg_edids(), metal(), probe() (+1 more)

### Community 168 - "memplus.rs"
Cohesion: 0.15
Nodes (22): measure(), pair(), ROUNDS, statistical(), statistical_mode_measures_unpinned(), summary(), summary_reads_the_upper_triangle(), two_cores_answer() (+14 more)

### Community 169 - "timers.rs"
Cohesion: 0.16
Nodes (15): find_agesa(), rom_agesa(), ACPI_PM_HZ, acpi_timer(), arm_cntfrq(), fadt(), fadt_pm_timer(), hz_text() (+7 more)

### Community 170 - "node"
Cohesion: 0.29
Nodes (9): bus_name(), controller_class(), controller_line(), formats(), gib(), link_line(), node(), pci_controller() (+1 more)

### Community 171 - "units.rs"
Cohesion: 0.21
Nodes (7): Temp, Celsius, Fahrenheit, Units, .DEFAULT, CURRENT, set()

### Community 172 - "net_io.rs"
Cohesion: 0.17
Nodes (4): FILTER, HARDWARE, read(), RTM_IFINFO2

### Community 173 - "Bottleneck Detector Design"
Cohesion: 0.12
Nodes (19): Bottleneck detector (measured evidence only), Build health checks on Overview, ADR 0014: No hardware balance bar in 1.0, Configuration checks / Build health (single channel, XMP off, PCIe, ReBAR...), Bottleneck Detector Design, Evidence-tied recommendations (no snake oil), Heuristic fallback without GPU Busy (lower confidence), Limiters (V-Sync, VRR, FPS cap, compositor) (+11 more)

### Community 174 - "History"
Cohesion: 0.16
Nodes (6): Buf, History, Series, Snapshot, Stat, Tick

### Community 175 - "probe"
Cohesion: 0.18
Nodes (14): Adl, adl(), dd(), DisplayDevice, displays(), EnumDisplayDevicesW(), live(), parse_hwid() (+6 more)

### Community 176 - "arm.rs"
Cohesion: 0.36
Nodes (8): build(), cpuinfo_field(), cpuinfo_midr(), implementer(), midr_text(), part(), sysctl_str(), sysctl_u64()

### Community 177 - "fixture.rs"
Cohesion: 0.26
Nodes (10): adapters(), app(), cpu(), ctx(), drives(), every_hardware_page_draws_from_the_fixtures(), every_overlay_style_stays_within_its_drawing_budget(), harness() (+2 more)

### Community 178 - "superio.rs (3)"
Cohesion: 0.15
Nodes (13): b(), Board, board_lookup(), board_map(), BOARDS, EC_GENERIC, ITE_GENERIC, ITE_OLD_GENERIC (+5 more)

### Community 179 - "os.rs (2)"
Cohesion: 0.26
Nodes (9): cpu_load(), host_processor_info(), mach_host_self(), mach_task_self_, parse_proc_stat(), PROCESSOR_CPU_LOAD_INFO, STOP, ticks() (+1 more)

### Community 180 - ".memory_more"
Cohesion: 0.22
Nodes (10): Timing, active_profile(), App, edac(), ES, finds_the_running_profile_and_tuning(), Live, num() (+2 more)

### Community 181 - "diskio.rs"
Cohesion: 0.30
Nodes (11): a_reset_counter_is_no_activity_not_garbage(), Counters, DISKSTATS, diskstats_keeps_whole_disks_only(), diskstats_units(), is_whole_disk(), parse_diskstats(), Rates (+3 more)

### Community 182 - "interfaces"
Cohesion: 0.19
Nodes (14): CM_Get_Device_Interface_List_SizeW(), CM_Get_Device_Interface_ListW(), CM_Get_DevNode_PropertyW(), Guid, GUID_DISK, GUID_USB_HUB, interfaces(), KEY_INSTANCE_ID (+6 more)

### Community 183 - "RecoveryAction"
Cohesion: 0.25
Nodes (7): RecoveryAction, .EDITABLE, None, Reboot, Restart, RunProgram, sc_type()

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

### Community 190 - ".mainboard_more"
Cohesion: 0.19
Nodes (6): App, ES, link_status(), link_text(), PT_BR, tree()

### Community 191 - "actions.rs (2)"
Cohesion: 0.18
Nodes (8): Priority, AboveNormal, .ALL, BelowNormal, High, Idle, Normal, Realtime

### Community 192 - "best_hz"
Cohesion: 0.36
Nodes (8): best_hz(), fixture(), mode(), ModeInfo, parse_modeinfo(), refresh_from_the_timing(), short_or_garbled_input_never_panics(), the_best_mode_is_at_the_current_resolution()

### Community 193 - "edid.rs"
Cohesion: 0.31
Nodes (10): decode(), displayid(), displayid_ext(), dtd(), Monitor, parse(), PNP, pnp_name() (+2 more)

### Community 194 - "layout.rs (2)"
Cohesion: 0.18
Nodes (10): Style, .ALL, Bar, Graph, Number, Ring, Theme, Dark (+2 more)

### Community 195 - "Impact"
Cohesion: 0.29
Nodes (5): Impact, High, Low, Medium, NotMeasured

### Community 196 - "smbios.rs (2)"
Cohesion: 0.33
Nodes (8): ioreg_hex(), synth(), uuid_bytes(), GetSystemFirmwareTable(), apple_table(), fetch(), sysfs_fallback(), sysfs_table()

### Community 197 - "encryption.rs"
Cohesion: 0.31
Nodes (6): crypt(), DONE, read(), STARTED, Status, status()

### Community 200 - "analyze"
Cohesion: 0.32
Nodes (4): analyze(), frame_trend(), selftest(), slope()

### Community 203 - "soak.rs"
Cohesion: 0.23
Nodes (9): App, enabled(), frame(), NAME, overlay_frame(), overlay_only(), PID, start() (+1 more)

### Community 204 - "jep106.rs"
Cohesion: 0.25
Nodes (6): mfr(), mfr8(), BANK1, BANK2, name(), OTHER

### Community 205 - "contrast"
Cohesion: 0.32
Nodes (6): chart_colours_meet_the_graphics_minimum(), class_colours_are_distinct_and_readable(), contrast(), luminance(), surfaces(), text_colours_meet_aa_on_every_surface()

### Community 206 - "db.rs"
Cohesion: 0.27
Nodes (7): AMD, INTEL, lookup(), NV, s(), Spec, TABLE

### Community 207 - "mod.rs (8)"
Cohesion: 0.38
Nodes (8): class_desc(), Ids, ids_parse_and_labels(), mcfg(), mcfg_base(), parse_ids(), vendor_label(), vendor_short()

### Community 208 - ".risk"
Cohesion: 0.29
Nodes (5): Risk, Critical, Irreversible, Normal, critical()

### Community 209 - "startup.rs (4)"
Cohesion: 0.39
Nodes (8): attr_value(), decode_startup_approved(), encode_startup_approved(), filetime_to_ymd(), impact_thresholds_and_wdi_xml_parser(), parse_wdi_startup_xml(), startup_approved_round_trip_and_filetime_decoding(), tag_u64()

### Community 211 - "wddm"
Cohesion: 0.33
Nodes (6): D3DKMTCloseAdapter(), D3DKMTOpenAdapterFromLuid(), D3DKMTQueryAdapterInfo(), OpenLuid, QueryInfo, wddm()

### Community 213 - "shotdiff.py"
Cohesion: 0.29
Nodes (4): compare(), diff(), read_png(), _test()

### Community 214 - "n"
Cohesion: 0.70
Nodes (4): imported_reports(), n(), settings_from_any_version(), update_answers()

### Community 215 - "layout.rs (3)"
Cohesion: 0.22
Nodes (9): Kind, Bar, Clock, Header, LabelValue, Separator, Sparkline, Throttle (+1 more)

### Community 216 - "memory.rs (2)"
Cohesion: 0.60
Nodes (4): Composition, meminfo(), MEMINFO, parse_meminfo()

### Community 217 - "Gen"
Cohesion: 0.50
Nodes (4): Gen, Ddr, Ddr2, Sdr

### Community 219 - "ci.yml (4)"
Cohesion: 0.22
Nodes (8): env, CARGO_TERM_COLOR, name, on, pull_request, push, workflow_dispatch, branches

### Community 220 - "Clock"
Cohesion: 0.50
Nodes (4): Clock, Auto, Ghz, Mhz

### Community 222 - "alerts.rs"
Cohesion: 0.21
Nodes (6): defaults(), fires_only_when_the_whole_window_is_over(), firing(), pts(), Rule, rules_load_from_older_settings()

### Community 223 - "limits.rs"
Cohesion: 0.24
Nodes (8): amd_cppc_highest(), cppc_ranks(), intel_hwp_highest(), intel_limits_decode(), intel_pkg_limits(), Limit, ranks(), Tour

### Community 224 - "netio.rs"
Cohesion: 0.43
Nodes (7): Counters, NET_DEV, parse_proc_net_dev(), proc_net_dev(), Rates, rates(), rates_in_megabits()

### Community 225 - "Size"
Cohesion: 0.50
Nodes (4): Size, Decimal, Iec, Windows

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
Cohesion: 0.39
Nodes (6): Read, compress(), hex(), Sha256, sha256_fips_vectors(), sha256_of()

### Community 238 - "query.rs (2)"
Cohesion: 0.29
Nodes (7): Op, Contains, Eq, Ge, Gt, Le, Lt

### Community 240 - "tr"
Cohesion: 0.10
Nodes (21): tr(), App, instructions(), topology(), alerts(), ES, lang_of(), page() (+13 more)

### Community 242 - "ci.yml (6)"
Cohesion: 0.33
Nodes (6): msi-releases, name, permissions, runs-on, permissions, contents

### Community 243 - "SysTab"
Cohesion: 0.14
Nodes (13): DetailTab, Network, Overview, Security, Threads, Windows, SysTab, AppHistory (+5 more)

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
- **502 isolated node(s):** `Cpu`, `Memory`, `Hybrid`, `Scaling`, `Suite` (+497 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 1873 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **26 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `Ctx` connect `Ctx` to `Sensors Page and Stress Monitor`, `egui`, `x86.rs`, `trf`, `System Performance Charts`, `layout_view.rs`, `history.rs`, `tr`, `CPU Benchmark Runner`, `Memory Benchmark`, `Storage Drives Probe`, `gui/mod.rs`, `Network Adapters`, `Probe Cache and ACPI`, `CPU Sensors and Power Limits`, `bench_tab.rs`, `mod.rs`, `timers.rs`, `main.rs`, `diff.rs`, `overview.rs`, `mod.rs (2)`, `telemetry.rs`, `software.rs`, `mod.rs (3)`, `bottleneck_ui.rs`, `mod.rs (4)`, `telemetry.rs (2)`, `chipset.rs`, `telemetry.rs (3)`, `devices.rs`, `battery.rs`, `usb.rs`, `npu.rs`, `system_users.rs`, `availability.rs`, `clocks.rs`?**
  _High betweenness centrality (0.125) - this node is a cross-community bridge._
- **Are the 7 inferred relationships involving `Ctx` (e.g. with `cli()` and `.page()`) actually correct?**
  _`Ctx` has 7 INFERRED edges - model-reasoned connections that need verification._
- **What connects `Cpu`, `Memory`, `Hybrid` to the rest of the system?**
  _502 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `Sensors Page and Stress Monitor` be split into smaller, more focused modules?**
  _Cohesion score 0.04460340837365874 - nodes in this community are weakly interconnected._
- **Why does `tr()` connect `tr` to `Sensors Page and Stress Monitor`, `egui`, `table.rs`, `trf`, `System Performance Charts`, `widgets.rs`, `tr`, `table.rs (2)`, `gui/tools.rs`, `system_history.rs`, `gui/mod.rs`, `.render_table`, `boxed`, `system.rs`, `bench_tab.rs`, `data.rs`, `palette.rs`, `system_services.rs`, `overview.rs`, `.memory_more`, `welcome.rs`, `layout_view.rs`, `intel.rs`, `.mainboard_more`, `tray.rs (2)`, `live_sessions.rs`, `bottleneck_ui.rs`, `.stage_shot`, `pci/windows.rs`, `live.rs`, `devices.rs`, `pins.rs`, `system_users.rs`, `detail.rs`?**
  _High betweenness centrality (0.057) - this node is a cross-community bridge._
- **Are the 148 inferred relationships involving `tr()` (e.g. with `.banner()` and `.frame()`) actually correct?**
  _`tr()` has 148 INFERRED edges - model-reasoned connections that need verification._
- **Should `egui` be split into smaller, more focused modules?**
  _Cohesion score 0.04683544303797468 - nodes in this community are weakly interconnected._