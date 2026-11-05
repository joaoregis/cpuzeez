//! GPU benchmark (S1-K05) on wgpu (DX12 / Vulkan / Metal), which the app
//! already ships for its renderer fallback (ADR 0013), so no new dependency:
//! - FP32 and FP16 throughput: chains of vec4 fused multiply-adds in a compute
//!   shader (FP16 only where the GPU and backend expose `shader-f16`);
//! - VRAM bandwidth: a 256 MiB buffer-to-buffer copy (read + write counted);
//! - PCIe host ↔ GPU: uploading and reading back 256 MiB (shows the real link);
//! - a raster scene: 65 536 instanced, shaded triangles at 1920 × 1080.
//!
//! Wall-clock timed around `queue.submit` + a blocking poll, after a warm-up.
//! Our own workloads: not comparable with other tools.

use crate::benchkit::{Ctl, Sub};
use eframe::egui_wgpu::wgpu;
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Native wgpu futures resolve without an executor; poll until ready.
fn block_on<F: std::future::Future>(f: F) -> F::Output {
    let mut f = std::pin::pin!(f);
    let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
    loop {
        if let std::task::Poll::Ready(v) = f.as_mut().poll(&mut cx) {
            return v;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}

/// Hardware adapters, one per GPU and backend (software renderers and
/// duplicates left out).
fn list() -> Vec<wgpu::Adapter> {
    let instance = wgpu::Instance::default();
    let mut seen = Vec::new();
    block_on(instance.enumerate_adapters(wgpu::Backends::PRIMARY))
        .into_iter()
        .filter(|a| {
            let i = a.get_info();
            let key = (i.name.clone(), i.backend);
            i.device_type != wgpu::DeviceType::Cpu && !seen.contains(&key) && {
                seen.push(key);
                true
            }
        })
        .collect()
}

/// "NVIDIA GeForce RTX 4060 (Vulkan)" per adapter.
pub fn adapters() -> Vec<String> {
    list().iter().map(|a| a.get_info()).map(|i| format!("{} ({:?})", i.name, i.backend)).collect()
}

struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
    f16: bool,
}

impl Gpu {
    fn open(index: usize) -> Result<(Gpu, String), String> {
        let adapter = list().into_iter().nth(index).ok_or("no GPU adapter (DX12, Vulkan or Metal)")?;
        let info = adapter.get_info();
        let f16 = adapter.features().contains(wgpu::Features::SHADER_F16);
        let (device, queue) = block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("cpuzeez-bench"),
            required_features: if f16 { wgpu::Features::SHADER_F16 } else { wgpu::Features::empty() },
            required_limits: adapter.limits(),
            ..Default::default()
        }))
        .map_err(|e| e.to_string())?;
        Ok((Gpu { device, queue, f16 }, format!("{} ({:?})", info.name, info.backend)))
    }

    /// Submits and blocks until the GPU is done; seconds taken.
    fn run(&self, enc: wgpu::CommandEncoder) -> f64 {
        let t = Instant::now();
        self.queue.submit([enc.finish()]);
        let _ = self.device.poll(wgpu::PollType::wait_indefinitely());
        t.elapsed().as_secs_f64()
    }

    fn encoder(&self) -> wgpu::CommandEncoder {
        self.device.create_command_encoder(&Default::default())
    }

    fn buffer(&self, size: u64, usage: wgpu::BufferUsages) -> wgpu::Buffer {
        self.device.create_buffer(&wgpu::BufferDescriptor { label: None, size, usage, mapped_at_creation: false })
    }
}

const FMA: &str = r#"
@group(0) @binding(0) var<storage, read_write> out: array<vec4<T>>;
@group(0) @binding(1) var<uniform> iters: vec4<u32>;
@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    var a = vec4<T>(T(id.x % 7u), T(1), T(2), T(3)) * T(0.01);
    var b = a + vec4<T>(T(0.5));
    var c = a * T(0.25);
    var d = b * T(0.75);
    var e = a + vec4<T>(T(0.125));
    var f = b * T(0.5);
    var g = c + vec4<T>(T(0.25));
    var h = d * T(0.125);
    let m = vec4<T>(T(0.999));
    let k = vec4<T>(T(0.001));
    for (var i = 0u; i < iters.x; i++) {
        a = fma(a, m, k);
        b = fma(b, m, k);
        c = fma(c, m, k);
        d = fma(d, m, k);
        e = fma(e, m, k);
        f = fma(f, m, k);
        g = fma(g, m, k);
        h = fma(h, m, k);
    }
    out[id.x] = a + b + c + d + e + f + g + h;
}
"#;
const GROUPS: u32 = 4096;
/// 8 independent vec4 FMAs per iteration = 64 FLOPs per invocation.
const FLOPS_PER_ITER: f64 = 64.0;

/// TFLOPS of the FMA shader with `ty` = "f32" or "f16".
fn flops(g: &Gpu, ty: &str, ctl: &Ctl) -> Result<f64, String> {
    let src = if ty == "f16" { format!("enable f16;\n{}", FMA.replace('T', "f16")) } else { FMA.replace('T', "f32") };
    let module = g.device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("fma"),
        source: wgpu::ShaderSource::Wgsl(src.into()),
    });
    let pipeline = g.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("fma"),
        layout: None,
        module: &module,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    });
    let invocations = GROUPS as u64 * 256;
    let out = g.buffer(invocations * 16, wgpu::BufferUsages::STORAGE);
    let uni = g.buffer(16, wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST);
    let bg = g.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: out.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 1, resource: uni.as_entire_binding() },
        ],
    });
    let dispatch = |iters: u32| {
        g.queue.write_buffer(&uni, 0, &[iters, 0, 0, 0].map(u32::to_le_bytes).concat());
        let mut enc = g.encoder();
        {
            let mut pass = enc.begin_compute_pass(&Default::default());
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bg, &[]);
            pass.dispatch_workgroups(GROUPS, 1, 1);
        }
        g.run(enc)
    };
    // Grow the work until one dispatch takes ~25 ms (TDR-safe on slow GPUs).
    let mut iters = 32u32;
    while dispatch(iters) < 0.025 && iters < 1 << 16 {
        ctl.check()?;
        iters *= 2;
    }
    let (mut t, mut work) = (0.0, 0.0);
    while t < 1.0 {
        ctl.check()?;
        t += dispatch(iters);
        work += invocations as f64 * iters as f64 * FLOPS_PER_ITER;
    }
    Ok(work / t / 1e12)
}

const BIG: u64 = 256 << 20;

/// VRAM GB/s: copy BIG bytes inside the GPU (read + write).
fn vram(g: &Gpu, ctl: &Ctl) -> Result<f64, String> {
    let a = g.buffer(BIG, wgpu::BufferUsages::COPY_SRC);
    let b = g.buffer(BIG, wgpu::BufferUsages::COPY_DST);
    let copy = || {
        let mut enc = g.encoder();
        for _ in 0..4 {
            enc.copy_buffer_to_buffer(&a, 0, &b, 0, BIG);
        }
        g.run(enc)
    };
    copy();
    let (mut t, mut n) = (0.0, 0.0);
    while t < 1.0 {
        ctl.check()?;
        t += copy();
        n += 4.0 * 2.0 * BIG as f64;
    }
    Ok(n / t / 1e9)
}

/// (upload, download) GB/s over the bus.
fn pcie(g: &Gpu, ctl: &Ctl) -> Result<(f64, f64), String> {
    // Upload: the CPU writes a mapped staging buffer, then the GPU copies it
    // into a GPU-only buffer. Where the staging lives is the driver's choice
    // (system RAM, or VRAM through ReBAR on Vulkan), so the bus is whichever
    // stage is slower: timing only the copy read 126 GB/s on a PCIe 4.0 x8 card.
    let host = vec![7u8; BIG as usize];
    let staging = g.buffer(BIG, wgpu::BufferUsages::MAP_WRITE | wgpu::BufferUsages::COPY_SRC);
    let dev = g.buffer(BIG, wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC);
    let read = g.buffer(BIG, wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ);
    let map = |b: &wgpu::Buffer, mode: wgpu::MapMode| -> Result<(), String> {
        let done: Arc<Mutex<Option<Result<(), wgpu::BufferAsyncError>>>> = Arc::default();
        let d = done.clone();
        b.slice(..).map_async(mode, move |r| *d.lock().unwrap_or_else(|e| e.into_inner()) = Some(r));
        let _ = g.device.poll(wgpu::PollType::wait_indefinitely());
        let r = done.lock().unwrap_or_else(|e| e.into_inner()).take();
        r.ok_or("mapping never finished")?.map_err(|e| e.to_string())
    };
    let up = || -> Result<f64, String> {
        map(&staging, wgpu::MapMode::Write)?;
        let t = Instant::now();
        staging.slice(..).get_mapped_range_mut().map_err(|e| e.to_string())?.copy_from_slice(&host);
        staging.unmap();
        let write = t.elapsed().as_secs_f64();
        let mut enc = g.encoder();
        enc.copy_buffer_to_buffer(&staging, 0, &dev, 0, BIG);
        Ok(write.max(g.run(enc)))
    };
    let down = || -> Result<f64, String> {
        let t = Instant::now();
        let mut enc = g.encoder();
        enc.copy_buffer_to_buffer(&dev, 0, &read, 0, BIG);
        g.queue.submit([enc.finish()]);
        map(&read, wgpu::MapMode::Read)?;
        let el = t.elapsed().as_secs_f64();
        if let Ok(v) = read.slice(..).get_mapped_range() {
            std::hint::black_box(v[0]);
        }
        read.unmap();
        Ok(el)
    };
    up()?;
    let mut best_up = f64::MAX;
    for _ in 0..4 {
        ctl.check()?;
        best_up = best_up.min(up()?);
    }
    down()?;
    let mut best_down = f64::MAX;
    for _ in 0..4 {
        ctl.check()?;
        best_down = best_down.min(down()?);
    }
    Ok((BIG as f64 / best_up / 1e9, BIG as f64 / best_down / 1e9))
}

const SCENE: &str = r#"
struct V { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32> }
@vertex
fn vs(@builtin(vertex_index) v: u32, @builtin(instance_index) i: u32) -> V {
    let cell = vec2<f32>(f32(i % 256u), f32(i / 256u)) / 128.0 - 1.0;
    var c = vec2<f32>(0.0, 0.0);
    if v == 1u { c = vec2<f32>(3.0, 0.0); }
    if v == 2u { c = vec2<f32>(0.0, 3.0); }
    var o: V;
    o.pos = vec4<f32>(cell + c / 128.0, f32(i % 97u) / 97.0, 1.0);
    o.uv = c;
    return o;
}
@fragment
fn fs(in: V) -> @location(0) vec4<f32> {
    var x = in.uv;
    for (var k = 0; k < 8; k++) {
        x = vec2<f32>(sin(x.x * 1.7 + x.y), cos(x.y * 1.3 - x.x));
    }
    return vec4<f32>(x * 0.5 + 0.5, 0.5, 1.0);
}
"#;

/// Frames per second of the raster scene at 1920 × 1080.
fn raster(g: &Gpu, ctl: &Ctl) -> Result<f64, String> {
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let tex = g.device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d { width: 1920, height: 1080, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = tex.create_view(&Default::default());
    let module = g.device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("scene"),
        source: wgpu::ShaderSource::Wgsl(SCENE.into()),
    });
    let pipeline = g.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("scene"),
        layout: None,
        vertex: wgpu::VertexState {
            module: &module,
            entry_point: Some("vs"),
            compilation_options: Default::default(),
            buffers: &[],
        },
        primitive: Default::default(),
        depth_stencil: None,
        multisample: Default::default(),
        fragment: Some(wgpu::FragmentState {
            module: &module,
            entry_point: Some("fs"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    });
    let frames = |n: u32| {
        let mut enc = g.encoder();
        for _ in 0..n {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            pass.set_pipeline(&pipeline);
            pass.draw(0..3, 0..65_536);
        }
        g.run(enc)
    };
    frames(2);
    let (mut t, mut n) = (0.0, 0.0);
    while t < 1.0 {
        ctl.check()?;
        t += frames(10);
        n += 10.0;
    }
    Ok(n / t)
}

/// The reference machine's RTX 4060 (PCIe 4.0 x8, Vulkan): 1000 points each.
const REFERENCE: &[(&str, f64)] = &[
    ("FP32", 10.48),
    ("FP16", 13.26),
    ("VRAM bandwidth", 256.0),
    ("Upload (host → GPU)", 11.36),
    ("Download (GPU → host)", 13.12),
    ("Raster scene 1080p", 3708.5),
];

/// Every test on adapter `index`; `notes` gets what couldn't run.
pub fn run(index: usize, ctl: &Ctl, notes: &mut Vec<String>) -> Result<(Vec<Sub>, String), String> {
    let (g, name) = Gpu::open(index)?;
    let mut out = Vec::new();
    ctl.set("FP32", 0.0);
    out.push(Sub::new("FP32", flops(&g, "f32", ctl)?, "TFLOPS", true));
    ctl.set("FP16", 0.2);
    if g.f16 {
        out.push(Sub::new("FP16", flops(&g, "f16", ctl)?, "TFLOPS", true));
    } else {
        notes.push("FP16: this GPU or backend doesn't expose shader-f16".into());
    }
    ctl.set("VRAM", 0.4);
    out.push(Sub::new("VRAM bandwidth", vram(&g, ctl)?, "GB/s", true));
    ctl.set("PCIe", 0.6);
    let (up, down) = pcie(&g, ctl)?;
    out.push(Sub::new("Upload (host → GPU)", up, "GB/s", true));
    out.push(Sub::new("Download (GPU → host)", down, "GB/s", true));
    ctl.set("Raster", 0.8);
    out.push(Sub::new("Raster scene 1080p", raster(&g, ctl)?, "fps", true));
    let out = out
        .into_iter()
        .map(|s| match REFERENCE.iter().find(|r| r.0 == s.name) {
            Some(r) => s.scored(r.1),
            None => s,
        })
        .collect();
    Ok((out, name))
}

/// GPU stress (S1-K07): the FP32 shader in a loop until cancelled.
pub fn stress(index: usize, ctl: &Ctl) -> Result<(), String> {
    let (g, _) = Gpu::open(index)?;
    while !ctl.cancelled() {
        flops(&g, "f32", ctl).or_else(|e| if ctl.cancelled() { Ok(0.0) } else { Err(e) })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn runs_where_a_gpu_exists() {
        // CI runners may have no adapter at all; that's an answer too.
        let names = super::adapters();
        if names.is_empty() {
            return;
        }
        let mut notes = Vec::new();
        let ctl = crate::benchkit::Ctl::default();
        let (subs, name) = super::run(0, &ctl, &mut notes).expect("benchmark ran");
        assert!(subs.iter().all(|s| s.value > 0.0), "{name}: {subs:?}");
    }

    #[test]
    fn gpu_stress_stops_when_cancelled() {
        if super::adapters().is_empty() {
            return;
        }
        let j = crate::benchkit::Job::spawn(|ctl| super::stress(0, ctl));
        std::thread::sleep(std::time::Duration::from_millis(1500));
        j.cancel();
        assert_eq!(j.wait(), Ok(()));
    }
}
