#!/usr/bin/env sh
# Builds the Masteraid CLAP and VST3 plugins on Linux and packages them for:
#   - Universal (.tar.gz with ./install.sh)
#   - Debian / Ubuntu (.deb)
#   - Arch Linux (.pkg.tar.zst and PKGBUILD)
#   - Fedora / RHEL (.rpm)
#
# Run this on the OLDEST glibc you intend to support (Ubuntu 22.04, glibc 2.35).
# Binaries built there run on newer distributions (Arch, Fedora 39/40, Debian 12).
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$ROOT"

SKIP_BUILD=0
if [ "${1:-}" = "--skip-build" ]; then
    SKIP_BUILD=1
fi

if [ "$(uname -s)" != "Linux" ]; then
    echo "This script must run on Linux." >&2
    exit 1
fi

for cmd in cargo pkg-config cc; do
    if ! command -v "$cmd" >/dev/null 2>&1; then
        echo "Missing build tool: $cmd" >&2
        echo "Install the dependencies listed in docker/linux-build.Dockerfile." >&2
        exit 1
    fi
done

GLIBC=$(getconf GNU_LIBC_VERSION 2>/dev/null | awk '{print $2}')
if [ -n "$GLIBC" ]; then
    OLDEST=$(printf '%s\n%s\n' "2.35" "$GLIBC" | sort -V | head -n 1)
    if [ "$OLDEST" != "2.35" ]; then
        echo "warning: this host has glibc $GLIBC; the package may not run on Ubuntu 22.04." >&2
        echo "         Use scripts/build-linux-docker.sh or the CI for release builds." >&2
    fi
fi

case "$(uname -m)" in
    x86_64 | amd64)
        VST3_ARCH="x86_64-linux"
        PKG_ARCH="x86_64"
        DEB_ARCH="amd64"
        ;;
    aarch64 | arm64)
        VST3_ARCH="aarch64-linux"
        PKG_ARCH="aarch64"
        DEB_ARCH="arm64"
        ;;
    *)
        echo "Unsupported architecture: $(uname -m)" >&2
        exit 1
        ;;
esac

if [ "$SKIP_BUILD" -eq 0 ]; then
    echo "Building CLAP and VST3 bundles..."
    cargo xtask bundle masteraid --release
fi

VERSION=$(sed -n 's/^version[[:space:]]*=[[:space:]]*"\([^"]*\)".*/\1/p' Cargo.toml | head -n 1)
if [ -z "$VERSION" ]; then
    echo "Could not read the package version from Cargo.toml." >&2
    exit 1
fi

BUNDLED="$ROOT/target/bundled"
CLAP_SOURCE="$BUNDLED/Masteraid.clap"
VST3_SOURCE="$BUNDLED/Masteraid.vst3/Contents/$VST3_ARCH/Masteraid.so"

for source in "$CLAP_SOURCE" "$VST3_SOURCE"; do
    if [ ! -f "$source" ]; then
        echo "Missing build artifact: $source" >&2
        echo "Run without --skip-build first." >&2
        exit 1
    fi
done

PKG="MQN-Masteraid-$VERSION-linux-$PKG_ARCH"
STAGE="$ROOT/dist/$PKG"
ARCHIVE="$ROOT/dist/$PKG.tar.gz"

rm -rf "$STAGE"
rm -f "$ARCHIVE" "$ARCHIVE.sha256"
mkdir -p "$STAGE/Masteraid.vst3/Contents/$VST3_ARCH"

cp "$CLAP_SOURCE" "$STAGE/Masteraid.clap"
cp "$VST3_SOURCE" "$STAGE/Masteraid.vst3/Contents/$VST3_ARCH/Masteraid.so"
if [ -f "$ROOT/assets/Masteraid_User_Manual.pdf" ]; then
    cp "$ROOT/assets/Masteraid_User_Manual.pdf" "$STAGE/"
fi

cat > "$STAGE/README.txt" <<EOF
MQN Masteraid $VERSION (Linux $PKG_ARCH)

Contents
  Masteraid.clap                              - CLAP plugin
  Masteraid.vst3/Contents/$VST3_ARCH/...      - VST3 plugin bundle
  Masteraid_User_Manual.pdf                   - Comprehensive User Manual

Install
  Run ./install.sh to copy both formats into your user plugin folders
  (~/.clap and ~/.vst3), or copy them manually:

    Masteraid.clap  ->  ~/.clap/
    Masteraid.vst3  ->  ~/.vst3/

  For system-wide install (Debian/Arch/Fedora):
    sudo install -Dm755 Masteraid.clap /usr/lib/clap/Masteraid.clap
    sudo cp -R Masteraid.vst3 /usr/lib/vst3/

  Rescan plugins in your host afterwards.
EOF

cp "$ROOT/scripts/install.sh" "$STAGE/install.sh"
chmod +x "$STAGE/install.sh"

# 1. Universal tarball
tar -czf "$ARCHIVE" -C "$ROOT/dist" "$PKG"
(
    cd "$ROOT/dist"
    sha256sum "$PKG.tar.gz" > "$PKG.tar.gz.sha256"
)
echo "Universal Tarball: $ARCHIVE"

# -------------------------------------------------------------------
# 2. Debian / Ubuntu Package (.deb)
# -------------------------------------------------------------------
DEB_DIR="$ROOT/dist/deb_build"
rm -rf "$DEB_DIR"
mkdir -p "$DEB_DIR/DEBIAN"
mkdir -p "$DEB_DIR/usr/lib/clap"
mkdir -p "$DEB_DIR/usr/lib/vst3"
mkdir -p "$DEB_DIR/usr/share/doc/masteraid"

cp "$CLAP_SOURCE" "$DEB_DIR/usr/lib/clap/"
cp -R "$STAGE/Masteraid.vst3" "$DEB_DIR/usr/lib/vst3/"
cp "$STAGE/README.txt" "$DEB_DIR/usr/share/doc/masteraid/"
if [ -f "$ROOT/assets/Masteraid_User_Manual.pdf" ]; then
    cp "$ROOT/assets/Masteraid_User_Manual.pdf" "$DEB_DIR/usr/share/doc/masteraid/"
fi

INSTALLED_SIZE=$(du -sk "$DEB_DIR" | cut -f1)

cat > "$DEB_DIR/DEBIAN/control" <<EOF
Package: masteraid
Version: $VERSION
Section: sound
Priority: optional
Architecture: $DEB_ARCH
Installed-Size: $INSTALLED_SIZE
Depends: libc6 (>= 2.35), libx11-6, libxcb1, libxcursor1, libgl1
Maintainer: MQN DSP <contact@mqndsp.com>
Homepage: https://github.com/xxlintux/mqndsp-masteraid
Description: Vintage Analog Mastering Chain plugin (CLAP and VST3)
 Masteraid is a mastering chain plugin featuring analog tape saturation,
 mastering clipper, 4-band Massive Passive EQ with continuous proportional-Q,
 vintage dynamics (Fairchild / Variable-Mu), true-peak limiter, and dual
 precision metering.
EOF

if command -v dpkg-deb >/dev/null 2>&1; then
    DEB_OUT="$ROOT/dist/masteraid_${VERSION}_${DEB_ARCH}.deb"
    dpkg-deb --build --root-owner-group "$DEB_DIR" "$DEB_OUT"
    ( cd "$ROOT/dist" && sha256sum "$(basename "$DEB_OUT")" > "$(basename "$DEB_OUT").sha256" )
    echo "Debian Package:    $DEB_OUT"
    rm -rf "$DEB_DIR"
fi

# -------------------------------------------------------------------
# 3. Arch Linux Package (.pkg.tar.zst)
# -------------------------------------------------------------------
ARCH_DIR="$ROOT/dist/arch_build"
rm -rf "$ARCH_DIR"
mkdir -p "$ARCH_DIR/usr/lib/clap"
mkdir -p "$ARCH_DIR/usr/lib/vst3"
mkdir -p "$ARCH_DIR/usr/share/doc/masteraid"

cp "$CLAP_SOURCE" "$ARCH_DIR/usr/lib/clap/"
cp -R "$STAGE/Masteraid.vst3" "$ARCH_DIR/usr/lib/vst3/"
cp "$STAGE/README.txt" "$ARCH_DIR/usr/share/doc/masteraid/"
if [ -f "$ROOT/assets/Masteraid_User_Manual.pdf" ]; then
    cp "$ROOT/assets/Masteraid_User_Manual.pdf" "$ARCH_DIR/usr/share/doc/masteraid/"
fi

TOTAL_BYTES=$(du -sb "$ARCH_DIR" 2>/dev/null | cut -f1 || du -sk "$ARCH_DIR" | awk '{print $1*1024}')
BUILD_DATE=$(date +%s)

cat > "$ARCH_DIR/.PKGINFO" <<EOF
pkgname = masteraid
pkgbase = masteraid
pkgver = ${VERSION}-1
pkgdesc = Vintage Analog Mastering Chain plugin (CLAP and VST3)
url = https://github.com/xxlintux/mqndsp-masteraid
builddate = $BUILD_DATE
packager = MQN DSP <contact@mqndsp.com>
size = $TOTAL_BYTES
arch = $PKG_ARCH
license = Proprietary
depend = glibc>=2.35
depend = libx11
depend = libxcb
depend = libxcursor
depend = mesa
provides = masteraid-clap
provides = masteraid-vst3
EOF

if command -v zstd >/dev/null 2>&1; then
    ARCH_PKG_OUT="$ROOT/dist/masteraid-${VERSION}-1-${PKG_ARCH}.pkg.tar.zst"
    (
        cd "$ARCH_DIR"
        tar -cf - .PKGINFO usr | zstd -T0 -19 > "$ARCH_PKG_OUT"
    )
    ( cd "$ROOT/dist" && sha256sum "$(basename "$ARCH_PKG_OUT")" > "$(basename "$ARCH_PKG_OUT").sha256" )
    echo "Arch Package:      $ARCH_PKG_OUT"
    rm -rf "$ARCH_DIR"
fi

# -------------------------------------------------------------------
# 4. Fedora / RHEL Package (.rpm)
# -------------------------------------------------------------------
RPM_TOP="$ROOT/dist/rpm_top"
rm -rf "$RPM_TOP"
mkdir -p "$RPM_TOP/BUILD" "$RPM_TOP/RPMS" "$RPM_TOP/SOURCES" "$RPM_TOP/SPECS" "$RPM_TOP/SRPMS"

cp "$CLAP_SOURCE" "$RPM_TOP/SOURCES/"
cp -R "$STAGE/Masteraid.vst3" "$RPM_TOP/SOURCES/"
cp "$STAGE/README.txt" "$RPM_TOP/SOURCES/"
if [ -f "$ROOT/assets/Masteraid_User_Manual.pdf" ]; then
    cp "$ROOT/assets/Masteraid_User_Manual.pdf" "$RPM_TOP/SOURCES/"
fi

if [ -f "$ROOT/packaging/fedora/masteraid.spec" ]; then
    cp "$ROOT/packaging/fedora/masteraid.spec" "$RPM_TOP/SPECS/"
    if command -v rpmbuild >/dev/null 2>&1; then
        rpmbuild --define "_topdir $RPM_TOP" -bb "$RPM_TOP/SPECS/masteraid.spec"
        find "$RPM_TOP/RPMS" -name "*.rpm" -exec cp {} "$ROOT/dist/" \;
        RPM_OUT=$(find "$ROOT/dist" -maxdepth 1 -name "*.rpm" | head -n 1)
        if [ -n "$RPM_OUT" ]; then
            ( cd "$ROOT/dist" && sha256sum "$(basename "$RPM_OUT")" > "$(basename "$RPM_OUT").sha256" )
            echo "Fedora Package:    $RPM_OUT"
        fi
        rm -rf "$RPM_TOP"
    fi
fi

echo ""
echo "All distribution packages ready in dist/:"
ls -lh "$ROOT/dist"
