# 1.0.0-beta.1: what to test

Thanks for trying the first 1.0 beta. These are the checks the automated tests can't do: they need your hardware, your games or your judgement. Each one says what "good" looks like. Report anything that isn't, with the report (*Tools › Reports › Save .TXT*) if you can.

Install the MSI with *Hardware access (PawnIO)* ticked and run CPU ZeeZ **as administrator** for the full picture. Some checks also say what to expect without admin.

## 1. First look (5 minutes)

- [ ] **Overview:** the summary line names your CPU, board, memory and GPU; click it and each part opens its page. The health pills match what you know about your build. *Customize* changes the tiles and *Restore defaults* brings them back.
- [ ] **`Ctrl+K`:** type "tRFC", "BIOS", "PCIe" or "save report"; Enter goes there.
- [ ] **Right-click any row:** copy works; *Pin* puts it on the Overview's *Pinned* card.
- [ ] **Settings › Units:** switch to °F and GHz; every page and tile follows.
- [ ] **Language:** Português and Español read naturally (tell us about any awkward or English leftover).

## 2. Pages

- [ ] **Processor:** the per-core table's *Idle states* (C1/C2/C3 %) move as the PC idles and works; *Preferred* ranks your best cores (on a 7900X, cores 0–1 first). Intel: *Power limits* shows PL1/PL2.
- [ ] **Mainboard:** *Resizable BAR* and *Above 4G decoding* match your BIOS.
- [ ] **Memory:** secondary timings and the active profile match the BIOS (AMD). Intel: tell us what's missing.
- [ ] **Devices:** USB shows as a tree; your monitor shows HDR and its refresh range; *Audio* lists your outputs with their format; Bluetooth headsets/mice show their battery; drives show *Encryption*.

## 3. Benchmarks (about 15 minutes)

Close games and heavy apps first; the results record the power plan and the starting temperature.

- [ ] **Processor › CPU suite**, *Runs: 3*: the total is stable within a few % (on the reference 7900X it's ~1000). Try *Thread scaling* and, on a hybrid Intel, *P × E cores*.
- [ ] **Processor › Sustained**, 10 min: on a well-cooled desktop the *Drop* stays small; a laptop or a weak cooler shows a bigger one.
- [ ] **Memory › More memory tests:** STREAM/GUPS run; *Large pages* says why it's unavailable unless you granted *Lock pages in memory*.
- [ ] **Graphics:** pick your GPU; *Upload/Download* should match your slot (PCIe 4.0 x16 ≈ 25 GB/s, x8 ≈ 13 GB/s).
- [ ] **Storage:** SEQ/RND numbers look like your drive's class. **Sustained write** (32 GB, writes 32 GB to the drive): a fast NVMe shows a cliff where its cache runs out.
- [ ] **Latency:** the heat map shows your CCD/cluster layout.
- [ ] **Save to history** after any run, then *History › Compare* two runs: every sub-test shows its %.

## 4. Stability test

- [ ] **Stress test:** CPU *Heavy* + *Memory 4 GB* + *GPU*. Temperatures and power rise; set *Stop at* a few degrees above the current CPU temperature and check it stops by itself with the reason.

## 5. Drive health (Tools › Drive health)

- [ ] The health card matches your drive's tool (life left, written TB). Type the drive's rated TBW from its datasheet: the % appears.
- [ ] **Self-test › Short** (your call: the drive tests itself for ~2 minutes and keeps working). Progress shows and the result lands in *History*. Some drives have no NVMe self-test (the Kingston A1000 here): the card says so.
- [ ] **Surface scan › Sample:** finishes in seconds with no unreadable blocks on a healthy drive.

## 6. Tray and mini monitors

- [ ] **Settings › Tray icons:** add GPU temperature as a *Ring* and RAM as a *Bar*: two icons appear (Windows 11 may hide new icons under ^; drag them out). The menu shows/hides the app, pauses monitoring and changes the update speed; the tooltip lists every value.
- [ ] **Settings › Mini monitors:** add a second one with the *Grid* preset. Drag both near a screen edge: they snap. *Lock* makes clicks go through (unlock from the tray). Start a full-screen game: they hide, and come back when you leave.

## 7. Games and anti-cheat (your call, your account)

- [ ] Play a match with CPU ZeeZ open (sensors on, PawnIO loaded) in games with kernel anti-cheat (Vanguard, FACEIT, EAC, BattlEye). Nothing should happen: CPU ZeeZ never touches game processes. If an anti-cheat complains, tell us which and when; that's spike X08.
- [ ] With a game that has DLSS or FSR Frame Generation, and one that runs above 240 FPS: nothing to do in the app yet (ZeeZ Live is the next beta), but note the games; we'll use them for spike X02.

## 8. Mac and Linux

- [ ] **macOS:** the checks in [macos.md](macos.md), plus *Benchmarks › Latency* (a spread instead of a heat map) and *Graphics* (Metal).
- [ ] **Linux:** the tray (needs a StatusNotifier host; GNOME needs the AppIndicator extension), *Drive health* as root, and `/sys/bus/thunderbolt` devices if you have any.
