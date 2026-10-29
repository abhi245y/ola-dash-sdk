#!/usr/bin/env bash
# Builds the interface test harness APK (dev.olatool.dash.ifacetest).
# Run on arlp-pc against a copy of sdk/. Usage: build.sh [out-dir]   (default: test-harness/out)
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
VI="$(cd "$HERE/.." && pwd)"
OUT="${1:-$HERE/out}"
JDK="${JDK_HOME:-$HOME/Android/jdk17}"
SDK="${ANDROID_SDK:-$HOME/Android/Sdk}"
BT="$SDK/build-tools/34.0.0"
PLATFORM="$SDK/platforms/android-34/android.jar"
KS="${KEYSTORE:-$HOME/Android/debug.keystore}"
export JAVA_HOME="$JDK"
export PATH="$JDK/bin:$BT:$PATH"

rm -rf "$OUT"
mkdir -p "$OUT/gen" "$OUT/classes" "$OUT/dex"

# 1. AIDL -> Java stubs and proxies. Parcelable declarations generate nothing.
aidl --lang=java -I "$VI/aidl" -o "$OUT/gen" "$VI"/aidl/com/olaelectric/vehicleinterface/*.aidl

# 2. Java: generated stubs, hand-written parcelables/enums, callback bases, harness sources.
find "$OUT/gen" "$VI/java" "$HERE/src" -name '*.java' > "$OUT/srcs.txt"
javac --release 8 -encoding UTF-8 -Xlint:-options -cp "$PLATFORM" -d "$OUT/classes" @"$OUT/srcs.txt"

# 3. Dex, manifest, sign.
find "$OUT/classes" -name '*.class' > "$OUT/classes.txt"
d8 --release --min-api 29 --lib "$PLATFORM" --output "$OUT/dex" @"$OUT/classes.txt"
aapt2 link -I "$PLATFORM" --manifest "$HERE/AndroidManifest.xml" \
    --min-sdk-version 29 --target-sdk-version 34 --version-code 1 --version-name 0.1.0 \
    -o "$OUT/base.apk"
cp "$OUT/base.apk" "$OUT/unsigned.apk"
(cd "$OUT/dex" && zip -q -j "$OUT/unsigned.apk" classes.dex)
zipalign -p -f 4 "$OUT/unsigned.apk" "$OUT/aligned.apk"
apksigner sign --ks "$KS" --ks-key-alias androiddebugkey \
    --ks-pass pass:android --key-pass pass:android \
    --out "$OUT/ifacetest.apk" "$OUT/aligned.apk"
echo "built $OUT/ifacetest.apk"
