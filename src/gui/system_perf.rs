//! System › Performance tab (`S2a-06`): Task Manager style left device list with
//! live mini sparklines and right-hand device detail view (CPU with overall or
//! per-core heat grid, Memory with composition bar, Disks, Network, GPUs with
//! per-engine breakdown, and NPUs), plus a compact Summary view, selectable time
//! window (1m / 10m / 1h / session), and CSV export.

use super::cache;
use super::i18n::{tr, trf};
use super::plot::{self, Line};
use super::sensors::Monitor;
use super::theme::{self, icon};
use super::widgets::{self, Tone};
use crate::Ctx;
use crate::procs::Snapshot as ProcSnap;
use crate::telemetry::Series;
use eframe::egui::{self, Color32, CornerRadius, Frame, Margin, RichText, Sense, Stroke, Ui, vec2};
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum PerfDev {
    #[default]
    Cpu,
    Memory,
    Disk(usize),
    Net(usize),
    Gpu(usize),
    Npu(usize),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum CpuGraph {
    #[default]
    Overall,
    LogicalCores,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Span {
    #[default]
    OneMin,
    TenMin,
    OneHour,
    Session,
}

impl Span {
    pub const ALL: [Span; 4] = [Span::OneMin, Span::TenMin, Span::OneHour, Span::Session];

    pub fn secs(self) -> f64 {
        match self {
            Span::OneMin => 60.0,
            Span::TenMin => 600.0,
            Span::OneHour => 3600.0,
            Span::Session => 7200.0,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Span::OneMin => "1m",
            Span::TenMin => "10m",
            Span::OneHour => "1h",
            Span::Session => "Session",
        }
    }
}

#[derive(Default)]
pub struct State {
    pub dev: PerfDev,
    pub cpu_graph: CpuGraph,
    pub span: Span,
    pub summary_view: bool,
    pub message: Option<String>,
}

impl State {
    pub fn show(&mut self, ui: &mut Ui, ctx: &Arc<Ctx>, mon: &mut Monitor, procs: &ProcSnap) {
        let tel = mon.tel();
        tel.want("sys_perf", &["cpu/", "mem/", "disk", "net", "gpu"]);
        tel.set_keep(Duration::from_secs(self.span.secs() as u64));
        let series = tel.series_with(|id| {
            id.starts_with("cpu/load/")
                || id.starts_with("cpu/clock/")
                || id.starts_with("cpu/temp/")
                || id.starts_with("mem/")
                || id.starts_with("disk")
                || id.starts_with("net")
                || id.starts_with("gpu")
        });
        let now = series.iter().filter_map(|s| s.points.last().map(|p| p.0)).fold(0.0, f64::max);
        self.show_with_series(ui, ctx, &series, now, procs);
    }

    pub fn show_with_series(&mut self, ui: &mut Ui, ctx: &Arc<Ctx>, series: &[Series], now: f64, procs: &ProcSnap) {
        let p = theme::palette(ui);

        // Toolbar: Summary view toggle, time window selector, CSV export.
        ui.horizontal_wrapped(|ui| {
            if ui
                .selectable_label(self.summary_view, format!("{} {}", icon::SQUARES_FOUR, tr("Summary view")))
                .clicked()
            {
                self.summary_view = !self.summary_view;
            }
            ui.separator();
            ui.label(RichText::new(tr("Window:")).small().color(p.text_secondary));
            for sp in Span::ALL {
                if ui.selectable_label(self.span == sp, tr(sp.label())).clicked() {
                    self.span = sp;
                }
            }
            ui.separator();
            if ui.button(format!("{} {}", icon::EXPORT, tr("Export CSV"))).clicked() {
                let path = "performance.csv";
                let csv = export_series_csv(series);
                self.message = Some(match std::fs::write(path, csv) {
                    Ok(()) => trf("Saved {}", &[&path]),
                    Err(e) => e.to_string(),
                });
            }
            if let Some(m) = &self.message {
                ui.label(RichText::new(m).small().color(p.text_secondary));
            }
        });
        ui.add_space(6.0);

        // Discover dynamic devices from series + cached probes.
        let disk_groups = groups_for(series, "Disk · ");
        let net_groups = groups_for(series, "Network · ");
        let gpus = crate::gpu::adapters(ctx);
        let npus = cache::npus(ctx);

        if self.summary_view {
            self.render_summary(ui, series, &disk_groups, &net_groups, gpus, &npus);
            return;
        }

        let avail = ui.available_width();
        let left_w = (avail * 0.25).clamp(200.0, 260.0);
        ui.horizontal_top(|ui| {
            ui.allocate_ui(vec2(left_w, ui.available_height()), |ui| {
                egui::ScrollArea::vertical().id_salt("perf_sidebar").auto_shrink([false, false]).show(ui, |ui| {
                    // CPU tile
                    let cpu_pct = current_val(series, "cpu/load/total").unwrap_or(0.0);
                    let cpu_ghz = current_val(series, "cpu/clock/avg").map(|m| m / 1000.0).unwrap_or(0.0);
                    let cpu_sub = if cpu_ghz > 0.0 {
                        format!("{cpu_pct:.0}% · {cpu_ghz:.2} GHz")
                    } else {
                        format!("{cpu_pct:.0}%")
                    };
                    if dev_tile(
                        ui,
                        icon::CPU,
                        "CPU",
                        &cpu_sub,
                        &spark_pts(series, "cpu/load/total"),
                        p.series[0],
                        self.dev == PerfDev::Cpu,
                    ) {
                        self.dev = PerfDev::Cpu;
                    }

                    // Memory tile
                    let mem_used_gb = current_val(series, "mem/size/used").unwrap_or(0.0) / 1024.0;
                    let mem_pct = current_val(series, "mem/load/used").unwrap_or(0.0);
                    let mem_sub = format!("{mem_used_gb:.1} GB ({mem_pct:.0}%)");
                    if dev_tile(
                        ui,
                        icon::MEMORY,
                        tr("Memory"),
                        &mem_sub,
                        &spark_pts(series, "mem/load/used"),
                        p.series[3],
                        self.dev == PerfDev::Memory,
                    ) {
                        self.dev = PerfDev::Memory;
                    }

                    // Disk tiles
                    for (i, name) in disk_groups.iter().enumerate() {
                        let id = format!("disk{i}/load/active");
                        let act = current_val(series, &id).unwrap_or(0.0);
                        let sub = format!("{name} · {act:.0}%");
                        if dev_tile(
                            ui,
                            icon::HARD_DRIVES,
                            &format!("{} {i}", tr("Disk")),
                            &sub,
                            &spark_pts(series, &id),
                            p.series[2],
                            self.dev == PerfDev::Disk(i),
                        ) {
                            self.dev = PerfDev::Disk(i);
                        }
                    }

                    // Network tiles
                    for (i, name) in net_groups.iter().enumerate() {
                        let rx_id = format!("net{i}/rate/rx");
                        let tx_id = format!("net{i}/rate/tx");
                        let rx = current_val(series, &rx_id).unwrap_or(0.0);
                        let tx = current_val(series, &tx_id).unwrap_or(0.0);
                        let sub = format!("{name} · ↓{rx:.1} ↑{tx:.1} Mbps");
                        if dev_tile(
                            ui,
                            icon::GLOBE,
                            name,
                            &sub,
                            &spark_pts(series, &rx_id),
                            p.series[4],
                            self.dev == PerfDev::Net(i),
                        ) {
                            self.dev = PerfDev::Net(i);
                        }
                    }

                    // GPU tiles
                    for (i, g) in gpus.iter().enumerate() {
                        let load = gpu_busy(series, i);
                        let temp = current_val(series, &format!("gpu{i}/temp/core"));
                        let sub = match temp {
                            Some(t) => format!("{load:.0}% ({t:.0} °C)"),
                            None => format!("{load:.0}%"),
                        };
                        if dev_tile(
                            ui,
                            icon::GRAPHICS_CARD,
                            &format!("GPU {i}"),
                            &sub,
                            &spark_pts_prefix(series, &format!("gpu{i}/")),
                            p.series[1],
                            self.dev == PerfDev::Gpu(i),
                        ) {
                            self.dev = PerfDev::Gpu(i);
                        }
                        let _ = g;
                    }

                    // NPU tiles
                    for (i, n) in npus.iter().enumerate() {
                        if dev_tile(
                            ui,
                            icon::CIRCUITRY,
                            &format!("NPU {i}"),
                            &n.name,
                            &[],
                            p.series[5],
                            self.dev == PerfDev::Npu(i),
                        ) {
                            self.dev = PerfDev::Npu(i);
                        }
                    }
                });
            });

            ui.add_space(theme::GAP);

            ui.allocate_ui(vec2(avail - left_w - theme::GAP, ui.available_height()), |ui| {
                egui::ScrollArea::vertical().id_salt("perf_detail").auto_shrink([false, false]).show(
                    ui,
                    |ui| match self.dev {
                        PerfDev::Cpu => self.render_cpu(ui, ctx, series, now, procs),
                        PerfDev::Memory => self.render_memory(ui, series, now),
                        PerfDev::Disk(i) => {
                            let name = disk_groups.get(i).cloned().unwrap_or_else(|| format!("Disk {i}"));
                            self.render_disk(ui, ctx, series, now, i, &name);
                        }
                        PerfDev::Net(i) => {
                            let name = net_groups.get(i).cloned().unwrap_or_else(|| format!("Adapter {i}"));
                            self.render_net(ui, ctx, series, now, i, &name);
                        }
                        PerfDev::Gpu(i) => self.render_gpu(ui, ctx, series, now, i),
                        PerfDev::Npu(i) => self.render_npu(ui, &npus, i),
                    },
                );
            });
        });
    }

    fn render_summary(
        &mut self,
        ui: &mut Ui,
        series: &[Series],
        disks: &[String],
        nets: &[String],
        gpus: &[crate::gpu::Adapter],
        npus: &[crate::npu::Npu],
    ) {
        let p = theme::palette(ui);
        let cpu_pct = current_val(series, "cpu/load/total").unwrap_or(0.0);
        let mem_pct = current_val(series, "mem/load/used").unwrap_or(0.0);
        let mem_gb = current_val(series, "mem/size/used").unwrap_or(0.0) / 1024.0;
        let mut tiles: Vec<(String, String, Vec<f64>, Color32, PerfDev)> = vec![
            ("CPU".into(), format!("{cpu_pct:.0} %"), spark_pts(series, "cpu/load/total"), p.series[0], PerfDev::Cpu),
            (
                tr("Memory").into(),
                format!("{mem_gb:.1} GB ({mem_pct:.0}%)"),
                spark_pts(series, "mem/load/used"),
                p.series[3],
                PerfDev::Memory,
            ),
        ];
        for (i, d) in disks.iter().enumerate() {
            let id = format!("disk{i}/load/active");
            let act = current_val(series, &id).unwrap_or(0.0);
            tiles.push((
                format!("{} {d}", tr("Disk")),
                format!("{act:.0} %"),
                spark_pts(series, &id),
                p.series[2],
                PerfDev::Disk(i),
            ));
        }
        for (i, n) in nets.iter().enumerate() {
            let rx_id = format!("net{i}/rate/rx");
            let rx = current_val(series, &rx_id).unwrap_or(0.0);
            tiles.push((n.clone(), format!("{rx:.1} Mbps"), spark_pts(series, &rx_id), p.series[4], PerfDev::Net(i)));
        }
        for (i, g) in gpus.iter().enumerate() {
            let load = gpu_busy(series, i);
            tiles.push((
                format!("GPU {i} · {}", g.name),
                format!("{load:.0} %"),
                spark_pts_prefix(series, &format!("gpu{i}/")),
                p.series[1],
                PerfDev::Gpu(i),
            ));
        }
        for (i, n) in npus.iter().enumerate() {
            tiles.push((format!("NPU {i}"), n.name.clone(), Vec::new(), p.series[5], PerfDev::Npu(i)));
        }
        let mut pick = None;
        widgets::columns(ui, 3, 220.0, tiles.len(), |ui, idx| {
            let (label, val, sp, col, dev) = &tiles[idx];
            widgets::metric(ui, label, val, sp, *col, Tone::Neutral);
            if ui.small_button(tr("Open details")).clicked() {
                pick = Some(*dev);
            }
        });
        if let Some(d) = pick {
            self.dev = d;
            self.summary_view = false;
        }
    }

    fn render_cpu(&mut self, ui: &mut Ui, ctx: &Arc<Ctx>, series: &[Series], now: f64, procs: &ProcSnap) {
        let p = theme::palette(ui);
        let c = crate::cpu::get(ctx);
        widgets::card(ui, icon::CPU, &c.name, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(tr("% Utilization")).small().color(p.text_secondary));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.selectable_label(self.cpu_graph == CpuGraph::LogicalCores, tr("Logical processors")).clicked()
                    {
                        self.cpu_graph = CpuGraph::LogicalCores;
                    }
                    if ui.selectable_label(self.cpu_graph == CpuGraph::Overall, tr("Overall utilization")).clicked() {
                        self.cpu_graph = CpuGraph::Overall;
                    }
                });
            });
            ui.add_space(4.0);

            match self.cpu_graph {
                CpuGraph::Overall => {
                    let pts = series_points(series, "cpu/load/total");
                    let lines = [Line { label: "CPU", color: p.series[0], points: pts }];
                    plot::chart(ui, &lines, now, self.span.secs(), 190.0, &|v| format!("{v:.1} %"));
                }
                CpuGraph::LogicalCores => {
                    // Per-core heat grid with load %, effective clock and temperature.
                    let n_logical = c.logical.len().max(1);
                    widgets::columns(ui, 4, 130.0, n_logical, |ui, i| {
                        let load = current_val(series, &format!("cpu/load/cpu{i}")).unwrap_or(0.0);
                        let mhz = current_val(series, &format!("cpu/clock/core{i}"));
                        let sp = spark_pts(series, &format!("cpu/load/cpu{i}"));
                        let tone = if load > 85.0 {
                            Tone::Danger
                        } else if load > 50.0 {
                            Tone::Warning
                        } else {
                            Tone::Neutral
                        };
                        let val_str = match mhz {
                            Some(m) => format!("{load:.0}% · {:.2} GHz", m / 1000.0),
                            None => format!("{load:.0}%"),
                        };
                        widgets::metric(ui, &format!("CPU {i}"), &val_str, &sp, p.series[0], tone);
                    });
                }
            }

            ui.add_space(8.0);
            let util = current_val(series, "cpu/load/total").unwrap_or(0.0);
            let mhz = current_val(series, "cpu/clock/avg").unwrap_or(0.0);
            let temp = current_val(series, "cpu/temp/package");
            let power = current_val(series, "cpu/power/package").or_else(|| current_val(series, "cpu/power/ppt"));
            let uptime = format_uptime(cpuzeez_platform::os::uptime_secs());
            let virt = cache::cpu_os(ctx)
                .map(|o| if o.virtualization.firmware_enabled == Some(true) { tr("Enabled") } else { tr("Disabled") })
                .unwrap_or("–");

            let mut kvs = vec![
                (tr("Utilization"), format!("{util:.1} %")),
                (tr("Speed"), if mhz > 0.0 { format!("{:.2} GHz", mhz / 1000.0) } else { "–".into() }),
                (tr("Processes"), procs.totals.processes.to_string()),
                (tr("Threads"), procs.totals.threads.to_string()),
                (tr("Handles"), procs.totals.handles.to_string()),
                (tr("Up time"), uptime),
                (tr("Cores"), c.cores.to_string()),
                (tr("Logical processors"), c.threads.to_string()),
                (tr("Virtualization"), virt.to_string()),
            ];
            if let Some(t) = temp {
                kvs.push((tr("Package temperature"), format!("{t:.1} °C")));
            }
            if let Some(w) = power {
                kvs.push((tr("Package power"), format!("{w:.1} W")));
            }
            let cstates = cpuzeez_platform::cstates::read();
            if let Some(first) = cstates.first().filter(|v| !v.is_empty()) {
                let summary: Vec<String> = first.iter().map(|(n, pct)| format!("{n}: {pct:.0}%")).collect();
                kvs.push((tr("C-state residency (CPU 0)"), summary.join(" · ")));
            }
            widgets::kv_list(ui, &kvs);
        });
    }

    fn render_memory(&mut self, ui: &mut Ui, series: &[Series], now: f64) {
        let p = theme::palette(ui);
        let comp = cpuzeez_platform::memory::composition();
        let total_gb = comp.total.unwrap_or(0.0) / 1024.0;
        let title =
            if total_gb > 0.0 { format!("{} ({total_gb:.1} GB)", tr("Memory")) } else { tr("Memory").to_string() };
        widgets::card(ui, icon::MEMORY, &title, |ui| {
            let pts = series_points(series, "mem/load/used");
            let lines = [Line { label: tr("Memory load"), color: p.series[3], points: pts }];
            plot::chart(ui, &lines, now, self.span.secs(), 180.0, &|v| format!("{v:.1} %"));

            ui.add_space(8.0);
            // Memory composition bar (In use / Modified / Standby-Cached / Free).
            if let Some(total) = comp.total.filter(|&t| t > 0.0) {
                ui.label(RichText::new(tr("Memory composition")).small().color(p.text_secondary));
                let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 22.0), Sense::hover());
                ui.painter().rect_filled(rect, CornerRadius::same(5), p.sidebar);
                let segments = [
                    (comp.used.unwrap_or(0.0), p.series[3], tr("In use")),
                    (comp.modified.unwrap_or(0.0), p.warning, tr("Modified")),
                    (comp.cached.unwrap_or(0.0), p.series[2], tr("Standby / Cached")),
                ];
                let mut x = rect.left();
                for (val, col, _) in segments {
                    let w = (val / total) as f32 * rect.width();
                    if w > 1.0 {
                        let r = egui::Rect::from_min_size(egui::pos2(x, rect.top()), vec2(w, rect.height()));
                        ui.painter().rect_filled(r, CornerRadius::same(2), col);
                        x += w;
                    }
                }
                ui.horizontal_wrapped(|ui| {
                    for (val, col, name) in segments {
                        if val > 0.0 {
                            ui.colored_label(col, format!("■ {name}: {:.1} GB", val / 1024.0));
                        }
                    }
                    if let Some(f) = comp.free {
                        ui.label(format!("□ {}: {:.1} GB", tr("Free"), f / 1024.0));
                    }
                });
                ui.add_space(6.0);
            }

            let fmt_mb = |v: Option<f64>| match v {
                Some(m) if m >= 1024.0 => format!("{:.2} GB", m / 1024.0),
                Some(m) => format!("{m:.0} MB"),
                None => "–".into(),
            };
            let committed = match (comp.commit, comp.commit_limit) {
                (Some(c), Some(l)) => format!("{} / {}", fmt_mb(Some(c)), fmt_mb(Some(l))),
                _ => "–".into(),
            };
            let kvs = vec![
                (tr("In use"), fmt_mb(comp.used)),
                (tr("Available"), fmt_mb(comp.available)),
                (tr("Committed"), committed),
                (tr("Cached / Standby"), fmt_mb(comp.cached)),
                (tr("Compressed"), fmt_mb(comp.compressed)),
                (tr("Paged pool"), fmt_mb(comp.paged_pool)),
                (tr("Non-paged pool / Wired"), fmt_mb(comp.wired)),
                (tr("Swap / Page file used"), fmt_mb(comp.swap_used)),
            ];
            widgets::kv_list(ui, &kvs);
        });
    }

    fn render_disk(&mut self, ui: &mut Ui, ctx: &Arc<Ctx>, series: &[Series], now: f64, i: usize, name: &str) {
        let p = theme::palette(ui);
        let drives = cache::drives(ctx);
        let model = drives.get(i).map(|d| d.name.as_str()).unwrap_or(name);
        widgets::card(ui, icon::HARD_DRIVES, &format!("{name} — {model}"), |ui| {
            let act_pts = series_points(series, &format!("disk{i}/load/active"));
            let lines = [Line { label: tr("Active time"), color: p.series[2], points: act_pts }];
            plot::chart(ui, &lines, now, self.span.secs(), 140.0, &|v| format!("{v:.0} %"));
            ui.add_space(6.0);

            let r_pts = series_points(series, &format!("disk{i}/rate/read"));
            let w_pts = series_points(series, &format!("disk{i}/rate/write"));
            let rw_lines = [
                Line { label: tr("Read"), color: p.series[2], points: r_pts },
                Line { label: tr("Write"), color: p.series[4], points: w_pts },
            ];
            plot::chart(ui, &rw_lines, now, self.span.secs(), 140.0, &|v| format!("{v:.1} MB/s"));
            ui.add_space(8.0);

            let get = |s: &str| current_val(series, &format!("disk{i}/{s}")).unwrap_or(0.0);
            let kvs = vec![
                (tr("Active time"), format!("{:.0} %", get("load/active"))),
                (tr("Read speed"), format!("{:.2} MB/s ({:.0} IOPS)", get("rate/read"), get("iops/read"))),
                (tr("Write speed"), format!("{:.2} MB/s ({:.0} IOPS)", get("rate/write"), get("iops/write"))),
                (tr("Read latency"), format!("{:.2} ms", get("latency/read"))),
                (tr("Write latency"), format!("{:.2} ms", get("latency/write"))),
                (tr("Queue depth"), format!("{:.1}", get("queue/depth"))),
            ];
            widgets::kv_list(ui, &kvs);
        });
    }

    fn render_net(&mut self, ui: &mut Ui, ctx: &Arc<Ctx>, series: &[Series], now: f64, i: usize, name: &str) {
        let p = theme::palette(ui);
        let adapters = crate::network::adapters(ctx);
        let meta = adapters.iter().find(|a| a.name == name);
        let title = match meta {
            Some(a) if !a.description.is_empty() => format!("{name} — {}", a.description),
            _ => name.to_string(),
        };
        widgets::card(ui, icon::GLOBE, &title, |ui| {
            let rx_pts = series_points(series, &format!("net{i}/rate/rx"));
            let tx_pts = series_points(series, &format!("net{i}/rate/tx"));
            let lines = [
                Line { label: tr("Receive"), color: p.series[4], points: rx_pts },
                Line { label: tr("Send"), color: p.series[0], points: tx_pts },
            ];
            plot::chart(ui, &lines, now, self.span.secs(), 180.0, &|v| format!("{v:.2} Mbps"));
            ui.add_space(8.0);

            let get = |s: &str| current_val(series, &format!("net{i}/{s}")).unwrap_or(0.0);
            let mut kvs = vec![
                (tr("Receive"), format!("{:.2} Mbps ({:.0} pkt/s)", get("rate/rx"), get("pps/rx"))),
                (tr("Send"), format!("{:.2} Mbps ({:.0} pkt/s)", get("rate/tx"), get("pps/tx"))),
                (tr("Errors / s"), format!("{:.1}", get("errors/total"))),
                (tr("Drops / s"), format!("{:.1}", get("drops/total"))),
            ];
            if let Some(a) = meta {
                kvs.push((tr("Connection type"), a.kind.clone()));
                if let Some(mbps) = a.link_mbps {
                    kvs.push((tr("Link speed"), crate::network::speed_text(mbps)));
                }
                if let Some(ip) = a.ipv4.first() {
                    kvs.push((tr("IPv4 address"), ip.clone()));
                }
                if let Some(ssid) = &a.wifi_ssid {
                    kvs.push(("SSID", ssid.clone()));
                }
            }
            widgets::kv_list(ui, &kvs);
        });
    }

    fn render_gpu(&mut self, ui: &mut Ui, ctx: &Arc<Ctx>, series: &[Series], now: f64, i: usize) {
        let p = theme::palette(ui);
        let adapters = crate::gpu::adapters(ctx);
        let name = adapters.get(i).map(|a| a.name.as_str()).unwrap_or("GPU");
        widgets::card(ui, icon::GRAPHICS_CARD, &format!("GPU {i} — {name}"), |ui| {
            let prefix = format!("gpu{i}/engine/");
            let engines: Vec<&Series> = series.iter().filter(|s| s.id.starts_with(&prefix)).collect();
            if !engines.is_empty() {
                widgets::columns(ui, 2, 220.0, engines.len().min(6), |ui, idx| {
                    let e = engines[idx];
                    ui.label(RichText::new(format!("{} ({:.0}%)", e.label, e.current)).small().color(p.text_secondary));
                    let lines = [Line { label: &e.label, color: p.series[1], points: &e.points }];
                    plot::chart(ui, &lines, now, self.span.secs(), 110.0, &|v| format!("{v:.0} %"));
                });
            } else {
                let pts = series_points(series, &format!("gpu{i}/load/core"));
                let lines = [Line { label: "GPU", color: p.series[1], points: pts }];
                plot::chart(ui, &lines, now, self.span.secs(), 160.0, &|v| format!("{v:.0} %"));
            }

            ui.add_space(8.0);
            let get = |s: &str| current_val(series, &format!("gpu{i}/{s}"));
            let mut kvs = vec![(tr("Utilization"), format!("{:.0} %", gpu_busy(series, i)))];
            if let Some(t) = get("temp/core") {
                kvs.push((tr("GPU temperature"), format!("{t:.0} °C")));
            }
            if let Some(h) = get("temp/hotspot") {
                kvs.push((tr("Hot spot"), format!("{h:.0} °C")));
            }
            if let Some(c) = get("clock/core") {
                kvs.push((tr("Core clock"), format!("{c:.0} MHz")));
            }
            if let Some(w) = get("power/board") {
                kvs.push((tr("Board power"), format!("{w:.1} W")));
            }
            if let Some(a) = adapters.get(i) {
                if let Some(vram) = a.mem_size_mb {
                    kvs.push((tr("Dedicated GPU memory"), format!("{:.1} GB", vram as f64 / 1024.0)));
                }
                if let Some(drv) = a.driver_version.as_deref().filter(|s| !s.is_empty()) {
                    kvs.push((tr("Driver version"), drv.to_string()));
                }
            }
            widgets::kv_list(ui, &kvs);
        });
    }

    fn render_npu(&mut self, ui: &mut Ui, npus: &[crate::npu::Npu], i: usize) {
        let Some(n) = npus.get(i) else {
            widgets::empty_state(ui, icon::CIRCUITRY, "NPU", tr("No NPU detected on this machine."));
            return;
        };
        widgets::card(ui, icon::CIRCUITRY, &format!("NPU {i} — {}", n.name), |ui| {
            let mut kvs = vec![(tr("Device"), n.name.clone())];
            if let Some(c) = n.cores {
                kvs.push((tr("Cores"), c.to_string()));
            }
            if let Some(m) = n.mem_mb {
                kvs.push((tr("Dedicated memory"), format!("{m} MB")));
            }
            if let Some(d) = &n.driver_version {
                kvs.push((tr("Driver version"), d.clone()));
            }
            widgets::kv_list(ui, &kvs);
        });
    }
}

fn dev_tile(ui: &mut Ui, ic: &str, title: &str, sub: &str, spark: &[f64], color: Color32, selected: bool) -> bool {
    let p = theme::palette(ui);
    let fill = if selected { p.accent_soft } else { p.card };
    let stroke = if selected { Stroke::new(1.5, p.accent) } else { Stroke::new(1.0, p.border) };
    let resp = Frame::default()
        .fill(fill)
        .stroke(stroke)
        .corner_radius(CornerRadius::same(theme::RADIUS_CARD))
        .inner_margin(Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(RichText::new(ic).color(if selected { p.accent } else { p.text_secondary }));
                ui.label(RichText::new(title).strong().color(p.text));
            });
            ui.label(RichText::new(sub).small().color(p.text_secondary));
            if spark.len() >= 2 {
                let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 18.0), Sense::hover());
                widgets::sparkline(ui.painter(), rect, spark, color);
            }
        })
        .response
        .interact(Sense::click());
    ui.add_space(6.0);
    resp.clicked()
}

fn current_val(series: &[Series], id: &str) -> Option<f64> {
    series.iter().find(|s| s.id == id).map(|s| s.current)
}

fn series_points<'a>(series: &'a [Series], id: &str) -> &'a [(f64, f64)] {
    series.iter().find(|s| s.id == id).map_or(&[], |s| s.points.as_slice())
}

fn spark_pts(series: &[Series], id: &str) -> Vec<f64> {
    series_points(series, id).iter().rev().take(60).rev().map(|p| p.1).collect()
}

fn spark_pts_prefix(series: &[Series], prefix: &str) -> Vec<f64> {
    series
        .iter()
        .find(|s| s.id.starts_with(prefix) && (s.id.contains("/load/") || s.id.contains("/engine/")))
        .map(|s| s.points.iter().rev().take(60).rev().map(|p| p.1).collect())
        .unwrap_or_default()
}

fn gpu_busy(series: &[Series], i: usize) -> f64 {
    let eng_prefix = format!("gpu{i}/engine/");
    let max_eng = series.iter().filter(|s| s.id.starts_with(&eng_prefix)).map(|s| s.current).fold(0.0, f64::max);
    current_val(series, &format!("gpu{i}/load/core")).unwrap_or(max_eng)
}

fn groups_for(series: &[Series], prefix: &str) -> Vec<String> {
    let mut out = Vec::new();
    for s in series {
        if let Some(rest) = s.group.strip_prefix(prefix)
            && !out.iter().any(|x| x == rest)
        {
            out.push(rest.to_string());
        }
    }
    out
}

pub fn format_uptime(secs: u64) -> String {
    let d = secs / 86_400;
    let h = (secs % 86_400) / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    format!("{d}:{h:02}:{m:02}:{s:02}")
}

pub fn export_series_csv(series: &[Series]) -> String {
    let mut out = String::from("id,group,label,current,min,max,avg\n");
    for s in series {
        out.push_str(&format!(
            "{},\"{}\",\"{}\",{:.2},{:.2},{:.2},{:.2}\n",
            s.id, s.group, s.label, s.current, s.min, s.max, s.avg
        ));
    }
    out
}

pub const PT_BR: &[(&str, &str)] = &[
    ("Summary view", "Visão resumida"),
    ("Window:", "Janela:"),
    ("1m", "1m"),
    ("10m", "10m"),
    ("1h", "1h"),
    ("Session", "Sessão"),
    ("Open details", "Abrir detalhes"),
    ("% Utilization", "% de utilização"),
    ("Logical processors", "Processadores lógicos"),
    ("Overall utilization", "Utilização geral"),
    ("Utilization", "Utilização"),
    ("Speed", "Velocidade"),
    ("Up time", "Tempo de atividade"),
    ("Virtualization", "Virtualização"),
    ("Enabled", "Ativada"),
    ("Disabled", "Desativada"),
    ("Package temperature", "Temperatura do pacote"),
    ("Package power", "Potência do pacote"),
    ("C-state residency (CPU 0)", "Residência C-state (CPU 0)"),
    ("Memory load", "Carga de memória"),
    ("Memory composition", "Composição da memória"),
    ("In use", "Em uso"),
    ("Modified", "Modificada"),
    ("Standby / Cached", "Em espera / Cache"),
    ("Free", "Livre"),
    ("Available", "Disponível"),
    ("Committed", "Confirmada"),
    ("Cached / Standby", "Em cache / Espera"),
    ("Compressed", "Comprimida"),
    ("Paged pool", "Pool paginado"),
    ("Non-paged pool / Wired", "Pool não paginado / Wired"),
    ("Swap / Page file used", "Swap / Arquivo de paginação usado"),
    ("Active time", "Tempo de atividade"),
    ("Read", "Leitura"),
    ("Write", "Gravação"),
    ("Read speed", "Velocidade de leitura"),
    ("Write speed", "Velocidade de gravação"),
    ("Read latency", "Latência de leitura"),
    ("Write latency", "Latência de gravação"),
    ("Queue depth", "Profundidade da fila"),
    ("Receive", "Receber"),
    ("Send", "Enviar"),
    ("Errors / s", "Erros / s"),
    ("Drops / s", "Descartes / s"),
    ("Connection type", "Tipo de conexão"),
    ("Link speed", "Velocidade do link"),
    ("IPv4 address", "Endereço IPv4"),
    ("GPU temperature", "Temperatura da GPU"),
    ("Board power", "Potência da placa"),
    ("Dedicated GPU memory", "Memória dedicada da GPU"),
    ("Driver version", "Versão do driver"),
    ("No NPU detected on this machine.", "Nenhuma NPU detectada nesta máquina."),
    ("Device", "Dispositivo"),
    ("Dedicated memory", "Memória dedicada"),
];

pub const ES: &[(&str, &str)] = &[
    ("Summary view", "Vista resumida"),
    ("Window:", "Ventana:"),
    ("1m", "1m"),
    ("10m", "10m"),
    ("1h", "1h"),
    ("Session", "Sesión"),
    ("Open details", "Abrir detalles"),
    ("% Utilization", "% de utilización"),
    ("Logical processors", "Procesadores lógicos"),
    ("Overall utilization", "Utilización general"),
    ("Utilization", "Utilización"),
    ("Speed", "Velocidad"),
    ("Up time", "Tiempo activo"),
    ("Virtualization", "Virtualización"),
    ("Enabled", "Habilitada"),
    ("Disabled", "Deshabilitada"),
    ("Package temperature", "Temperatura del paquete"),
    ("Package power", "Potencia del paquete"),
    ("C-state residency (CPU 0)", "Residencia C-state (CPU 0)"),
    ("Memory load", "Carga de memoria"),
    ("Memory composition", "Composición de la memoria"),
    ("In use", "En uso"),
    ("Modified", "Modificada"),
    ("Standby / Cached", "En espera / Caché"),
    ("Free", "Libre"),
    ("Available", "Disponible"),
    ("Committed", "Confirmada"),
    ("Cached / Standby", "En caché / Espera"),
    ("Compressed", "Comprimida"),
    ("Paged pool", "Bloque paginado"),
    ("Non-paged pool / Wired", "Bloque no paginado / Wired"),
    ("Swap / Page file used", "Swap / Archivo de paginación usado"),
    ("Active time", "Tiempo de actividad"),
    ("Read", "Lectura"),
    ("Write", "Escritura"),
    ("Read speed", "Velocidad de lectura"),
    ("Write speed", "Velocidad de escritura"),
    ("Read latency", "Latencia de lectura"),
    ("Write latency", "Latencia de escritura"),
    ("Queue depth", "Profundidad de cola"),
    ("Receive", "Recibir"),
    ("Send", "Enviar"),
    ("Errors / s", "Errores / s"),
    ("Drops / s", "Descartes / s"),
    ("Connection type", "Tipo de conexión"),
    ("Link speed", "Velocidad de enlace"),
    ("IPv4 address", "Dirección IPv4"),
    ("GPU temperature", "Temperatura de GPU"),
    ("Board power", "Potencia de la placa"),
    ("Dedicated GPU memory", "Memoria de GPU dedicada"),
    ("Driver version", "Versión del controlador"),
    ("No NPU detected on this machine.", "No se detectó ninguna NPU en este equipo."),
    ("Device", "Dispositivo"),
    ("Dedicated memory", "Memoria dedicada"),
];

#[cfg(test)]
mod tests {
    use super::*;
    use cpuzeez_core::metrics::Unit;

    #[test]
    fn uptime_and_csv_export() {
        assert_eq!(format_uptime(90_061), "1:01:01:01");
        let s = vec![Series {
            id: "cpu/load/total".into(),
            group: "CPU".into(),
            label: "Total load".into(),
            unit: Unit::Percent,
            current: 25.0,
            min: 10.0,
            max: 50.0,
            avg: 25.0,
            points: vec![(0.0, 10.0), (1.0, 25.0)],
        }];
        let csv = export_series_csv(&s);
        assert!(csv.contains("cpu/load/total") && csv.contains("25.00"));
    }

    #[test]
    fn performance_tab_switches_devices_and_summary() {
        use egui_kittest::{Harness, kittest::Queryable};
        let ctx = Arc::new(Ctx::new(Default::default()));
        let s = vec![
            Series {
                id: "cpu/load/total".into(),
                group: "CPU".into(),
                label: "Total load".into(),
                unit: Unit::Percent,
                current: 33.0,
                min: 10.0,
                max: 60.0,
                avg: 30.0,
                points: vec![(0.0, 20.0), (1.0, 33.0)],
            },
            Series {
                id: "mem/load/used".into(),
                group: "Memory".into(),
                label: "Memory load".into(),
                unit: Unit::Percent,
                current: 55.0,
                min: 50.0,
                max: 60.0,
                avg: 55.0,
                points: vec![(0.0, 50.0), (1.0, 55.0)],
            },
        ];
        let procs = ProcSnap::default();
        let mut h = Harness::builder().with_size(egui::vec2(1080.0, 680.0)).build_ui_state(
            move |ui, st: &mut State| {
                if !ui.style().text_styles.contains_key(&egui::TextStyle::Name("title".into())) {
                    super::super::theme::install(ui.ctx());
                    ui.ctx().request_repaint();
                    return;
                }
                st.show_with_series(ui, &ctx, &s, 1.0, &procs);
            },
            State::default(),
        );
        h.run();
        h.run();
        assert_eq!(h.state().dev, PerfDev::Cpu);
        h.get_by_label_contains("Summary view").click();
        h.run();
        assert!(h.state().summary_view);
    }
}
