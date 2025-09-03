# ClipVault

A linux clipboard history for text & images, built with **Rust** + **egui/eframe**.
There are known compatibility issues regarding the `eframe` framework and `Windows`, support has been temporarily suspended for `Windows`.   

ClipVault lives in your **system tray**, supports a global hotkey [Super+V] to toggle the window, and **encrypts your history at rest** using an Argon2-derived key and **XChaCha20-Poly1305**.

![Presentation image](https://raw.githubusercontent.com/AndreiVladescu/ClipVault/refs/heads/master/img/presentation.png)

## How it looks

### Unlocking it
![How it's unlocked](https://raw.githubusercontent.com/AndreiVladescu/ClipVault/refs/heads/master/img/unlock_clipvault.png)

### Main window after unlocking it
![Main window](https://raw.githubusercontent.com/AndreiVladescu/ClipVault/refs/heads/master/img/main_clipvault.png)

### Additional options panel
![Options menu](https://raw.githubusercontent.com/AndreiVladescu/ClipVault/refs/heads/master/img/additional_options.png)

## How to use it

### Runtime prerequisites
ClipVault uses `egui` + `gtk3` and links to a few X11/graphics libs.
Install these runtime libraries before running the prebuilt binary:

#### Debian/Ubuntu/Mint
```
sudo apt update
sudo apt install \
  libgtk-3-0 \
  libglib2.0-0 \
  libpango-1.0-0 \
  libgdk-pixbuf-2.0-0 \
  libatk1.0-0 \
  libcairo2 \
  libx11-6 \
  libxkbcommon0 \
  libxdo3 \
  libxi6 libxtst6 libxinerama1 libxcursor1 libxrandr2 libxrender1 \
  libwayland-client0
```
### Build prerequisites

#### Debian/Ubuntu/Mint
```
sudo apt install build-essential curl pkg-config clang
sudo apt install \
  libgtk-3-dev libglib2.0-dev libpango1.0-dev libgdk-pixbuf-2.0-dev \
  libatk1.0-dev libcairo2-dev libx11-dev libxkbcommon-dev libxdo-dev
sudo apt install libayatana-appindicator3-dev || sudo apt install libappindicator3-dev
curl https://sh.rustup.rs -sSf | sh
```

#### Building it

```cargo build --release```
