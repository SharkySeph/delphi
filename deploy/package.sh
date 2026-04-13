#!/usr/bin/env bash
# ╔══════════════════════════════════════════════════╗
# ║  DELPHI — PACKAGE BUILD SYSTEM                   ║
# ║  Builds .deb, .rpm, .tar.gz, PKGBUILD            ║
# ╚══════════════════════════════════════════════════╝
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

# ── Extract version & arch ───────────────────────────────
VERSION=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/')
ARCH=$(uname -m)
case "$ARCH" in
    x86_64)  DEB_ARCH="amd64"; RPM_ARCH="x86_64" ;;
    aarch64) DEB_ARCH="arm64";  RPM_ARCH="aarch64" ;;
    armv7l)  DEB_ARCH="armhf";  RPM_ARCH="armv7hl" ;;
    *)       DEB_ARCH="$ARCH";  RPM_ARCH="$ARCH" ;;
esac

PACKAGE="delphi"
CLI_BIN="target/release/delphi"
GUI_BIN="target/release/delphi-studio"
ASSETS="crates/delphi-gui/assets"
OUT_DIR="dist"

echo "╔══════════════════════════════════════════════════╗"
echo "║  DELPHI PACKAGE BUILD v${VERSION}"
echo "╚══════════════════════════════════════════════════╝"
echo ""

# ── [1/5] Build release binaries ────────────────────────
echo "[1/5] Building release binaries..."
cargo build --release -p delphi-cli -p delphi-gui

for bin in "$CLI_BIN" "$GUI_BIN"; do
    if [ ! -f "$bin" ]; then
        echo "ERROR: Binary not found at $bin"
        exit 1
    fi
    echo "       $(basename "$bin"): $(du -h "$bin" | cut -f1)"
done

mkdir -p "$OUT_DIR"

# ── [2/5] .tar.gz (portable) ───────────────────────────
build_tarball() {
    echo "[2/5] Building portable tarball..."
    local TARBALL_NAME="${PACKAGE}-${VERSION}-linux-${ARCH}"
    local STAGING="/tmp/${TARBALL_NAME}"
    rm -rf "$STAGING"
    mkdir -p "$STAGING"

    cp "$CLI_BIN" "$GUI_BIN" "$STAGING/"
    cp "$ASSETS/delphi-studio.desktop" "$STAGING/"
    cp README.md "$STAGING/"
    [ -f LICENSE ] && cp LICENSE "$STAGING/" || true
    mkdir -p "$STAGING/assets"
    cp "$ASSETS"/icon*.png "$STAGING/assets/" 2>/dev/null || true
    cp "$ASSETS"/icon.svg  "$STAGING/assets/" 2>/dev/null || true

    if [ -d examples ]; then
        mkdir -p "$STAGING/examples"
        cp examples/*.delphi examples/*.dstudio "$STAGING/examples/" 2>/dev/null || true
    fi

    tar -czf "$OUT_DIR/${TARBALL_NAME}.tar.gz" -C /tmp "$TARBALL_NAME"
    rm -rf "$STAGING"
    echo "       → $OUT_DIR/${TARBALL_NAME}.tar.gz"
}

# ── [3/5] .deb (Debian / Ubuntu / Mint / Pop!_OS) ──────
build_deb() {
    echo "[3/5] Building .deb package..."
    if ! command -v dpkg-deb &>/dev/null; then
        echo "       SKIP: dpkg-deb not found"
        return
    fi

    local DEB_NAME="${PACKAGE}_${VERSION}_${DEB_ARCH}"
    local DEB_ROOT="/tmp/${DEB_NAME}"
    rm -rf "$DEB_ROOT"

    # Directory structure
    mkdir -p "$DEB_ROOT/DEBIAN"
    mkdir -p "$DEB_ROOT/usr/bin"
    mkdir -p "$DEB_ROOT/usr/share/applications"
    mkdir -p "$DEB_ROOT/usr/share/icons/hicolor/256x256/apps"
    mkdir -p "$DEB_ROOT/usr/share/icons/hicolor/64x64/apps"
    mkdir -p "$DEB_ROOT/usr/share/icons/hicolor/32x32/apps"
    mkdir -p "$DEB_ROOT/usr/share/icons/hicolor/scalable/apps"
    mkdir -p "$DEB_ROOT/usr/share/doc/$PACKAGE"

    # Binaries
    install -m 755 "$CLI_BIN" "$DEB_ROOT/usr/bin/delphi"
    install -m 755 "$GUI_BIN" "$DEB_ROOT/usr/bin/delphi-studio"

    # Desktop file
    install -m 644 "$ASSETS/delphi-studio.desktop" \
        "$DEB_ROOT/usr/share/applications/delphi-studio.desktop"

    # Icons
    [ -f "$ASSETS/icon.png" ]    && cp "$ASSETS/icon.png"    "$DEB_ROOT/usr/share/icons/hicolor/256x256/apps/delphi-studio.png"
    [ -f "$ASSETS/icon-64.png" ] && cp "$ASSETS/icon-64.png" "$DEB_ROOT/usr/share/icons/hicolor/64x64/apps/delphi-studio.png"
    [ -f "$ASSETS/icon-32.png" ] && cp "$ASSETS/icon-32.png" "$DEB_ROOT/usr/share/icons/hicolor/32x32/apps/delphi-studio.png"
    [ -f "$ASSETS/icon.svg" ]    && cp "$ASSETS/icon.svg"    "$DEB_ROOT/usr/share/icons/hicolor/scalable/apps/delphi-studio.svg"

    # Docs
    cp README.md "$DEB_ROOT/usr/share/doc/$PACKAGE/"
    [ -f LICENSE ] && cp LICENSE "$DEB_ROOT/usr/share/doc/$PACKAGE/" || true

    # Installed size (KiB)
    local INSTALLED_SIZE
    INSTALLED_SIZE=$(du -sk "$DEB_ROOT" | cut -f1)

    # Control file
    cat > "$DEB_ROOT/DEBIAN/control" << CTRL
Package: ${PACKAGE}
Version: ${VERSION}
Section: sound
Priority: optional
Architecture: ${DEB_ARCH}
Installed-Size: ${INSTALLED_SIZE}
Depends: libc6 (>= 2.31), libgcc-s1, libasound2, libgl1, libxkbcommon0
Recommends: soundfont-fluid
Maintainer: SharkySeph <SharkySeph@users.noreply.github.com>
Homepage: https://github.com/SharkySeph/delphi
Description: Delphi — music composition and notation
 Delphi is a music composition toolkit with a custom notation language.
 Includes the delphi CLI (export, play, info) and delphi-studio GUI
 (multi-track editor, piano roll, mixer, SoundFont playback, MIDI/WAV export).
CTRL

    # Post-install: update icon cache
    cat > "$DEB_ROOT/DEBIAN/postinst" << 'POSTINST'
#!/bin/sh
set -e
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -f /usr/share/icons/hicolor 2>/dev/null || true
fi
if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database /usr/share/applications 2>/dev/null || true
fi
POSTINST
    chmod 755 "$DEB_ROOT/DEBIAN/postinst"

    # Post-remove: clean icon cache
    cat > "$DEB_ROOT/DEBIAN/postrm" << 'POSTRM'
#!/bin/sh
set -e
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -f /usr/share/icons/hicolor 2>/dev/null || true
fi
if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database /usr/share/applications 2>/dev/null || true
fi
POSTRM
    chmod 755 "$DEB_ROOT/DEBIAN/postrm"

    dpkg-deb --build --root-owner-group "$DEB_ROOT" "$OUT_DIR/${DEB_NAME}.deb"
    rm -rf "$DEB_ROOT"
    echo "       → $OUT_DIR/${DEB_NAME}.deb"
}

# ── [4/5] .rpm (Fedora / RHEL / openSUSE) ──────────────
build_rpm() {
    echo "[4/5] Building .rpm package..."
    if ! command -v rpmbuild &>/dev/null; then
        echo "       SKIP: rpmbuild not found (install rpm-build)"
        return
    fi

    local RPM_BUILD="/tmp/rpmbuild-delphi"
    rm -rf "$RPM_BUILD"
    mkdir -p "$RPM_BUILD"/{BUILD,RPMS,SOURCES,SPECS,SRPMS}

    # Create tarball source
    local SRC_DIR="${PACKAGE}-${VERSION}"
    local SRC_STAGING="/tmp/${SRC_DIR}"
    rm -rf "$SRC_STAGING"
    mkdir -p "$SRC_STAGING"
    cp "$CLI_BIN" "$GUI_BIN" "$SRC_STAGING/"
    cp "$ASSETS/delphi-studio.desktop" "$SRC_STAGING/"
    cp README.md "$SRC_STAGING/"
    [ -f LICENSE ] && cp LICENSE "$SRC_STAGING/" || true
    mkdir -p "$SRC_STAGING/assets"
    cp "$ASSETS"/icon*.png "$SRC_STAGING/assets/" 2>/dev/null || true
    cp "$ASSETS"/icon.svg  "$SRC_STAGING/assets/" 2>/dev/null || true
    tar -czf "$RPM_BUILD/SOURCES/${SRC_DIR}.tar.gz" -C /tmp "$SRC_DIR"
    rm -rf "$SRC_STAGING"

    # Spec file
    cat > "$RPM_BUILD/SPECS/delphi.spec" << SPEC
Name:           ${PACKAGE}
Version:        ${VERSION}
Release:        1%{?dist}
Summary:        Delphi — music composition and notation
License:        MIT
URL:            https://github.com/SharkySeph/delphi
Source0:        %{name}-%{version}.tar.gz

Requires:       glibc libgcc alsa-lib mesa-libGL libxkbcommon

%description
Delphi is a music composition toolkit with a custom notation language.
Includes the delphi CLI (export, play, info) and delphi-studio GUI
(multi-track editor, piano roll, mixer, SoundFont playback, MIDI/WAV export).

%prep
%setup -q

%install
install -Dm755 delphi        %{buildroot}/usr/bin/delphi
install -Dm755 delphi-studio %{buildroot}/usr/bin/delphi-studio
install -Dm644 delphi-studio.desktop %{buildroot}/usr/share/applications/delphi-studio.desktop
[ -f assets/icon.png ]    && install -Dm644 assets/icon.png    %{buildroot}/usr/share/icons/hicolor/256x256/apps/delphi-studio.png
[ -f assets/icon-64.png ] && install -Dm644 assets/icon-64.png %{buildroot}/usr/share/icons/hicolor/64x64/apps/delphi-studio.png
[ -f assets/icon-32.png ] && install -Dm644 assets/icon-32.png %{buildroot}/usr/share/icons/hicolor/32x32/apps/delphi-studio.png
[ -f assets/icon.svg ]    && install -Dm644 assets/icon.svg    %{buildroot}/usr/share/icons/hicolor/scalable/apps/delphi-studio.svg

%files
/usr/bin/delphi
/usr/bin/delphi-studio
/usr/share/applications/delphi-studio.desktop
/usr/share/icons/hicolor/*/apps/delphi-studio.*

%post
gtk-update-icon-cache -f /usr/share/icons/hicolor 2>/dev/null || true
update-desktop-database /usr/share/applications 2>/dev/null || true

%postun
gtk-update-icon-cache -f /usr/share/icons/hicolor 2>/dev/null || true
update-desktop-database /usr/share/applications 2>/dev/null || true
SPEC

    rpmbuild --define "_topdir $RPM_BUILD" -bb "$RPM_BUILD/SPECS/delphi.spec"
    find "$RPM_BUILD/RPMS" -name "*.rpm" -exec cp {} "$OUT_DIR/" \;
    rm -rf "$RPM_BUILD"
    echo "       → $OUT_DIR/*.rpm"
}

# ── [5/5] PKGBUILD (Arch / Manjaro / EndeavourOS) ──────
build_pkgbuild() {
    echo "[5/5] Generating Arch PKGBUILD..."
    local PKGBUILD_DIR="$OUT_DIR/arch"
    mkdir -p "$PKGBUILD_DIR"

    cat > "$PKGBUILD_DIR/PKGBUILD" << 'PKGBUILD'
# Maintainer: SharkySeph <SharkySeph@users.noreply.github.com>
pkgname=delphi
pkgver=_VERSION_
pkgrel=1
pkgdesc="Delphi — music composition and notation"
arch=('x86_64' 'aarch64')
url="https://github.com/SharkySeph/delphi"
license=('MIT')
depends=('gcc-libs' 'glibc' 'alsa-lib' 'libgl' 'libxkbcommon')
optdepends=('soundfont-fluid: General MIDI SoundFont for playback')
makedepends=('cargo')
source=("$pkgname-$pkgver.tar.gz::https://github.com/SharkySeph/delphi/archive/v$pkgver.tar.gz")
sha256sums=('SKIP')

build() {
    cd "delphi-$pkgver"
    cargo build --release -p delphi-cli -p delphi-gui --locked
}

package() {
    cd "delphi-$pkgver"
    install -Dm755 "target/release/delphi"        "$pkgdir/usr/bin/delphi"
    install -Dm755 "target/release/delphi-studio"  "$pkgdir/usr/bin/delphi-studio"
    install -Dm644 "crates/delphi-gui/assets/delphi-studio.desktop" "$pkgdir/usr/share/applications/delphi-studio.desktop"
    install -Dm644 "crates/delphi-gui/assets/icon.png"    "$pkgdir/usr/share/icons/hicolor/256x256/apps/delphi-studio.png"
    install -Dm644 "crates/delphi-gui/assets/icon-64.png" "$pkgdir/usr/share/icons/hicolor/64x64/apps/delphi-studio.png"
    install -Dm644 "crates/delphi-gui/assets/icon-32.png" "$pkgdir/usr/share/icons/hicolor/32x32/apps/delphi-studio.png"
    [ -f "crates/delphi-gui/assets/icon.svg" ] && install -Dm644 "crates/delphi-gui/assets/icon.svg" "$pkgdir/usr/share/icons/hicolor/scalable/apps/delphi-studio.svg"
}
PKGBUILD

    sed -i "s/_VERSION_/${VERSION}/" "$PKGBUILD_DIR/PKGBUILD"
    echo "       → $PKGBUILD_DIR/PKGBUILD"
}

# ── Run all builders ────────────────────────────────────
echo ""
build_tarball
build_deb
build_rpm
build_pkgbuild

echo ""
echo "══════════════════════════════════════════════════"
echo "  BUILD COMPLETE — v${VERSION}"
echo ""
echo "  Artifacts in $OUT_DIR/:"
ls -lh "$OUT_DIR/" 2>/dev/null | grep -v "^total"
echo ""
echo "  Install commands:"
echo "    .deb:     sudo dpkg -i $OUT_DIR/${PACKAGE}_${VERSION}_${DEB_ARCH}.deb"
echo "    .rpm:     sudo rpm -i $OUT_DIR/${PACKAGE}-${VERSION}*.rpm"
echo "    tarball:  tar xzf $OUT_DIR/${PACKAGE}-${VERSION}-linux-${ARCH}.tar.gz && sudo cp ${PACKAGE}-*/delphi ${PACKAGE}-*/delphi-studio /usr/local/bin/"
echo "    Arch:     cd $OUT_DIR/arch && makepkg -si"
echo "══════════════════════════════════════════════════"
