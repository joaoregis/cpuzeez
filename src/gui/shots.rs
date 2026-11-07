//! F5/F6/F7 captures and the `-screenshots` automation.

#[allow(unused_imports)]
use crate::gui::*;

impl App {
    /// Put the UI in the state `step` captures; true when it is ready.
    pub(crate) fn stage_shot(
        &mut self,
        step: Step,
        first: bool,
        report: Option<&[Section]>,
        ectx: &egui::Context,
    ) -> bool {
        let idx = self.auto.as_ref().map_or(0, |a| a.step);
        // The last capture shows the light theme.
        ectx.set_theme(if SHOTS.get(idx).is_some_and(|s| s.0.ends_with("-light")) {
            egui::ThemePreference::Light
        } else {
            egui::ThemePreference::Dark
        });
        if !matches!(step, Step::Bench(BenchView::Stress)) && self.mon.stress.running() {
            self.mon.stress.stop(None); // don't load the CPU during later captures
        }
        match step {
            Step::Page(pg) => {
                self.page = pg;
                match pg {
                    Page::Devices => self.devices.loaded(&self.ctx),
                    // Graphs need some history.
                    Page::Sensors | Page::Overview => {
                        self.auto.as_ref().is_some_and(|a| a.started.elapsed() > Duration::from_secs(9))
                    }
                    _ => true,
                }
            }
            Step::Bench(v) => {
                self.page = Page::Benchmarks;
                self.bench_view = v;
                match v {
                    BenchView::Cpu => self.bench.autorun(&self.ctx),
                    BenchView::Memory => self.bench.autorun_memory(&self.ctx),
                    BenchView::Latency | BenchView::Gpu | BenchView::Storage => true,
                    BenchView::Stress => {
                        if first && !self.mon.stress.running() {
                            self.mon.start_stress(crate::os::logical_cpus());
                        }
                        self.mon.stress.elapsed_secs() >= 8.0
                    }
                    BenchView::History => {
                        if first {
                            // This session's results only: nothing from the user's history.json.
                            self.history.preview(self.bench.session_entries(&self.ctx));
                            self.history_dirty = false;
                        }
                        true
                    }
                }
            }
            Step::Tool(v) => {
                self.page = Page::Tools;
                self.tool_view = v;
                if v == ToolView::Compare && first {
                    // Masked (serials, MAC/IP addresses and raw dumps removed)
                    // and into this run's own output dir — never a fixed name
                    // in shared /tmp, where every local user could read the
                    // machine's full unmasked inventory, and where a planted
                    // symlink would redirect the write. The README promises
                    // masking for everything this mode produces.
                    let masked = report.map(report::masked);
                    let dir = self.auto.as_ref().map_or_else(|| std::env::temp_dir(), |a| a.dir.clone());
                    let path = dir.join(format!("{}-screenshot-report.json", crate::NAME));
                    let _ = std::fs::write(&path, masked.as_deref().map(report::to_json).unwrap_or_default());
                    self.compare.current = true;
                    self.compare.b = path.display().to_string();
                    self.compare.run(masked.as_deref());
                    // Published image: no user name from the path.
                    self.compare.b = path.file_name().map_or(String::new(), |n| n.to_string_lossy().into());
                }
                true
            }
            Step::System(tab) => {
                self.page = Page::System;
                self.system.tab = tab;
                if first && tab == system::SysTab::Services {
                    // Show the inspector on a service every OS has.
                    let name = if cfg!(windows) {
                        "EventLog"
                    } else if cfg!(target_os = "macos") {
                        "com.apple.syslogd"
                    } else {
                        "systemd-journald.service"
                    };
                    self.system.services.selected_name = Some(name.into());
                }
                // Let the sampler and the background queries (services, startup) answer.
                !first && self.auto.as_ref().is_some_and(|a| a.since.elapsed() > Duration::from_secs(3))
            }
            Step::Bottleneck => {
                self.page = Page::Live;
                self.live.tab = live::Tab::Bottleneck;
                if first {
                    // A real capture (PresentMon 2.6 on a Ryzen 9 7900X + RTX 4060, Deadlock).
                    let csv = include_str!("../../crates/core/tests/fixtures/presentmon-2.6-deadlock.csv");
                    self.live.detector = bottleneck_ui::Detector::from_capture(csv, "deadlock.exe");
                }
                true
            }
            Step::AboutPt => {
                i18n::set(i18n::Lang::PtBr);
                self.page = Page::About;
                true
            }
        }
    }

    pub(crate) fn auto_shots(&mut self, ectx: &egui::Context, report: Option<&[Section]>) {
        let Some(a) = &self.auto else { return };
        ectx.request_repaint();
        self.mon.tel().want("sensors", &[""]); // sample early so the Sensors shot has history
        // Let the report load and the clock sampler produce a value first.
        if report.is_none() || a.started.elapsed() < Duration::from_secs(3) || self.shot.is_some() {
            return;
        }
        let idx = a.step;
        let Some(&(name, step)) = SHOTS.get(idx) else {
            self.mon.stress.stop(None);
            ectx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        };
        let first = a.prepared != Some(idx);
        let ready = self.stage_shot(step, first, report, ectx);
        let Some(a) = &mut self.auto else { return };
        if first {
            a.prepared = Some(idx);
            a.since = std::time::Instant::now();
        }
        if !ready {
            return;
        }
        a.wait += 1;
        // A few frames so the page is laid out, and time for windows to fade in.
        if a.wait < 4 || a.since.elapsed() < Duration::from_millis(600) {
            return;
        }
        a.wait = 0;
        a.step += 1;
        self.shot = Some(Shot::Png(a.dir.join(format!("{name}.png"))));
        ectx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
    }

    pub(crate) fn take_screenshot(&mut self, ectx: &egui::Context) {
        if self.shot.is_none() {
            return;
        }
        let img = ectx.input(|i| {
            i.raw.events.iter().find_map(|e| match e {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        let Some(img) = img else { return };
        match self.shot.take() {
            Some(Shot::Clipboard) => {
                ectx.copy_image((*img).clone());
                self.status = tr("Tab copied to clipboard").into();
            }
            Some(Shot::Bmp) => {
                let name = format!("{}-{}.bmp", crate::NAME, page_name(self.page).to_lowercase());
                self.status = match std::fs::write(&name, bmp::encode(&img)) {
                    Ok(()) => trf("Saved {}", &[&name]),
                    Err(e) => format!("{name}: {e}"),
                };
            }
            Some(Shot::Png(path)) => {
                if let Err(e) = write_png(&path, &img) {
                    eprintln!("{}: {e}", path.display());
                } else {
                    eprintln!("wrote {}", path.display());
                }
            }
            None => {}
        }
    }
}

pub(crate) fn write_png(path: &std::path::Path, img: &egui::ColorImage) -> Result<(), Box<dyn std::error::Error>> {
    let f = std::io::BufWriter::new(std::fs::File::create(path)?);
    let mut enc = png::Encoder::new(f, img.size[0] as u32, img.size[1] as u32);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    let data: Vec<u8> = img.pixels.iter().flat_map(|p| p.to_array()).collect();
    enc.write_header()?.write_image_data(&data)?;
    Ok(())
}
