# ADR 0013: The GUI renders with OpenGL (glow) and falls back to wgpu

- **Status:** accepted (2026-11-04)
- **Decision ref:** Q6c, spike S0-X01 · [docs/v1/dependencies.md §3](../v1/dependencies.md)

## Context
glow can't start where Windows offers only OpenGL 1.1: VMs, Remote Desktop
sessions, machines without a GPU driver. wgpu can use DX12 (including WARP,
the software rasterizer), Vulkan or Metal. ZeeZ Live also needs a transparent,
click-through, always-on-top window. The plan was to switch if wgpu handled
all of that.

## Measurements (Windows 11, Ryzen 9 7900X + RTX 4060, eframe 0.36.2, release build, Overview page, 30 s idle)

| | glow | wgpu (auto: DX12) | wgpu Vulkan |
|---|---|---|---|
| Working set | **133 MB** | 259 MB | 188 MB |
| Private memory | **124 MB** | 456 MB | 384 MB |
| Idle CPU (one core) | 1.1 % | 1.3 % | 2.5 % |
| Time to first frame | 0.4–1.7 s (noisy, loaded machine) | 0.65–1.6 s | 0.5–1.5 s |
| Transparent window | ✅ | ✅ | |
| Transparent + click-through | ❌ invisible: winit sets `WS_EX_LAYERED` with no attributes; one `SetLayeredWindowAttributes(hwnd, 0, 255, LWA_ALPHA)` fixes it ✅ | ✅ | |
| No GPU driver | ❌ fails (eframe panics) | ✅ WARP ("Microsoft Basic Render Driver") renders | |

Binary size: wgpu adds about 6.8 MB. The GPU benchmark ships wgpu anyway (ADR 0006).

## Decision
- glow stays the renderer. It uses less than a third of wgpu's private memory, and CpuZeeZ often sits in the tray all day.
- If glow fails to start, whether it returns an error or panics, the GUI starts again in the same process on wgpu and logs the switch in `app.log`. `CZ_RENDERER=wgpu` forces wgpu, e.g. to work around a broken GL driver.
- ZeeZ Live's click-through overlay applies the layered-window fix after the window opens on Windows. Spike SP8 measures the overlay's GPU cost on both renderers.

## Alternatives considered
- **Switch to wgpu.** It works everywhere with no workarounds, but costs 280–330 MB more private memory on every machine.
- **Keep glow with no fallback.** The app wouldn't open in VMs or over RDP without a GPU.

## Consequences
- wgpu is compiled into the release now (it was due with the GPU benchmark in S1).
- Machines without a GPU driver get a slower software-rendered GUI instead of no GUI.
- Revisit if eframe or the GL drivers change the memory picture.
