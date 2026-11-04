# macOS validation checklist (1.0, stage S0)

Thank you for testing! This takes about 30–40 minutes. Nothing here changes your Mac's settings, and nothing is uploaded: you send the results yourself at the end.

Works on Apple Silicon (M1–M5) and Intel Macs. If you have both, run it on both.

## 1. Build it (about 5 minutes)

Building from source avoids Gatekeeper warnings (the release builds aren't notarized yet).

```bash
xcode-select --install          # skip if the command-line tools are already installed
curl https://sh.rustup.rs -sSf | sh -s -- -y && source "$HOME/.cargo/env"
git clone https://github.com/joaoregis/cpuzeez.git && cd cpuzeez
git checkout s0-foundation
cargo build --release
mkdir -p ~/cpuzeez-results
```

Every command below runs from the `cpuzeez` folder, and its output goes to `~/cpuzeez-results`.

## 2. Command line

| # | Run | Expected | Pass? |
|---|---|---|---|
| C1 | `./target/release/cpuzeez -txt=$HOME/cpuzeez-results/report -json=$HOME/cpuzeez-results/report` | Writes `report.txt` and `report.json` without errors | |
| C2 | Open `report.txt`. Compare with  › About This Mac | Same chip, memory size, macOS version. Graphics shows the GPU / Metal | |
| C3 | `./target/release/cpuzeez -sensors \| tee $HOME/cpuzeez-results/sensors.txt` | A list of temperatures, fans (if the Mac has any), power and a battery (laptops) | |
| C4 | `./target/release/cpuzeez -log=$HOME/cpuzeez-results/log,30 2> $HOME/cpuzeez-results/log-stderr.txt` (runs 30 s) | `log.csv` with about 30 rows. The last line of `log-stderr.txt` starts with `source read time (mean):` | |

## 3. The app

Start it with `./target/release/cpuzeez` (from the terminal, so errors show up there).

| # | Do | Expected | Pass? |
|---|---|---|---|
| G1 | First start | A **Welcome to CpuZeeZ** screen. "Check for new versions at startup" is **unchecked**. Press **Continue** | |
| G2 | **Overview** | Within ~5 s the tiles show CPU temperature and CPU clock. The clock is plausible for the chip (e.g. 3–4.5 GHz on Apple Silicon) | |
| G3 | **Mainboard** and **Memory** pages | A yellow banner: *Not available: … (macOS doesn't allow third-party register access)*. There is **no** "Restart as administrator" button | |
| G4 | Banner › **Hide** | It shrinks to a small "Show what's hidden" button; clicking it brings the banner back | |
| G5 | **Sensors** | Sensors grouped by device; ticking a row draws it in the chart | |
| G6 | Sensors › **Start CSV log** | **● REC** in the window title and a red **● REC** pill above the version in the sidebar. Clicking the pill stops it, and both disappear | |
| G7 | **Tools › Clocks** | Cores listed with values, `source: measured` | |
| G8 | **Settings › Privacy & data** | The data folder path (in `~/Library/Application Support/CpuZeeZ`) and sizes per category. "Sensor logs" now has the CSV from G6 | |
| G9 | Privacy & data › Sensor logs › **Delete** | The first click turns the button into **Confirm**; the second deletes it, and the count goes to 0 | |
| G10 | **Settings › Mini monitor** on, then quit and start the app again | The mini monitor comes back after the restart | |
| G11 | **Settings › Show tray / menu-bar monitor** | A CPU temperature appears in the menu bar | |
| G12 | Privacy & data › **Delete everything** (two clicks), then quit and start again | The Welcome screen shows again | |

## 4. Cost (Activity Monitor)

Open Activity Monitor › CPU, find `cpuzeez`, and write down its **% CPU** after ~30 s in each state:

| # | State | % CPU |
|---|---|---|
| P1 | Overview visible | |
| P2 | Window minimized (Dock) | |
| P3 | Sensors page visible | |

Expected: P2 should be close to 0. P1 rises briefly every 5 s, because macOS has no live clock counter and CpuZeeZ times a short loop to measure the clock.

## 5. Screenshots

```bash
./target/release/cpuzeez -screenshots=$HOME/cpuzeez-results/shots
```

It visits every page by itself and quits after about a minute. Don't touch the window meanwhile. Expected: 17 PNG files in `shots/`.

## 6. Experiments for the overlay (no code, only Apple's own tools)

These help us decide how ZeeZ Live will measure FPS on macOS (spike SP6). There's no right answer; just tell us what you see.

| # | Do | Tell us |
|---|---|---|
| E1 | Start any Metal game or app with Apple's HUD: `MTL_HUD_ENABLED=1 open -a "<App Name>"` (or, for a Steam game, set its launch options to `MTL_HUD_ENABLED=1 %command%`) | Does a HUD with FPS appear in the corner? Which app/game? |
| E2 | While E1 runs, in another terminal: `log stream --style compact --predicate 'process == "<App process name>"' \| grep -i -E "hud\|fps\|frame" \| head -50` | Do any lines with FPS/frame times appear? Paste a few |
| E3 | Same as E1, but also add `MTL_HUD_LOG_ENABLED=1` | Any difference in E2's output? |
| E4 | Put that game in **fullscreen** (its own Space), then open CpuZeeZ's mini monitor | Does the mini monitor show over the fullscreen game, or only on the desktop? |
| E5 | `nettop -P -L 1 -x -J bytes_in,bytes_out \| head -n 15` (for the per-process network column, spike S0-X10) | Does it print per-process byte counts without `sudo`? Remove any process names you'd rather not share |

## 7. Send back

1. Zip `~/cpuzeez-results`: `cd ~ && zip -r cpuzeez-results.zip cpuzeez-results`
2. The tables above with ✅/❌ and notes (a photo or a copy of this file is fine).
3. Your Mac: model, chip (e.g. "MacBook Air M2"), macOS version.
4. Anything that looked wrong, slow or confusing, even if it isn't on the list.

The report files contain your Mac's serial numbers; screenshots taken with `-screenshots` mask them, but `report.txt`/`report.json` don't. Remove those lines first if you prefer.
