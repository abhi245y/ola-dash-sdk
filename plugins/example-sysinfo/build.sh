#!/usr/bin/env bash
# Builds the example sysinfo plugin APK (debug-signed). Run on a build machine with the Android SDK
# (same toolchain as host/build.sh). Output: plugins/example-sysinfo/out/example-sysinfo-debug.apk
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
SDK="${ANDROID_HOME:-$HOME/Android/Sdk}"
JDK="${JDK_HOME:-$HOME/Android/jdk17}"
BT="$SDK/build-tools/34.0.0"
PLATFORM="$SDK/platforms/android-34/android.jar"
KS="$HOME/Android/debug.keystore"

export JAVA_HOME="$JDK"
export PATH="$JDK/bin:$BT:$PATH"

B="$HERE/build"
rm -rf "$B"
mkdir -p "$B/aidl" "$B/gen" "$B/classes" "$B/dex" "$HERE/out"

# 1. Plugin API stubs from the SDK. Copy first: aidl without --lang=java rewrites its inputs.
cp -r "$ROOT/sdk/plugin-api/aidl/dev" "$B/aidl/"
aidl --lang=java -I "$B/aidl" -o "$B/gen" "$B/aidl/dev/olatool/dash/plugin/"*.aidl

# 2. Plugin classes and stubs -> classes.dex.
find "$HERE/src" "$B/gen" -name '*.java' > "$B/srcs.txt"
javac --release 8 -encoding UTF-8 -Xlint:-options -cp "$PLATFORM" -d "$B/classes" @"$B/srcs.txt"
find "$B/classes" -name '*.class' > "$B/classes.txt"
d8 --release --min-api 29 --lib "$PLATFORM" --output "$B/dex" @"$B/classes.txt"

# 3. Manifest -> base APK, then add the dex (aapt2 writes no classes).
aapt2 link -I "$PLATFORM" --manifest "$HERE/AndroidManifest.xml" \
    --min-sdk-version 29 --target-sdk-version 34 --version-code 1 --version-name 0.1.0 \
    -o "$B/base.apk"
python3 - "$B/base.apk" "$B/dex/classes.dex" "$B/unsigned.apk" <<'PY'
import sys, zipfile
base, dex, out = sys.argv[1:4]
with zipfile.ZipFile(base) as src, zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED) as dst:
    for item in src.infolist():
        dst.writestr(item, src.read(item.filename), compress_type=item.compress_type)
    dst.write(dex, "classes.dex")
PY

# 4. Align and sign with the debug key (created by host/build.sh; create it there first).
if [ ! -f "$KS" ]; then
    echo "build.sh: $KS missing. Run host/build.sh once to create the debug key." >&2
    exit 1
fi
zipalign -p -f 4 "$B/unsigned.apk" "$B/aligned.apk"
apksigner sign --ks "$KS" --ks-pass pass:android --key-pass pass:android \
    --ks-key-alias androiddebugkey --out "$HERE/out/example-sysinfo-debug.apk" "$B/aligned.apk"
apksigner verify "$HERE/out/example-sysinfo-debug.apk"
ls -l "$HERE/out/example-sysinfo-debug.apk"
