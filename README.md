# Fairstack Coreview

Know your computer inside out. Fairstack Coreview is a modern system information and monitoring app for macOS and Windows.

- Detailed hardware information (processor, memory, graphics, storage, network, and more)
- Live monitoring of load, temperatures, clocks, fans and power draw, with per-sensor history
- History of temperatures and clock speeds over time, with markers and CSV export
- Stress tests for CPU, memory, disk and GPU
- Transparent overlay with FPS for games (Windows)
- Startup program manager, process list, network connections
- Snapshots to compare what changed in your hardware
- Companion mobile app over your local network (view stats, simple remote commands)
- 12 languages, custom themes and window transparency

## Download

Installers for macOS and Windows are published on the [Releases](../../releases) page.

## Sensors on Windows

CPU temperatures and motherboard fan speeds need a hardware-access driver on Windows. Fairstack Coreview can install the open-source [PawnIO](https://pawnio.eu) driver for you, only after you agree. It is not bundled with the app and can be removed at any time in *Apps*. GPU data comes from the GPU driver (`nvidia-smi` for NVIDIA cards) and works without it.

## Build from source

Requirements: Node.js 20+, Rust (stable). On Windows also the .NET SDK (for the sensor helper in `sensors-helper/`).

```
npm install
npm run tauri dev       # run in development
npm run tauri build     # build installers
```

On Windows, build the sensor helper before `tauri build`:

```
dotnet publish sensors-helper/CoreviewSensors.csproj -c Release -o sensors-helper/out
```

The repository's GitHub Actions workflow (`.github/workflows/build.yml`) builds macOS (Apple Silicon and Intel) and Windows installers.

## Privacy

Fairstack Coreview does not send any data anywhere. See [PRIVACY.md](PRIVACY.md).

## Code signing

See [CODE_SIGNING_POLICY.md](CODE_SIGNING_POLICY.md).

## License

[MIT](LICENSE). Third-party components and their licenses are listed in [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
