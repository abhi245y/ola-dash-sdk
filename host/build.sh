#!/usr/bin/env bash
# Builds the debug-signed host APK on a Linux build machine (see docs/building.md):
#   host/build.sh
# Output: host/out/ola-dash-debug.apk
# Release build (non-debuggable, signed with your key from the environment):
#   OLADASH_KEYSTORE=... OLADASH_KEY_ALIAS=... OLADASH_KS_PASS=... OLADASH_KEY_PASS=... host/build.sh --release
# Output: host/out/ola-dash-release.apk
set -euo pipefail

RELEASE=0
case "${1:-}" in
    "") ;;
    --release) RELEASE=1 ;;
    *) echo "usage: host/build.sh [--release]" >&2; exit 2 ;;
esac
if [ "$RELEASE" = 1 ]; then
    for v in OLADASH_KEYSTORE OLADASH_KEY_ALIAS OLADASH_KS_PASS OLADASH_KEY_PASS; do
        [ -n "${!v:-}" ] || { echo "release build: $v is not set (needs OLADASH_KEYSTORE, OLADASH_KEY_ALIAS, OLADASH_KS_PASS, OLADASH_KEY_PASS)" >&2; exit 1; }
    done
    [ -f "$OLADASH_KEYSTORE" ] || { echo "release build: keystore not found: $OLADASH_KEYSTORE" >&2; exit 1; }
fi

HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/.." && pwd)"
SDK="${ANDROID_HOME:-$HOME/Android/Sdk}"
JDK="${JDK_HOME:-$HOME/Android/jdk17}"
NDK="$SDK/ndk/27.2.12479018"
BT="$SDK/build-tools/34.0.0"
PLATFORM="$SDK/platforms/android-34/android.jar"
API=29
TRIPLE=aarch64-linux-android
TC="$NDK/toolchains/llvm/prebuilt/linux-x86_64/bin"
KS="$HOME/Android/debug.keystore"

export JAVA_HOME="$JDK"
export ANDROID_HOME="$SDK"
export ANDROID_NDK_HOME="$NDK"
export PATH="$JDK/bin:$BT:$PATH"
export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$TC/aarch64-linux-android$API-clang"
export CC_aarch64_linux_android="$TC/aarch64-linux-android$API-clang"
export CXX_aarch64_linux_android="$TC/aarch64-linux-android$API-clang++"
export AR_aarch64_linux_android="$TC/llvm-ar"

# Vehicle-service interface: clean-room definitions in sdk/vehicle-interface (see its README).
VI="$ROOT/sdk/vehicle-interface"

B="$HERE/build"
rm -rf "$B"
mkdir -p "$B/classes" "$B/dex" "$B/gen" "$B/assets/contract" "$B/assets/host" "$B/assets/fallback" "$B/assets/last-resort" "$HERE/out"

# 1. Native library (release build, even for the debug APK).
(cd "$HERE" && cargo build --release --target "$TRIPLE")
cp "$HERE/target/$TRIPLE/release/liboladash.so" "$B/liboladash.so"

# 2. Bundled assets, copied to files/ on first start (contract, overlay and host font, fallback, last-resort).
cp "$ROOT/contract/vehicle.slint" "$B/assets/contract/"
cp "$HERE/ui/"*.slint "$B/assets/host/"
cp -r "$HERE/ui/fonts" "$B/assets/host/"
cp -r "$ROOT/themes/default/." "$B/assets/fallback/"       # bundled default theme (fallback tier)
# Optional EXTRA_THEMES=<dir>: each subfolder with a theme.json is bundled as themes/<id>/.
for d in ${EXTRA_THEMES:+"$EXTRA_THEMES"/*/}; do
    [ -f "$d/theme.json" ] || continue
    id=$(basename "$d")
    case $id in active|*[!A-Za-z0-9_-]*) echo "bad theme id: $id" >&2; exit 1;; esac
    mkdir -p "$B/assets/themes"
    cp -r "$d" "$B/assets/themes/$id"
done
cp "$HERE/ui/last-resort/main.slint" "$B/assets/last-resort/"  # placeholder (last-resort tier)

# 3. Java: bridge + vehicle interface + plugin AIDL stubs -> classes.dex.
#    The vehicle-interface callback bases call dev.olatool.dash.Trace, so they share
#    one javac run with the bridge.
# Compile from a copy: aidl without --lang=java can rewrite its inputs in place.
mkdir -p "$B/aidl"
cp -r "$ROOT/sdk/plugin-api/aidl/dev" "$VI/aidl/com" "$B/aidl/"
"$BT/aidl" --lang=java -I "$B/aidl" -o "$B/gen" "$B/aidl/dev/olatool/dash/plugin/"*.aidl
"$BT/aidl" --lang=java -I "$B/aidl" -o "$B/gen" "$B/aidl/com/olaelectric/vehicleinterface/"*.aidl
find "$HERE/java" "$VI/java" "$B/gen" -name '*.java' > "$B/srcs.txt"
javac --release 8 -encoding UTF-8 -Xlint:-options -cp "$PLATFORM" -d "$B/classes" @"$B/srcs.txt"
find "$B/classes" -name '*.class' > "$B/classes.txt"
d8 --release --min-api "$API" --lib "$PLATFORM" --output "$B/dex" @"$B/classes.txt"

# Debug keeps the defaults and --debug-mode (the manifest no longer says debuggable).
# Release: version from the latest v* tag (code = MAJOR*10000+MINOR*100+PATCH), else the defaults.
VCODE=1 VNAME=0.1.0 DEBUGMODE=--debug-mode
if [ "$RELEASE" = 1 ]; then
    DEBUGMODE=
    # OLADASH_VERSION (e.g. v1.2.3) stands in for the tag where .git is not synced (build host).
    if DESC=${OLADASH_VERSION:-$(git -C "$ROOT" describe --tags --match 'v[0-9]*' 2>/dev/null)}; [ -n "$DESC" ]; then
        TAG=${DESC%%-*}; VNAME=${DESC#v}
        IFS=. read -r MA MI PA <<<"${TAG#v}"
        PA=${PA:-0}
        [[ "$MA$MI$PA" =~ ^[0-9]+$ && $MI -lt 100 && $PA -lt 100 ]] || { echo "release build: bad version $DESC" >&2; exit 1; }
        VCODE=$((MA * 10000 + MI * 100 + PA))
    fi
fi

# 4. Manifest + assets -> base APK, then add dex and the native library.
aapt2 link -I "$PLATFORM" --manifest "$HERE/AndroidManifest.xml" -A "$B/assets" \
    --min-sdk-version "$API" --target-sdk-version 34 --version-code "$VCODE" --version-name "$VNAME" --replace-version $DEBUGMODE \
    -o "$B/base.apk"
python3 - "$B/base.apk" "$B/dex/classes.dex" "$B/liboladash.so" "$B/unsigned.apk" <<'PY'
import sys, zipfile
base, dex, so, out = sys.argv[1:5]
with zipfile.ZipFile(base) as src, zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED) as dst:
    for item in src.infolist():
        dst.writestr(item, src.read(item.filename), compress_type=item.compress_type)
    dst.write(dex, "classes.dex")
    dst.write(so, "lib/arm64-v8a/liboladash.so")
PY

# 5. Align and sign: the debug key (created once), or the release key from the environment.
zipalign -p -f 4 "$B/unsigned.apk" "$B/aligned.apk"
if [ "$RELEASE" = 1 ]; then
    OUT="$HERE/out/ola-dash-release.apk"
    apksigner sign --ks "$OLADASH_KEYSTORE" --ks-pass env:OLADASH_KS_PASS --key-pass env:OLADASH_KEY_PASS \
        --ks-key-alias "$OLADASH_KEY_ALIAS" --out "$OUT" "$B/aligned.apk"
    if aapt2 dump badging "$OUT" | grep -q application-debuggable; then
        rm -f "$OUT"; echo "release APK is debuggable; refusing" >&2; exit 1
    fi
    apksigner verify --print-certs "$OUT" | tee "$B/certs.txt"
    echo "signer SHA-256: $(sed -n 's/^Signer #1 certificate SHA-256 digest: //p' "$B/certs.txt")"
else
    OUT="$HERE/out/ola-dash-debug.apk"
    if [ ! -f "$KS" ]; then
        mkdir -p "$(dirname "$KS")"
        keytool -genkeypair -keystore "$KS" -storepass android -keypass android \
            -alias androiddebugkey -dname "CN=Android Debug,O=Android,C=US" \
            -keyalg RSA -keysize 2048 -validity 10000
    fi
    apksigner sign --ks "$KS" --ks-pass pass:android --key-pass pass:android \
        --ks-key-alias androiddebugkey --out "$OUT" "$B/aligned.apk"
    apksigner verify "$OUT"
fi
ls -l "$OUT"
