# ClipVault

A linux clipboard history for text & images, built with **Rust** + **egui/eframe**.
There are known compatibility issues regarding the `eframe` framework and `Windows`, support has been temporarily suspended for `Windows`.   

ClipVault lives in your **system tray**, supports a global hotkey [Super+V] to toggle the window, and **encrypts your history at rest** using an Argon2-derived key and **XChaCha20-Poly1305**.


## How it looks

### Unlocking it
![alt text](https://raw.githubusercontent.com/AndreiVladescu/ClipVault/refs/heads/master/img/unlock_clipvault.png)

### Main window after unlocking it
![alt text](https://raw.githubusercontent.com/AndreiVladescu/ClipVault/refs/heads/master/img/main_clipvault.png)

### Additional options panel
![alt text](https://raw.githubusercontent.com/AndreiVladescu/ClipVault/refs/heads/master/img/additional_options.png)

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
#### Fedora (and RHEL clones)
```
sudo dnf install \
  gtk3 \
  glib2 \
  pango \
  gdk-pixbuf2 \
  atk \
  cairo \
  libX11 \
  libxkbcommon \
  xdotool-libs \
  libXi libXtst libXinerama libXcursor libXrandr libXrender \
  wayland-libs-client
```
#### Arch/Manjaro
```
sudo pacman -S --needed \
  gtk3 glib2 pango gdk-pixbuf2 atk cairo \
  libx11 libxkbcommon xdotool \
  libxi libxtst libxinerama libxcursor libxrandr libxrender \
  wayland
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
#### Fedora
```
sudo dnf groupinstall "Development Tools"
sudo dnf install clang pkgconf-pkg-config rustup
sudo dnf install \
  gtk3-devel glib2-devel pango-devel gdk-pixbuf2-devel \
  atk-devel cairo-devel libX11-devel libxkbcommon-devel libxdo-devel
sudo dnf install libappindicator-gtk3-devel || sudo dnf install libayatana-appindicator-gtk3
rustup default stable
```
#### Arch/Manjaro
```
sudo pacman -S --needed base-devel clang pkgconf rustup
sudo pacman -S --needed \
  gtk3 glib2 pango gdk-pixbuf2 atk cairo \
  libx11 libxkbcommon xdotool \
  libappindicator-gtk3
rustup default stable
```

#### Building it

```cargo build --release```
