<div align="center">

<img src="docs/assets/icon.png" alt="Llama Recipe Manager" width="160" height="160" />

# Llama Recipe Manager

**A native desktop launcher for `llama-server`.**

Save your `llama.cpp` invocations as named _recipes_, switch between them in one click, stop juggling flags.

[![CI](https://github.com/Llama-Recipe-Manager/llama-recipe-manager/actions/workflows/ci.yml/badge.svg)](https://github.com/Llama-Recipe-Manager/llama-recipe-manager/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](./LICENSE)
[![Tauri 2](https://img.shields.io/badge/Tauri-2-24C8DB?logo=tauri&logoColor=white)](https://tauri.app)
[![Svelte 5](https://img.shields.io/badge/Svelte-5-FF3E00?logo=svelte&logoColor=white)](https://svelte.dev)
[![Rust](https://img.shields.io/badge/Rust-stable-orange?logo=rust&logoColor=white)](https://www.rust-lang.org)

</div>

---

## Highlights

- **Recipe-first** — named bundles of `llama-server` flags, switch models and GPU layouts in one click
- **Local-first** — no account, no telemetry, no network dependency. Recipes stored in SQLite
- **Preset & INI Import / Export** — import and export recipes directly from/to `llama-server --models-preset` compatible `.ini` files or `.ripe` bundles
- **Safety rails** — flag deny-list, path confinement, input validation. See [SECURITY.md](./SECURITY.md)
- **Cross-platform** — macOS, Linux, Windows with native theming

## Presets & INI Integration

Llama Recipe Manager supports two-way integration with `llama-server` preset files:

- **Import from INI Presets**: Click the **Import INI** button in the Recipes sidebar header or run `import recipes ini presets` from the Command Palette (`Ctrl+K` / `Cmd+K`). The dialog previews models, resolves global `[*]` defaults, detects `{llama-root}` placeholders, allows remapping to your local paths, and batch-imports recipes into SQLite.
- **Single Preset Export**: Right-click any recipe in the sidebar or open the 3-dot menu in Recipe Detail, then select **Export as INI preset…** (or **Export .ripe…** for app-to-app sharing).
- **Export All to INI**: Click the **Export All** button in the Recipes sidebar header or run `export all recipes ini` from the Command Palette (`Ctrl+K` / `Cmd+K`) to generate a complete multi-model preset file with shared global `[*]` defaults.
- **Run Directly with `llama-server`**:
  ```bash
  llama-server --models-preset ./llama_models.ini
  ```
- **CLI Utilities**:
  Batch import and export directly from the terminal without opening the GUI:

  ```bash
  # Import presets with path remapping
  python3 scripts/import-recipes-ini.py path/to/presets.ini [path/to/recipes.db] [--remap llama-root=/media/models]

  # Export database to INI
  python3 scripts/export-recipes-ini.py [path/to/recipes.db] [output.ini]
  ```

## Install

Pre-built bundles ship on the [Releases](https://github.com/Llama-Recipe-Manager/llama-recipe-manager/releases) page.

```bash
git clone https://github.com/Llama-Recipe-Manager/llama-recipe-manager.git
cd llama-recipe-manager
bun install
bun run tauri dev      # development
bun run tauri build    # produces .app / .deb / .AppImage / .msi
```

Full build instructions: [`docs/Development.md`](./docs/Development.md)

## Documentation

- [Local deployment guide](./deployment.md) · [Development guide](./docs/Development.md) · [Security policy](./SECURITY.md) · [Contributing](./CONTRIBUTING.md)
- [Changelog](./CHANGELOG.md) · [Code of Conduct](./CODE_OF_CONDUCT.md)

## License

[MIT](./LICENSE) © 2026 Mohammad Ashar Khan

Built on [llama.cpp](https://github.com/ggml-org/llama.cpp), [Tauri](https://tauri.app), [SvelteKit](https://kit.svelte.dev).
