# Building Reeves on Windows

These steps build the Windows installers on a Windows 10 or 11 PC (64-bit,
x64). They produce two files:

- `Reeves_<version>_x64-setup.exe`: a setup program that installs Reeves for the
  current user (no administrator rights needed). Best for researchers
  installing on their own laptop.
- `Reeves_<version>_x64_en-US.msi`: a Windows Installer package, for IT
  departments that deploy software centrally.

Both download Microsoft's WebView2 component during installation if the PC
doesn't already have it (Windows 11 and recent Windows 10 do).

## 1. Install the tools (one time)

1. **Microsoft C++ Build Tools.** Download "Build Tools for Visual Studio" from
   <https://visualstudio.microsoft.com/visual-cpp-build-tools/>. In the
   installer, tick **Desktop development with C++** and install.
2. **Rust.** Download and run `rustup-init.exe` from <https://rustup.rs> and
   accept the defaults. It uses the toolchain installed in step 1.
3. **Node.js 20 or newer.** Install the LTS version from <https://nodejs.org>.
4. **Git.** Install from <https://git-scm.com/download/win> (or download the
   repository as a ZIP from GitHub instead).

Open a new **PowerShell** window after installing so the new tools are on the
path, and check them:

```powershell
rustc --version
cargo --version
node --version
npm --version
```

## 2. Get the code

```powershell
git clone https://github.com/davicompu/Fangio.git
cd Fangio
```

## 3. Build

```powershell
npm install
npm run tauri build
```

The first build takes several minutes. The installers are written to:

```
src-tauri\target\release\bundle\nsis\Reeves_<version>_x64-setup.exe
src-tauri\target\release\bundle\msi\Reeves_<version>_x64_en-US.msi
```

To try the app without installing, run `npm run tauri dev` instead.

## 4. Install and first run

Double-click the setup `.exe` (or the `.msi`).

The installers aren't code-signed yet, so Windows SmartScreen shows a blue
**"Windows protected your PC"** window. Click **More info**, then
**Run anyway**. This appears once per installer version.

Reeves keeps its list of recent projects in `%APPDATA%\carta\recent.json`.
Each project lives in the folder you choose for it, as on the Mac.

For the AI features, install LM Studio for Windows from
<https://lmstudio.ai>, load a chat model and an embedding model, and start the
local server from the Developer tab. Reeves' default address
(`http://127.0.0.1:1234/v1`) is the same on Windows.

## Optional: faster PDF text extraction

Reeves reads PDF text with a built-in extractor. If the `pdftotext` program
(from Poppler) is on the PATH, Reeves uses it instead, which handles more PDFs
well. On Windows you can get it from the "poppler-windows" releases on GitHub;
add its `Library\bin` folder to the PATH. This is optional.

## Troubleshooting

- **`link.exe not found` or `MSVC` errors:** the C++ Build Tools (step 1) are
  missing or the "Desktop development with C++" workload wasn't ticked.
- **The MSI step fails while downloading WiX:** the build downloads WiX the first
  time; check the internet connection and run the build again. To build only the
  setup `.exe`, run `npm run tauri build -- --bundles nsis`.
- **Antivirus slows or blocks the build:** exclude the `src-tauri\target` folder
  from real-time scanning.

## For maintainers: a test setup `.exe` from a Mac

The setup `.exe` (not the `.msi`) can also be cross-built on a Mac, which is
handy for a quick test on Windows. One-time setup:

```sh
brew install llvm lld makensis
cargo install cargo-xwin --locked
rustup target add x86_64-pc-windows-msvc
```

Then:

```sh
export PATH="/opt/homebrew/opt/llvm/bin:/opt/homebrew/opt/lld/bin:$PATH"
npm run tauri build -- --runner cargo-xwin --target x86_64-pc-windows-msvc
```

The installer is written to
`src-tauri/target/x86_64-pc-windows-msvc/release/bundle/nsis/`. Release
installers, including the `.msi`, should still be built on Windows.
