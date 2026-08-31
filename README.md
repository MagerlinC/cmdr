# cmdr

Cmdr is a local development control plane which makes it easier to run various services on your machine, and especially to swap between predefined sets of configurations called layers.

Define layers (Database, API, Frontend, etc.) with one or more runtimes in each, and toggle between the various runtime variants.
<img width="497" height="630" alt="image" src="https://github.com/user-attachments/assets/19515b68-f34b-4279-8630-f85d4e3d5234" />

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
      - name: Terminal
        type: terminal
        cwd: /path/to/project/api
        up: pnpm dev
```

Relative `cwd` paths are resolved from the config file's directory.
