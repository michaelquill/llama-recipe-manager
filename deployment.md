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
Run the Tauri build command:

```bash
bun run tauri build
```
*(Or `npx tauri build` if using npm)*

During the build process:
1. The SvelteKit frontend is compiled into `build/`.
2. Cargo builds the optimized Rust binary with all dependencies.
3. Tauri packages the outputs into `.deb` and `.AppImage` bundles under:
   ```
   src-tauri/target/release/bundle/deb/
   src-tauri/target/release/bundle/appimage/
   src-tauri/target/release/llama-recipe-manager   (standalone binary)
   ```

---

## 5. Install Locally

You have three options to install your build. **Option A is recommended** as it updates your existing system package cleanly.

### Option A: Install `.deb` Package (Recommended)
This directly replaces `/usr/bin/llama-recipe-manager` and refreshes desktop menu icons:

```bash
sudo dpkg -i src-tauri/target/release/bundle/deb/llama-recipe-manager_*_amd64.deb
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
