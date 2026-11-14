# Fuzz results

Coverage-guided fuzzing of every parser that reads untrusted input (`fuzz/`, cargo-fuzz on nightly Rust, `.github/workflows/fuzz.yml`). Five minutes per target run every night; the long campaigns below are started by hand with *Run workflow* and a number of minutes.

## 2026-11-13: 60 minutes per target (v1.1 M5-01)

**First run** ([43534184892](https://github.com/joaoregis/cpuzeez/actions/runs/43534184892)): 4 of the 11 targets crashed, all four in `cpuzeez-core`. Each was fixed in [#55](https://github.com/joaoregis/cpuzeez/pull/55); the inputs are kept in `fuzz/regressions/` and `robustness::fuzz_campaign_crashes` replays them on every test run.

| Target | Crash | Fix |
|---|---|---|
| `pci_config` | A Resizable BAR size field past 2^63 shifted out of range | `checked_shl`; that BAR is not listed |
| `edid` | Width × height of a detailed timing overflowed u32 | Compared in u64 |
| `spd` | The PC rating of an absurdly short tCK overflowed | `saturating_mul` |
| `output_lines` | A UTF-8 byte-order mark split across two reads was not stripped, so chunked and whole input differed | Wait for the third byte after `EF BB` |

**Second run, on the fixes** ([43536645558](https://github.com/joaoregis/cpuzeez/actions/runs/43536645558)): every target ran its 60 minutes with no crash.

| Target | Executions | Coverage (edges) | Features | Corpus |
|---|---:|---:|---:|---:|
| `edid` | 240,167,415 | 271 | 1,056 | 254 |
| `session_json` | 206,912,264 | 3,154 | 10,086 | 4,428 |
| `elevated_op` | 152,506,579 | 1,224 | 5,004 | 1,665 |
| `spd` | 130,308,722 | 1,362 | 2,246 | 358 |
| `output_lines` | 32,866,187 | 387 | 1,651 | 441 |
| `layout_json` | 25,985,151 | 2,114 | 9,548 | 2,422 |
| `hw_csv` | 21,497,833 | 481 | 3,000 | 971 |
| `pci_config` | 11,955,072 | 588 | 2,683 | 710 |
| `smbios` | 9,071,806 | 1,481 | 9,698 | 2,866 |
| `detector` | 6,785,655 | 1,315 | 5,363 | 1,114 |
| `presentmon_csv` | 2,707,739 | 1,078 | 5,752 | 1,679 |

The grown corpora are the run's `corpus-<target>` artifacts; they are not folded back into `fuzz/corpus/` yet.
