# Local Compilation & Deployment Guide

This document provides step-by-step instructions for compiling **Llama Recipe Manager** from source and installing it locally on your Linux system.

---

## 1. Current Environment Context

- **Installed Binary**: `/usr/bin/llama-recipe-manager` (Installed via `.deb` package)
- **Data Location**: `~/.local/share/com.ashar.llama-recipe-manager/recipes.db`
- **Tauri Bundle Identifier**: `com.ashar.llama-recipe-manager`

> [!NOTE]
> Because the Tauri bundle identifier remains `com.ashar.llama-recipe-manager`, the freshly compiled build will **automatically load your existing 40 recipes and settings** from `~/.local/share/com.ashar.llama-recipe-manager/recipes.db`.

---

## 2. Backup Your Recipes Database (Recommended)

Before updating your local installation, create a backup copy of your database:

```bash
cp ~/.local/share/com.ashar.llama-recipe-manager/recipes.db \
   ~/.local/share/com.ashar.llama-recipe-manager/recipes.db.bak
```

---

## 3. Prerequisites

### Rust Toolchain
Ensure Rust stable (1.78+) is installed:
```bash
rustc --version
cargo --version
```
*(If missing or outdated, update with `rustup update stable`)*

### Node / Bun
Upstream uses Bun by default. If `bun` is not yet installed:
```bash
curl -fsSL https://bun.sh/install | bash
source ~/.bashrc
```
*(Alternatively, you can use `npm` / Node v20+)*

### System Dependencies (Ubuntu / Debian)
Tauri 2 requires WebKit2GTK and related GUI libraries:
```bash
sudo apt update && sudo apt install -y \
  libwebkit2gtk-4.1-dev \
  build-essential \
  curl \
  wget \
  file \
  libxdo-dev \
  libssl-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev
```

---

## 4. Compile From Source

### Step 1: Install Frontend Dependencies
From the repository root:

```bash
cd /media/michaelq/Data/anti_gravity/llama-recipe-manager
bun install
```
*(Or `npm install` if using Node)*

### Step 2: Build the Release Package
For local builds, disable updater signature generation (which requires upstream's private release signing key):

```bash
bun run tauri build -c '{"bundle": {"createUpdaterArtifacts": false}}'
```
*(Or `npx tauri build -c '{"bundle": {"createUpdaterArtifacts": false}}'` if using npm)*

> [!NOTE]
> If you run `bun run tauri build` without disabling updater artifacts, you will see:
> `Error: A public key has been found, but no private key. Make sure to set TAURI_SIGNING_PRIVATE_KEY`
> **This does NOT affect your build!** The `.deb` and `.AppImage` packages are already completely built and written to disk before this error occurs.

During the build process:
1. The SvelteKit frontend is compiled into `build/`.
2. Cargo builds the optimized Rust binary with all dependencies.
3. Tauri packages the outputs into `.deb` and `.AppImage` bundles under:
   ```
   src-tauri/target/release/bundle/deb/Llama Recipe Manager_0.5.1_amd64.deb
   src-tauri/target/release/bundle/appimage/Llama Recipe Manager_0.5.1_amd64.AppImage
   src-tauri/target/release/llama-recipe-manager   (standalone binary)
   ```

---

## 5. Install Locally

You have three options to install your build. **Option A is recommended** as it updates your existing system package cleanly.

### Option A: Install `.deb` Package (Recommended)
This directly replaces `/usr/bin/llama-recipe-manager` and refreshes desktop menu icons:

```bash
sudo dpkg -i "src-tauri/target/release/bundle/deb/Llama Recipe Manager_0.5.1_amd64.deb"
```

### Option B: Replace Standalone Binary
If you prefer not to use `dpkg`:

```bash
# Ensure no server or app instance is currently running
killall llama-recipe-manager 2>/dev/null || true

# Overwrite system binary
sudo cp src-tauri/target/release/llama-recipe-manager /usr/bin/llama-recipe-manager
```

Or install it to your user binary path (`~/.local/bin`):
```bash
cp src-tauri/target/release/llama-recipe-manager ~/.local/bin/llama-recipe-manager
```

### Option C: Run as AppImage
Make the generated AppImage executable and place it in your applications directory:

```bash
mkdir -p ~/Applications
cp src-tauri/target/release/bundle/appimage/llama-recipe-manager_*_amd64.AppImage ~/Applications/
chmod +x ~/Applications/llama-recipe-manager*.AppImage
```

---

## 6. Verify the Installation

1. Launch the updated application:
   ```bash
   llama-recipe-manager
   ```
2. Confirm that:
   - All 40 existing recipes appear in the sidebar immediately.
   - Right-clicking any recipe item displays the **"Export as INI preset…"** option.
   - Opening a recipe's detail view shows **"Export as INI preset…"** in the 3-dot dropdown.
   - Clicking the new **"Export all recipes as INI"** button in the sidebar header exports a unified `llama_models.ini` file.
   - Pressing `Ctrl+K` shows the `export all recipes ini` command in the Command Palette.

---

## 7. Troubleshooting & FAQ

### Error: `TAURI_SIGNING_PRIVATE_KEY` not set

```text
    Finished 3 bundles at:
        src-tauri/target/release/bundle/deb/Llama Recipe Manager_0.5.0_amd64.deb
        src-tauri/target/release/bundle/rpm/Llama Recipe Manager-0.5.0-1.x86_64.rpm
        src-tauri/target/release/bundle/appimage/Llama Recipe Manager_0.5.0_amd64.AppImage

A public key has been found, but no private key. Make sure to set `TAURI_SIGNING_PRIVATE_KEY` environment variable.
       Error A public key has been found, but no private key. Make sure to set `TAURI_SIGNING_PRIVATE_KEY` environment variable.
error: script "tauri" exited with code 1
```

#### Why does this happen?
`src-tauri/tauri.conf.json` has `createUpdaterArtifacts: true` and an official `pubkey` for upstream GitHub Releases. After Tauri successfully creates the `.deb` and `.AppImage` packages, it attempts an extra step: creating cryptographically signed `.tar.gz.sig` files for the built-in auto-updater. Because the private key is only held by the upstream repository maintainer, Tauri errors on that final signature step.

#### Is my build broken?
**No.** Notice the lines immediately before the error:
`Finished 3 bundles at: ...`
The `.deb` installer and `.AppImage` are already 100% built, optimized, and ready to install!

#### What to do:
1. **Install the package that was just built:**
   ```bash
   sudo dpkg -i "src-tauri/target/release/bundle/deb/Llama Recipe Manager_0.5.1_amd64.deb"
   ```
2. **Or avoid the error on future builds by disabling updater artifacts:**
   ```bash
   bun run tauri build -c '{"bundle": {"createUpdaterArtifacts": false}}'
   ```
   *(Or `npx tauri build -c '{"bundle": {"createUpdaterArtifacts": false}}'`)*
