# CpuZeeZ threat model

> Status: written for `1.0.0-beta.8` (S3-05). It describes what the code does today, which test or CI check backs each control, and the risks we accept. Where a claim has no test, it says so.
> Reporting a vulnerability: see [SECURITY.md](../../SECURITY.md).

## 1. What we protect

| Asset | Why it matters |
|---|---|
| **The user's machine** | CpuZeeZ runs elevated for some actions (kernel driver install, ending system processes, service changes). A flaw there is a privilege escalation. |
| **The user's games and accounts** | An overlay or a monitor that touches a game can get its owner banned by an anti-cheat. |
| **The user's data** | Reports, captures and histories list hardware serials, process names and play habits. |
| **The release channel** | Users install what we publish and let the app update itself. |

## 2. Principles that shape the model

- **P1: local only.** No telemetry, no account, no cloud. The update check is the one network path (`update.rs`), off until the user turns it on.
- **P3: elevation is per action.** The app starts unprivileged. Each privileged action runs in a one-shot elevated copy of the same executable.
- **P4: no injection, ever** ([ADR 0012](../adr/0012-no-code-injection-ever.md)). Other processes are opened with `PROCESS_QUERY_LIMITED_INFORMATION` at most; their memory is never read.

## 3. Trust boundaries

```
 ┌──────────────── user session (unprivileged) ────────────────┐
 │  CpuZeeZ GUI/CLI ──► core (pure logic) ──► platform (OS)    │
 │        │                                                    │
 │        │ Action (closed enum, validated, hex on the wire)   │
 │        ▼                                                    │
 └────────┼────────────────────────────────────────────────────┘
          │  one OS elevation prompt per action (UAC / pkexec / macOS auth)
 ┌────────▼────────────────────────────────────────────────────┐
 │  elevated one-shot copy: `--elevated-op=<hex>`              │
 │  decode ─► validate ─► perform ─► exit code ─► audit log    │
 └─────────────────────────────────────────────────────────────┘

 outside:  GitHub (releases)   PawnIO driver (third party, signed)
           PresentMon (third party, pinned)   OS tools (curl, systemctl, nettop…)
           firmware/device bytes (SMBIOS, SPD, EDID, PCI)   files on disk
```

## 4. Who might attack, and from where

| Actor | Can | Cannot (assumed) |
|---|---|---|
| **Malicious input** (a crafted file the user opens or imports, a damaged capture, a hostile device descriptor) | Make a parser misbehave | Run code without a parser bug |
| **Local unprivileged process** (malware running as the same user) | Edit the user's files, start our exe, trigger prompts | Bypass the OS elevation prompt, or write to protected folders |
| **Network attacker** | Tamper with the update check's traffic | Break TLS to GitHub |
| **Compromised upstream** (a GitHub action, a crate, PresentMon, PawnIO) | Ship bad code into the build or the package | — (see §8, accepted and mitigated risks) |
| **Curious game** (anti-cheat) | Detect and punish tools that touch the game | — |

Out of scope: an attacker who is already administrator or kernel; physical access; a user who approves any prompt without reading it.

## 5. Entry points and their controls

| Input | Who controls it | How it is read | Control | Backed by |
|---|---|---|---|---|
| `--elevated-op=<hex>` | Anyone who can start our exe | `actions::decode`: hex → JSON → closed `Action` enum with `deny_unknown_fields`, then `Action::validate` | Only variants we defined; service names (no leading `-`, bounded charset), startup ids (no path separators), module names (allow-list), process targets must carry their start time; refused when not elevated (exit 3) | `strings_in_actions_are_validated`, `service_edits_are_validated`, fuzz `elevated_op`, `elevated_op_parser` |
| PID in a process action | The user, then time | `verify_identity` compares the start time again in the elevated copy | A reused PID is refused; anti-cheat processes refuse memory dumps | tests in `executor`, `anti_cheat` |
| Update answer (GitHub JSON), `SHA256SUMS.txt`, installer | Network, GitHub | `update::parse`, `sha256_file` | The download is checked against the release's checksums file before it is run; no checksum, no install; version comparison is SemVer | `update_is_the_only_network_path`, `update_answers`, `newer` tests |
| Bundled binaries (PawnIO installer, PresentMon) | Our release | SHA-256 pins in `packaging/pins.txt`, compiled into the app | Checked at release build **and** by the app before each run | `pins_are_compiled_in`, release workflow `sha256sum -c`, the sidecar's `BadChecksum` state |
| `settings.json`, `history.json`, `app_history.sqlite`, `sessions/*` | The user (and anything running as them) | serde with defaults; SQLite through bound parameters | A bad file is moved to `.bad` and defaults load; a file from a newer version is never overwritten; data folder is private (`0700` on Unix) | `settings_from_any_version`, settings tests, `session_json_files` |
| Capture files, imported CSVs, MangoHud logs | The user, other tools | `frames::parse_presentmon`, `parse_mangohud`, `session::parse_hw_csv` | No panics on any input; session length capped (`MAX_SESSION_SECONDS`); reports escape every string and contain no script | `presentmon_csv_files`, `hardware_and_mangohud_logs`, `corrupt_sessions_stay_bounded`, fuzz `presentmon_csv`, `hw_csv`, `session_json` |
| PresentMon output | The sidecar we pin | Chunk-safe UTF-8/UTF-16 decoder | Same parser as above | `output_lines_ignore_chunking`, fuzz `output_lines` |
| Layout and report JSON the user pastes or opens | The user | `Layout::from_json`, report loader | Size-bounded parse; errors shown, never panics | `layout_files`, `imported_reports`, fuzz `layout_json` |
| Firmware and device bytes (SMBIOS, SPD, EDID, PCI config, AMD PM tables) | Hardware, which can be hostile or buggy | Pure decoders in `core` | Bounds-checked; out-of-range fields left out, never guessed | `smbios_tables`, `spd_images`, `edid_blocks`, `pci_config_space`, `smu_pm_tables`, fuzz targets |
| Text from OS tools and `/proc` (`systemctl show`, `nettop`, `vmstat`, `meminfo`, netlink replies) | The OS | Pure parsers | No panics; unknown lines ignored | `small_text_parsers`, `network_and_process_text` |

## 6. The elevated path in detail

1. The GUI builds an `Action` and calls `executor::run`. `validate()` runs first, in the unprivileged process.
2. If the action needs administrator rights and we don't have them, the app starts **itself** elevated, with one argument: `--elevated-op=<hex>`. Nothing else crosses the boundary on purpose; on Linux only a fixed list of display variables is passed along so the elevated copy can reach the session.
3. The elevated copy runs `decode` again (the argument is untrusted: anything on the machine can start the exe with it), then refuses unless it really is elevated.
4. It performs the action, prints one line, and exits. The result, success or failure, is appended to the local `audit.log`. A cancelled prompt is not logged because nothing ran.
5. Protected services (a documented list) are refused by `platform::services::apply` even for an administrator.

What an attacker gains by starting `cpuzeez.exe --elevated-op=…` themselves: nothing the prompt doesn't show. The user sees a UAC prompt for CpuZeeZ, and the action set is the closed list in `core::actions`. An attacker who is already the same user can still **cause prompts**; we rely on the user declining unexpected ones (see §8).

## 7. Reading other processes

- Only `PROCESS_QUERY_LIMITED_INFORMATION` is requested. A test fails the build if `PROCESS_VM_READ`, `PROCESS_ALL_ACCESS`, `ReadProcessMemory` or `PROCESS_TERMINATE` appear in the sampling code (`reading_uses_only_the_limited_right`).
- Ending, suspending or changing a process uses a separate module (`proc_ops`) and only runs on an explicit user action.
- Memory dumps of processes on the anti-cheat list are refused in the elevated helper too.
- ZeeZ Live's **anti-cheat safe mode** closes the single gate through which every PawnIO call passes (`Module::exec`), per game.
- The overlay is a window of ours: transparent, always on top, never focused. No hook, no injection.

## 8. Risks, how we reduce them, and what we accept

| Risk | Mitigation today | Residual (accepted) |
|---|---|---|
| **Unsigned releases** (ADR 0009): Windows shows an unknown publisher | `SHA256SUMS.txt` with every release; the app verifies updates against it; installers documented in the README | A user who installs from somewhere else has no signature to check. Planned: signing when funded or through a free OSS program |
| **A compromised release** also swaps its checksums file (the sums travel with the files) | Releases are built by CI from a tag, with a read-only token everywhere except the publish job | The sums protect against corruption and mirrors, **not** against a hijacked repository or account. A detached signature (minisign or Sigstore) would; not built |
| **Portable zip run from a folder the user can write**, then elevated: a same-user attacker could swap the exe before the prompt | The MSI installs to `Program Files` (administrator-only); the UAC prompt shows the path | If a user runs the portable copy from a writable folder and approves a prompt, the exe they see is the exe that runs. Documented in the README |
| **Third-party code that runs elevated**: PresentMon inherits our elevation; the PawnIO driver is kernel-level | Both pinned by SHA-256 and checked before use; PawnIO modules are an allow-list; PawnIO is optional and install is an explicit button; safe mode for anti-cheat games | We don't audit PresentMon's or PawnIO's code. A malicious upstream release **with the same pinned hash** is impossible; a malicious release that **we** re-pin is not, so each re-pin is reviewed |
| **GitHub Actions and crates** (build supply chain) | Actions pinned by commit SHA; read-only workflow tokens; `cargo-deny` on licenses, advisories and sources; Dependabot opens reviewable pull requests; `Cargo.lock` committed | A reviewer can still approve a bad update. Reproducible builds are not claimed |
| **Same-user malware** | Data folder private on Unix; no secrets stored | On Windows the profile's default ACLs apply; we keep no credentials, but captures list process names |
| **Prompt fatigue**: the user approves an unexpected elevation | One prompt per action, naming the action in the app first; the audit log lists every action that ran | We can't stop a user from approving |
| **`curl` for the update check** | Only called by `update.rs` (a test enforces that nothing else touches the network); fixed arguments; 8–30 s timeouts | Resolves `curl` through the normal search path |
| **SQLite file** (`app_history.sqlite`) | Own file in the private data folder; every query uses bound parameters | A same-user attacker can edit it, like any other file of ours |

## 8.1 Privacy

- Nothing is uploaded. Captures, sessions and histories stay in the data folder; *Settings › Privacy & data* lists every category, its size and a delete button (ZeeZ Live sessions included).
- Nothing is recorded until the user starts it; ● REC shows while it does.
- *Diagnostic bundles* (the app log plus this computer's report) have serial numbers, MAC and IP addresses removed before they are written, and stay on this computer.

## 9. How this is checked

| Layer | What | Where it runs |
|---|---|---|
| Unit and GUI tests | Every control above has a named test | every push, Windows/Linux/macOS |
| Deterministic robustness tests | 15–20 thousand mutated inputs per parser, fixed seed, plus invariants (statistics order, evidence argument counts, report escaping, bounded size and time) | every push (`robustness.rs` in `core`, `platform`, and the app) |
| Coverage-guided fuzzing | 11 targets: `elevated_op`, `layout_json`, `smbios`, `spd`, `edid`, `pci_config`, `presentmon_csv`, `output_lines`, `hw_csv`, `session_json`, `detector` | nightly (`fuzz.yml`, nightly Rust, 5 minutes each) |
| Dependencies | `cargo-deny`: licenses, RustSec advisories, sources | every push |
| Static analysis | `clippy -D warnings` on Windows, Linux and macOS targets | every push |
| Packaging | MSI install, upgrade chain, downgrade block, clean uninstall; `.deb` install/run/uninstall | every push |
| Soak | Memory, handles and threads must plateau over hours | nightly, and before a release ([soak.md](../testing/soak.md)) |

## 10. Known gaps

- No detached signature on releases (see §8).
- No third-party penetration test. An AI-assisted audit guide is in [docs/audit/](../audit/README.md).
- The kernel-driver surface (PawnIO modules) is reviewed only to the extent of the allow-list.
