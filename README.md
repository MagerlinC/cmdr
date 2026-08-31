# cmdr

Local development stack control plane. Define layers (Database, API, Frontend, etc.) with one or more runtimes each, and toggle between them.

## Build & Install

```bash
pnpm install
pnpm tauri build
```

The built app is at `src-tauri/target/release/bundle/macos/cmdr.app`. Drag it to `/Applications` or open it directly.

## Development

```bash
pnpm tauri dev
```

## Configuration

Place a `cmdr.yaml` in `~/.config/cmdr/`. On first launch, click **Generate Sample Config** to create a starter file, or use the **Config** button to open the folder.

```yaml
layers:
  - name: Database
    runtimes:
      - name: Docker
        type: docker
        cwd: /path/to/project
        up: docker compose up -d postgres
        down: docker compose stop postgres

  - name: API
    runtimes:
      - name: Docker
        type: docker
        cwd: /path/to/project
        up: docker compose up -d api
        down: docker compose stop api
      - name: Local
        type: local
        cwd: /path/to/project/api
        up: pnpm dev
```

Relative `cwd` paths are resolved from the config file's directory.
