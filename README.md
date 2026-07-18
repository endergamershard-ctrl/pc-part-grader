# PC Part Grader

Transparent, PCMark-inspired system benchmarks for **Linux** and **Windows**.
Measure Everyday, Productivity, Content Creation, Graphics, and Storage suites,
then get suite scores, an overall score, letter grades, and local history.

This project is **not affiliated with UL or PCMark**. Scores are not compatible
with PCMark results.

**Releases:** https://github.com/endergamershard-ctrl/pc-part-grader/releases

## Install (one-liner)

### Linux (x86_64)

```sh
curl -fsSL https://raw.githubusercontent.com/endergamershard-ctrl/pc-part-grader/master/scripts/install.sh | bash
```

This installs the binary to `~/.local/bin`, a desktop entry, and an icon. Launch
from your app menu (Super+Space / Activities) or run `pc-part-grader`.

Uninstall:

```sh
curl -fsSL https://raw.githubusercontent.com/endergamershard-ctrl/pc-part-grader/master/scripts/uninstall.sh | bash
```

### Windows (x64)

In PowerShell:

```powershell
irm https://raw.githubusercontent.com/endergamershard-ctrl/pc-part-grader/master/scripts/install.ps1 | iex
```

This downloads the latest NSIS installer from GitHub Releases and runs it.
Launch **PC Part Grader** from the Start Menu.

> Windows builds are currently **unsigned**. SmartScreen may show a warning on
> first run — choose **More info → Run anyway**. WebView2 Runtime is required
> (preinstalled on most Windows 10/11 systems).

## Features

- **Standard** (~3–5 min) and **Extended** (~10–15 min) profiles
- Warm-ups, repeated samples, median metrics, and CV-based reliability
- Realistic workloads: parsing, office-style compute, image pipelines, GPU
  compute/render, sequential and random storage
- Score model `2.0.0` with uncapped reference-normalized scores (1000 ≈ mainstream)
- Weighted geometric means so weak subsystems stay visible
- Durable local history with previous/best comparisons
- Opt-in community settings that are API-ready but disabled by default

## Scoring methodology

Model `2.0.0`:

1. Each workload warms up, then records multiple samples.
2. The median sample becomes the workload metric.
3. Coefficient of variation (CV%) determines reliability.
4. Valid workloads are normalized against fixed reference values where **1000**
   represents an estimated mainstream desktop (mid-range 6-core CPU, mid-range
   discrete GPU, NVMe SSD).
5. Suite and overall scores use weighted geometric means.
6. A separate 0–100 grade and letter grade are derived for display.

Unreliable or failed workloads are excluded instead of counted as zero.
Reference tiers in `src-tauri/baselines/v2.json` are engineering guides, not
population percentiles. The same reference set is used on Linux and Windows.

## Suites

- **Everyday**: JSON parsing, text search, compression, small-file ops
- **Productivity**: spreadsheet formulas, sort/aggregate, document transforms, archives
- **Content Creation**: image decode/resize/filter/encode and CPU rendering
- **Graphics**: `wgpu` compute, offscreen render, transfer bandwidth diagnostic
- **Storage**: sequential and random I/O with syncs and automatic cleanup

## Privacy

- Benchmarks run entirely on-device.
- History is stored in your local app data directory.
- Community upload is off unless you opt in **and** configure an endpoint.
- Even then, this release only prepares/redacts payloads; it does not upload.

## Develop

### Linux

Dependencies (Arch example):

```sh
sudo pacman -S --needed base-devel curl wget openssl webkit2gtk-4.1 \
  appmenu-gtk-module gtk3 libappindicator-gtk3 librsvg
npm install
npm run tauri dev
```

Useful commands:

```sh
npm test
cargo test --manifest-path src-tauri/Cargo.toml
cargo run --release --manifest-path src-tauri/Cargo.toml --bin calibrate -- standard
npm run tauri build -- --no-bundle
./scripts/package-linux.sh
```

### Windows

Prerequisites:

1. [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) (C++ workload)
2. [Rust](https://rustup.rs/) (MSVC toolchain)
3. [Node.js 22+](https://nodejs.org/)
4. [WebView2 Runtime](https://developer.microsoft.com/en-us/microsoft-edge/webview2/)

```powershell
npm install
npm run tauri dev
npm run tauri build -- --bundles nsis
```

### Calibration

Always calibrate with `--release`. Debug builds are 20–60× slower on CPU
workloads and will poison references. Do **not** copy a single machine's medians
into `src-tauri/baselines/v2.json` — that makes the machine score 1000 by
definition.

## Troubleshooting

| Issue | Fix |
| --- | --- |
| `pc-part-grader: command not found` | Ensure `~/.local/bin` is on your `PATH` |
| Missing icon / app menu entry | Re-run the Linux install script; log out/in if needed |
| Windows SmartScreen warning | Expected for unsigned builds; More info → Run anyway |
| Blank window on Windows | Install/update WebView2 Runtime |
| Very low Graphics score | Integrated GPUs score below the discrete mainstream reference |

## Architecture

- `src/`: React UI (Home, Run, Results, History, Settings)
- `src-tauri/src/benchmarks/`: profiled runner and workload suites
- `src-tauri/src/scoring.rs`: score model 2.0
- `src-tauri/src/history.rs`: durable local reports
- `src-tauri/src/community.rs`: opt-in API boundary
- `src-tauri/baselines/v2.json`: shared reference values
- `scripts/`: install / uninstall / packaging helpers
- `.github/workflows/release.yml`: Linux + Windows release builds
