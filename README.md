# cmdr

Cmdr is a local development control plane which makes it easier to run various services on your machine, and especially to swap between predefined sets of configurations called layers.

Define layers (Database, API, Frontend, etc.) with one or more runtimes in each, and easily toggle between the various runtime variants, either via clicking the buttons or start hotkey-sequences (see below).

<img width="497" height="630" alt="image" src="https://github.com/user-attachments/assets/19515b68-f34b-4279-8630-f85d4e3d5234" />

## Hotkey sequences
The layers are numbered in order, and hotkey sequences can be begun by pressing their corresponding number key. After pressing a key, the various actions of that layer will be highlighted with their own hotkeys.

As an example, pressing `1r` restarts layer 1, while `3<tab>` toggles between runtimes of layer 3.

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
