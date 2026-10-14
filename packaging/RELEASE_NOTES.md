cpuzeez is a clean-room, open-source reimplementation of CPU-Z's feature set for Windows, Linux and macOS. See the [README](https://github.com/joaoregis/cpuzeez#readme) for screenshots, the feature-coverage table and details on privileged access.

### Downloads

| Platform | File |
|---|---|
| Windows 10/11 x64 | `.msi` installer, or the `-portable.zip` |
| macOS 11+ (universal: Intel + Apple Silicon) | `.dmg` (app) or `.pkg` (app + CLI in `/usr/local/bin`) |
| Debian / Ubuntu | `.deb` (amd64, arm64) |
| Arch Linux | `.pkg.tar.zst` |
| Other Linux | `.tar.gz` |

Verify downloads with `SHA256SUMS.txt`.

### Notes

- The installers have **no publisher certificate**:
  - Windows: SmartScreen → *More info* → *Run anyway*.
  - macOS: executables are ad-hoc signed, but not Developer ID signed or notarized. Check `SHA256SUMS.txt`, attempt to open the app or `.pkg`, then use *System Settings → Privacy & Security → Open Anyway* and confirm *Open*. On macOS 11–12 use *System Preferences → Security & Privacy → General*. Repeat for the installed app if prompted. Right-click → *Open* no longer bypasses Gatekeeper on macOS 15 and later. See [Apple's instructions](https://support.apple.com/en-us/102445).
- Windows: install [PawnIO](https://pawnio.eu) and its modules, then run as administrator, to get MSR, SPD, memory timings and Super I/O sensors.
- Linux: run with `sudo` (and `modprobe msr i2c-dev`) for the same.
- The benchmark is cpuzeez's own workload: scores are not comparable with CPU-Z's.

Not affiliated with CPUID. CPU-Z is a trademark of CPUID.
