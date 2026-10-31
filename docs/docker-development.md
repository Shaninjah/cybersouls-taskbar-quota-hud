# Docker development

The application uses native Windows APIs and Explorer's taskbar. Docker Desktop's
Linux engine provides the Rust toolchain, a MinGW Windows cross-compiler, and Wine
with a virtual display for unit tests. It does not display the widget in the host
Windows taskbar or provide access to the host Windows Credential Manager.

From the repository root:

```powershell
docker compose up -d --build
docker compose exec dev cargo fmt --check
docker compose exec dev cargo test --locked
docker compose exec dev cargo clippy --locked
docker compose exec dev cargo build --release --locked
```

Source changes are immediately visible through the repository bind mount. Cargo
downloads and build outputs use named Docker volumes. The container exposes no
ports and does not mount provider credential directories. Provider unit tests use
fixtures; authenticated quota requests and taskbar placement need native Windows
validation.

Compose uses Docker's existing `bridge` network, so it does not allocate another
subnet on machines with many existing development projects. The service is limited
to two CPUs, two Cargo compilation jobs, and 3 GiB of RAM.

To copy the cross-compiled executable for a local Windows smoke test:

```powershell
New-Item -ItemType Directory -Force dist
docker compose cp dev:/build/x86_64-pc-windows-gnu/release/cybersouls-taskbar-quota-hud.exe dist/
(Get-Item dist/cybersouls-taskbar-quota-hud.exe).VersionInfo | Select-Object ProductVersion, FileVersion
```

This executable targets Windows GNU; official releases continue to use the
existing Windows CI and `scripts/build-release.ps1`. Do not replace the release
workflow with a Wine test run. The installer safety checks require PowerShell on
Windows and can be run separately using `./scripts/test-install-safety.ps1`.

```powershell
docker compose stop
docker compose start
docker compose down
```

`down` preserves the Cargo/build volumes. Avoid `down -v` unless you intend to
discard those caches.
