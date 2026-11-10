//! Fake data for drawing the hardware pages in tests (M2-01): a report, a
//! context with every probe off and no hardware backend, a fixed CPU, fixed
//! slow probes (`gui::cache`), devices, and a telemetry engine over fixed
//! readings. The result is the same on every machine and in CI.
//!
//! Still read from the OS while a page draws: C-states (Processor) and the
//! volume encryption status (Devices). Both are cheap, need no rights, and
//! do not change what the tests look at.

use super::*;
use crate::report::{Section, f, g, gv};
use crate::telemetry::{Telemetry, Unit};
use cpuzeez_core::metrics::Reading;

/// A context that probes nothing: every `Config` switch off, no hardware
/// backend, no SMBIOS, no PCI devices, and a fixed CPU.
pub fn ctx() -> Arc<Ctx> {
    let cfg = crate::config::Config {
        acpi: false,
        pci: false,
        dmi: false,
        sensor: false,
        smbus: false,
        display: false,
        use_display_api: false,
        bus_clock: false,
        chipset: false,
        spd: false,
        lpcio: false,
        smart: false,
        xoc: false,
        check_updates: false,
        ..Default::default()
    };
    let c = Ctx {
        cfg,
        hw: crate::hw::none(),
        smbios: std::sync::OnceLock::from(None),
        pci: std::sync::OnceLock::from(Vec::new()),
        cpu: std::sync::OnceLock::new(),
    };
    let _ = c.cpu.set(cpu());
    Arc::new(c)
}

pub fn cpu() -> crate::cpu::Cpu {
    use crate::cpu::{Cache, Cpu, db::Info};
    let cache =
        |level, kind, size, count, sharing| Cache { level, kind, size, ways: 8, line: 64, count, sharing, note: None };
    Cpu {
        vendor: "AuthenticAMD".into(),
        brand: "AMD Ryzen 7 7800X3D 8-Core Processor".into(),
        name: "AMD Ryzen 7 7800X3D".into(),
        info: Info {
            codename: Some("Raphael".into()),
            package: Some("Socket AM5 (LGA1718)".into()),
            technology: Some("5 nm".into()),
            tdp_w: Some(120.0),
            tjmax_c: Some(89.0),
            ..Default::default()
        },
        sig: (0x19, 0x61, 2),
        instructions: ["MMX", "SSE", "SSE2", "SSE3", "SSSE3", "SSE4.1", "SSE4.2", "AVX", "AVX2", "AVX-512F", "FMA3"]
            .map(String::from)
            .to_vec(),
        microcode: Some("0xA601209".into()),
        caches: vec![
            cache(1, 'D', 32 << 10, 8, 2),
            cache(1, 'I', 32 << 10, 8, 2),
            cache(2, 'U', 1 << 20, 8, 2),
            cache(3, 'U', 96 << 20, 1, 16),
        ],
        sockets: 1,
        cores: 8,
        threads: 16,
        base_mhz: Some(4200.0),
        max_mhz: Some(5050.0),
        bus_mhz: Some(100.0),
        ..Default::default()
    }
}

/// The report sections the hardware pages read.
pub fn report() -> Vec<Section> {
    let s = |title: &str, nodes| Section { title: title.into(), nodes };
    vec![
        s("Processors Information", vec![f("Microcode Revision", "0xA601209"), f("Number of cores", "8")]),
        s(
            "DMI",
            vec![
                g(
                    "DMI BIOS",
                    vec![f("vendor", "American Megatrends Inc."), f("version", "2.10"), f("date", "05/14/2026")],
                ),
                g("DMI Baseboard", vec![f("vendor", "Some Vendor"), f("model", "X670E BOARD"), f("revision", "1.0")]),
            ],
        ),
        s("BIOS", vec![f("UEFI", "Yes")]),
        s(
            "Chipset",
            vec![
                f("Northbridge", "AMD Ryzen SOC rev. 00"),
                f("Southbridge", "AMD X670E rev. 51"),
                f("Bus Specification", "PCI-Express 5.0 (32.0 GT/s)"),
                f("Graphic Interface", "PCI-Express 4.0"),
                f("PCI-E Link Width", "x16"),
                f("PCI-E Link Speed", "16.0 GT/s"),
                f("Memory Type", "DDR5"),
                f("Memory Size", "32 GBytes"),
                f("Channels", "2 x 64-bit"),
                f("Memory Frequency", "3000.0 MHz"),
                f("CAS# latency (CL)", "30.0"),
                f("RAS# to CAS# delay (tRCD)", "36"),
                f("RAS# Precharge (tRP)", "36"),
                f("Cycle Time (tRAS)", "76"),
            ],
        ),
        s("LPCIO", vec![f("LPCIO Vendor", "Nuvoton"), f("LPCIO Model", "NCT6799D")]),
        s(
            "Display Adapters",
            vec![gv(
                "Display adapter",
                0,
                vec![
                    f("Name", "NVIDIA GeForce RTX 4070"),
                    f("Memory size", "12 GB"),
                    g("Performance Level", vec![f("Core clock", "2475 MHz"), f("Memory clock", "10501 MHz")]),
                ],
            )],
        ),
    ]
}

/// Fixed readings: a warm, busy CPU, a GPU and a fan.
pub fn readings() -> Vec<Reading> {
    let r = |id: &str, group: &str, label: &str, unit, value| Reading {
        id: id.into(),
        group: group.into(),
        label: label.into(),
        unit,
        value,
    };
    let mut v = vec![
        r("cpu/temp/package", "CPU", "Package", Unit::Celsius, 71.5),
        r("cpu/clock/avg", "CPU", "Average clock", Unit::Mhz, 4875.0),
        r("cpu/clock/bus", "CPU", "Bus clock", Unit::Mhz, 100.0),
        r("cpu/load/total", "CPU", "Total load", Unit::Percent, 37.0),
        r("cpu/power/package", "CPU", "Package power", Unit::Watt, 64.2),
        r("gpu0/temp/core", "GPU · NVIDIA GeForce RTX 4070", "Core", Unit::Celsius, 58.0),
        r("gpu0/load/core", "GPU · NVIDIA GeForce RTX 4070", "Core load", Unit::Percent, 22.0),
        r("board/fan/sys1", "Mainboard", "System fan 1", Unit::Rpm, 1150.0),
        r("mem/used", "Memory", "Used", Unit::Megabytes, 14_200.0),
    ];
    for i in 0..8 {
        v.push(r(
            &format!("cpu/clock/core{i}"),
            "CPU",
            &format!("Core #{i} clock"),
            Unit::Mhz,
            4800.0 + 25.0 * i as f64,
        ));
        v.push(r(&format!("cpu/temp/core{i}"), "CPU", &format!("Core #{i}"), Unit::Celsius, 65.0 + i as f64));
    }
    v
}

pub fn drives() -> Vec<crate::storage::Drive> {
    vec![crate::storage::Drive {
        id: "0/0/-1/-1".into(),
        name: "Test NVMe SSD 2TB".into(),
        capacity: 2_000_398_934_016,
        ssd: true,
        bus: 17,
        health: vec![("Percentage used".into(), "3 %".into()), ("Temperature".into(), "41 C".into())],
        ..Default::default()
    }]
}

pub fn adapters() -> Vec<crate::network::Adapter> {
    vec![crate::network::Adapter {
        name: "Ethernet".into(),
        description: "Test 2.5GbE Controller".into(),
        kind: "Ethernet".into(),
        mac: "00:11:22:33:44:55".into(),
        link_mbps: Some(2500),
        up: true,
        ipv4: vec!["192.168.1.10/24".into()],
        ..Default::default()
    }]
}

/// The app on `page`, drawn from the fixtures above.
pub fn app(page: Page) -> App {
    let ctx = ctx();
    cache::seed_fixtures(drives());
    let tel = Telemetry::fixture(readings());
    // The first tick, so pages see values in their first frame.
    let t0 = std::time::Instant::now();
    while tel.snapshot().stats.is_empty() {
        tel.want("fixture", &[""]);
        assert!(t0.elapsed() < std::time::Duration::from_secs(10), "the fixture engine never ticked");
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let mon = sensors::Monitor::fixture(ctx.clone(), tel);
    let mut app = App::new(ctx, Arc::new(Mutex::new(Some(report()))), mon, None, Default::default());
    app.page = page;
    app.devices = devices::State::fixture(drives(), adapters());
    app
}

/// A harness that draws `page`'s body (as `page_body` does in the window),
/// themed and English.
pub fn harness(page: Page, size: egui::Vec2) -> egui_kittest::Harness<'static, App> {
    i18n::set(i18n::Lang::En);
    let report = report();
    let mut h = egui_kittest::Harness::builder().with_size(size).build_ui_state(
        move |ui, app: &mut App| {
            if !ui.style().text_styles.contains_key(&egui::TextStyle::Name("title".into())) {
                theme::install(ui.ctx());
                ui.ctx().request_repaint();
                return;
            }
            egui::ScrollArea::vertical().show(ui, |ui| app.page_body(ui, &report));
        },
        app(page),
    );
    // Theme, first layout, and the telemetry's first tick.
    for _ in 0..4 {
        h.run();
    }
    h
}

#[test]
fn every_hardware_page_draws_from_the_fixtures() {
    use egui_kittest::kittest::Queryable;
    for (page, expect) in [
        (Page::Processor, "AMD Ryzen 7 7800X3D"),
        (Page::Mainboard, "X670E BOARD"),
        (Page::Memory, "DDR5"),
        (Page::Graphics, "NVIDIA GeForce RTX 4070"),
        (Page::Devices, "Test NVMe SSD 2TB"),
        (Page::Sensors, "Package"),
        (Page::Tools, "Tools"),
        (Page::About, "CPU ZeeZ"),
    ] {
        let h = harness(page, egui::vec2(1120.0, 2400.0));
        assert!(h.query_all_by_label_contains(expect).count() > 0, "{page:?} does not show {expect:?}");
    }
}
