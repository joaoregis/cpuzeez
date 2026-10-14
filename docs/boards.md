# Board sensor maps

A Super I/O chip (Nuvoton NCT67xx, ITE IT86xx/IT87xx, ...) only measures
*inputs*: "voltage input 4", "temperature input 2", "tachometer 3". Which rail,
diode or fan header each input is wired to, and through which resistor
divider, is decided by the motherboard designer. CpuZeeZ keeps that knowledge
as data, one entry per board, in `BOARDS` in
[`src/sensors/superio.rs`](../src/sensors/superio.rs).

Without an entry for your board you still get every input, named after the
chip's datasheet (`CPUVCORE`, `VIN1`, `AVCC`, `TMPIN3`, `AUXFAN0`, ...) and
scaled only by the chip's own internal dividers. That is correct for the CPU
core voltage, battery and 3.3 V standby rails, but the +12 V and +5 V inputs
read low (they sit behind a divider on the board), and the names don't tell
you which header is which.

## How an entry looks

```rust
b(
    "GIGABYTE",                      // SMBIOS board vendor (normalized, substring)
    &["B550 AORUS ELITE"],           // SMBIOS board product patterns (normalized, substring)
    &["IT8688"],                     // Super I/O model as CpuZeeZ reports it
    m(
        &[                           // voltages: input, label, multiplier
            v(0, "CPU", 1.0),
            v(2, "+12V", 6.0),       // 1 + Ri/Rf of the board's divider
            v(7, "+3.3V Standby", 2.0), // chip-internal /2
        ],
        &[t(1, "Chipset"), t(2, "CPU")], // temperatures: input, label
        &[t(0, "CPU"), t(1, "SYS 1")],   // fans: input, label
    ),
),
```

* **Matching.** Vendor and model are compared after upper-casing and
  dropping everything that isn't a letter or digit, so `PRO X870-P WIFI
  (MS-7E47)` matches the pattern `PRO X870-P WIFI`. A pattern matches when it
  is contained in the product name, so `B550 AORUS ELITE` also covers
  `B550 AORUS ELITE V2`. The first matching entry wins: put specific boards
  before the chipset-series defaults (`"B650"`, `"X870"`, ...).
* **Multiplier `k`.** Volts at the pin (ADC count × the chip's LSB) times
  `k`. `k` includes the board divider (`1 + Ri/Rf`) *and* the chip's internal
  /2 on AVCC, 3VCC, 3VSB and VBAT style inputs (`k = 2.0` for those).
* **Hiding.** A non-empty list shows only the inputs it names; anything not
  listed is hidden (floating inputs, rails the vendor tool doesn't show). An
  empty list (`&[]`) keeps the chip's generic list for that kind.
* **Row numbers.** `v()` reports the input on its own row number. `vr(input,
  row, ...)` puts it on another row, for when you want to match the numbering
  of another tool's report.
* **Chipset temperature.** Label the input the board routes the chipset
  (AMD Promontory, Intel PCH) diode to `Chipset` or `PCH`. It then also shows
  up in the separate *Chipset* monitor that the Sensors view shows next to
  the CPU and GPU temperatures.

## Contributing a map

1. Run CpuZeeZ elevated (Windows: as administrator with PawnIO; Linux: as
   root, with no `nct6775`/`it87` driver bound to the chip) and dump the
   board section with every input visible:

   ```
   # Windows (cmd)
   set CPUZEEZ_EC_DUMP=1
   cpuzeez -txt=board
   # Linux
   sudo CPUZEEZ_EC_DUMP=1 cpuzeez -txt=board
   ```

   `CPUZEEZ_EC_DUMP=1` lists the inputs your board's map would hide as
   `NAME (unmapped)`, and for NCT6683/6686/6687 chips adds the raw EC
   register page `EC 0x100` under *Hardware registers*. The file `board.txt`
   contains the *Monitoring* (board model), *LPCIO* (chip) and *Hardware
   Monitors* sections you need.
2. Write down what the board vendor's own tool or the BIOS hardware-monitor
   page shows at the same moment: each voltage, temperature and fan with its
   name. Idle the machine so values don't drift between the two readings.
3. Match the readings: an input whose pin voltage is 1/6 of 12.1 V is the
   +12 V rail with `k = 6`; a fan input that stops when you unplug a header
   is that header. Unplugging fans and watching RPM go to zero is the
   quickest way to name tachometers.
4. Either open a pull request adding the entry to `BOARDS` (keep the
   `cargo test` checks passing, they validate the table), or open a
   [*Board sensor map* issue](../.github/ISSUE_TEMPLATE/board-sensors.yml)
   with `board.txt` and your notes, and a maintainer will turn it into an
   entry.

Please only contribute readings you took yourself or facts from public
documentation (board manuals, datasheets). Don't paste tables copied from
other programs' source code.
