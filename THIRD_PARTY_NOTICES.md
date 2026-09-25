# Third-Party Notices

Fairstack Coreview is licensed under the MIT License (see [LICENSE](LICENSE)). It uses the following main third-party components, each under its own license. The complete list of Rust and JavaScript dependencies with exact versions is in `src-tauri/Cargo.lock` and `package-lock.json`.

| Component | Purpose | License |
|---|---|---|
| [Tauri](https://tauri.app) | application framework | MIT or Apache-2.0 |
| [React](https://react.dev), [Recharts](https://recharts.org), [qrcode](https://github.com/soldair/node-qrcode) | interface, charts, QR codes | MIT |
| [sysinfo](https://github.com/GuillaumeGomez/sysinfo) | system information | MIT |
| [wgpu](https://wgpu.rs) | GPU stress test | MIT or Apache-2.0 |
| [window-vibrancy](https://github.com/tauri-apps/window-vibrancy) | translucent window | MIT or Apache-2.0 |
| [axum](https://github.com/tokio-rs/axum), [rustls](https://github.com/rustls/rustls), [rcgen](https://github.com/rustls/rcgen) | encrypted local network agent for the mobile app | MIT / Apache-2.0 / ISC |
| [LibreHardwareMonitor](https://github.com/LibreHardwareMonitor/LibreHardwareMonitor) (library, unmodified) | Windows hardware sensors | MPL-2.0 |
| [PresentMon](https://github.com/GameTechDev/PresentMon) | FPS measurement (Windows) | MIT |
| [PawnIO](https://github.com/namazso/PawnIO) | optional driver for hardware access on Windows. **Not bundled**; installed separately, only with the user's consent | GPL-2.0 |

The source code of LibreHardwareMonitor, used under MPL-2.0, is available at the link above. Trademarks belong to their respective owners.
