#!/usr/bin/env bash
# Builds the debug-signed host APK on a Linux build machine (see docs/building.md):
#   host/build.sh
# Output: host/out/ola-dash-debug.apk
set -euo pipefail

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

# 2. Bundled assets, copied to files/ on first start (contract, overlay, fallback, last-resort).
cp "$ROOT/contract/vehicle.slint" "$B/assets/contract/"
cp "$HERE/ui/"*.slint "$B/assets/host/"
cp -r "$ROOT/themes/default/." "$B/assets/fallback/"       # bundled default theme (fallback tier)
# Optional EXTRA_THEMES=<dir>: each subfolder with a theme.json is bundled as themes/<id>/.
[ -z "${EXTRA_THEMES:-}" ] || for d in "$EXTRA_THEMES"/*/; do
    [ -f "$d/theme.json" ] && { mkdir -p "$B/assets/themes"; cp -r "$d" "$B/assets/themes/$(basename "$d")"; } || true
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

# 4. Manifest + assets -> base APK, then add dex and the native library.
aapt2 link -I "$PLATFORM" --manifest "$HERE/AndroidManifest.xml" -A "$B/assets" \
    --min-sdk-version "$API" --target-sdk-version 34 --version-code 1 --version-name 0.1.0 \
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

# 5. Align and sign with the debug key (created once).
zipalign -p -f 4 "$B/unsigned.apk" "$B/aligned.apk"
if [ ! -f "$KS" ]; then
    mkdir -p "$(dirname "$KS")"
    keytool -genkeypair -keystore "$KS" -storepass android -keypass android \
        -alias androiddebugkey -dname "CN=Android Debug,O=Android,C=US" \
        -keyalg RSA -keysize 2048 -validity 10000
fi
apksigner sign --ks "$KS" --ks-pass pass:android --key-pass pass:android \
    --ks-key-alias androiddebugkey --out "$HERE/out/ola-dash-debug.apk" "$B/aligned.apk"
apksigner verify "$HERE/out/ola-dash-debug.apk"
ls -l "$HERE/out/ola-dash-debug.apk"
