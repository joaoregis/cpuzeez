# Storage audit and safe cleanup

> **Status:** idea, not planned. Written 2026-11-06 for a later version (2.0 or 3.0), after 1.0.0 ships.
> Nothing here is promised. 🔬 items must be proven before they're in scope.

## 1. Why

The request that started this: *"auditing and recovering disk and SSD space"*, a problem the author and a partner hit all the time on shared PCs. CpuZeeZ already reads drives (S.M.A.R.T., health), lists processes, services and startup items with undo, and keeps a local action log. A storage audit fits next to that, **as long as it keeps the 1.0 principles**: local only, read-only by default, nothing destructive without a preview, every action in the audit log.

## 2. Rules for every feature here

- **Read-only first.** Scanning never changes anything. Each cleanup action is a separate step.
- **Exact numbers before any action.** The bytes each item frees, and a preview of what goes.
- **Nothing irreversible.** Items go to the Recycle Bin / Trash, or are removed by the OS's own tool (`cleanmgr`, DISM, `journalctl --vacuum`, the package manager). CpuZeeZ never deletes system folders by hand.
- **Protected paths** (like the protected-services list): the OS, the user's profile roots, anything under a game's anti-cheat folder, cloud-sync roots.
- **Elevation per action**, asked at the moment of the action (ADR 0002). No standing administrator rights.
- **Nothing runs by itself** and nothing is uploaded (P1). No scheduled "cleaners".
- **Every action is written to the action log**, with what it freed.

## 3. Features, in the order I'd build them

### 3.1 Space map (the core)
A treemap and a sortable table of the biggest folders and files on each drive; filter by age, type and owner.
- **Windows:** read the NTFS MFT directly (whole drive in seconds). Needs an administrator (🔑); without it, a parallel directory walk (minutes).
- **Linux / macOS:** parallel walk with `statx` / `getattrlistbulk`. macOS needs the Full Disk Access permission for protected folders, and the UI says which folders were skipped.
- **Counting traps, to handle on purpose:**
  - hard links (WinSxS counts the same bytes many times);
  - junctions and symlinks (loops, double counting);
  - cloud placeholders (OneDrive "files on demand" use almost no disk);
  - sparse and compressed files (size vs size on disk);
  - mounted volumes inside a folder.
- Tooling: drawn with egui like the other charts; the walk runs on a thread like the System hub's samplers (never on the UI thread).

### 3.2 What changed since last time
Two scans compared, the same pattern as *Snapshot* in the Task Manager: "what grew 40 GB this week?". Scans are saved locally; the user decides how many to keep.

### 3.3 Duplicate files
Group by size, then a partial hash, then a full SHA-256 (the hash function already exists). Shows which copy would stay; removal goes to the Recycle Bin, never straight to deletion. Option: skip small files, skip chosen folders.

### 3.4 Reclaimable space report
A per-category list with exact sizes:
- **Windows:** temp files, Windows Update cache, Delivery Optimization, crash dumps, thumbnails, shader caches, `Windows.old`, System Restore shadow-storage size (shown, with a link to change it).
- **Linux / macOS:** package caches (apt, pacman, brew), journal logs.
- **For developers:** `node_modules` and `target/` of projects untouched for N months, `.gradle`, Docker and cargo caches.
- Cleanup uses the official tool for that category, or the Recycle Bin. Browser caches are listed but cleaned from the browser.

### 3.5 Per-user usage
How much each account on a shared PC takes (builds on the Users tab). Useful for two people on one machine.

### 3.6 SSD care
Warn when free space is under ~10 % (performance and wear get worse), and show whether TRIM is on. Defrag/ReTrim stay in the post-1.0 *Drive optimization* item.

### 3.7 Installed programs
List with size, install date and last use. "Last use" comes from the App history recorder, so it only covers apps seen while CpuZeeZ ran, and the screen says so. *Uninstall* opens the program's own uninstaller; CpuZeeZ never removes files itself. Pre-installed Windows apps are listed with what they do, with no bulk removal.

### 3.8 Startup and services, with evidence
Extends the tabs 1.0 already has:
- suggestions only from a short documented list (e.g. Fax, Remote Registry, Print Spooler with no printer), each saying what stops working;
- the change is *Manual*, not *Disabled*, with the existing Undo;
- boot time shown before and after. On a modern PC the gain is small, and the screen says so.

## 4. What we won't build, and why

| Idea | Why not |
|---|---|
| **Registry cleaner** | No measurable speed gain, Microsoft doesn't support it, and one wrong key can stop Windows from booting |
| RAM "optimizer" | Frees memory the OS would have used as cache; slows things down afterwards |
| Driver updater | Wrong-driver risk; vendors' own tools do it better |
| Defragmenting an SSD | Wear for no benefit |
| One-click "tweak" packs | Break things you only notice weeks later |

## 5. Rough order and size

1. **3.1 + 3.2** (space map and comparison): read-only, the part that solves the real problem. Largest piece: the Windows MFT reader.
2. **3.3 + 3.4** (duplicates and reclaimable): first actions, Recycle Bin and official tools only.
3. **3.5 + 3.6 + 3.7 + 3.8**: smaller, built on tabs that already exist.

## 6. Open questions

- 🔬 Raw MFT reads on volumes with BitLocker, ReFS or exFAT: fall back to the walk, but measure the cost.
- 🔬 Is `getattrlistbulk` fast enough on APFS for a whole-disk map without Full Disk Access?
- Where it lives in the app: inside *Devices*, or its own *Storage* entry next to *System*. A new entry fits better if the space map gets its own tabs.
- Whether scans are kept between runs by default (privacy: they list file names). Default: kept only if the user turns it on.
