# example-sysinfo

The canonical example plugin for the Ola Dash SDK. It publishes three value widgets
(free RAM, hottest CPU thermal zone, uptime) every 5 seconds and toggles units when
one of the first two is tapped. Copy this folder to start a plugin of your own.

It is display-only: it reads `/proc/meminfo` and `/sys/class/thermal` and has no
vehicle access. The API it uses is described in [docs/plugins.md](../../docs/plugins.md).

## Anatomy

| File | Role |
|---|---|
| `AndroidManifest.xml` | Declares the service, its action, and the slots it uses. |
| `src/.../SysinfoService.java` | Implements `IDashPlugin` and publishes through `IDashHost`. |
| `build.sh` | Compiles the SDK's AIDL, the plugin, and packages a debug-signed APK. |

### Manifest

```xml
<service android:name=".SysinfoService" android:exported="true">
    <intent-filter>
        <action android:name="dev.olatool.dash.plugin.PLUGIN" />
    </intent-filter>
    <meta-data android:name="dev.olatool.dash.plugin.slots" android:value="widgets" />
</service>
```

- The action is how the host finds the plugin. `exported="true"` is required so the host
  (a different app) can bind it.
- `slots` lists what the plugin publishes: `widgets`, `media` or `navigation`. The host
  ignores calls for any slot not listed here, and for any slot the owner did not enable.

### AIDL

The interfaces are in `sdk/plugin-api/aidl/dev/olatool/dash/plugin/`. `build.sh` runs
`aidl --lang=java` on them, so the plugin needs no copy of the SDK sources in its build.
Keep the package `dev.olatool.dash.plugin` for the generated `IDashPlugin` and `IDashHost`.

### Lifecycle

1. The owner enables the plugin (`files/plugins/enabled`, see the plugin docs). The host binds the service.
2. The host calls `attach(host, hostApiVersion)`. Keep `host` and start publishing.
3. The plugin calls `host.publishWidget(id, title, value, unit)` on every update. Each call
   replaces the widget with that `id`.
4. A tap on a widget arrives as `onAction("widget.<id>")`.
5. The host calls `detach()` when it unbinds (owner disabled the plugin, or the host stopped).
   Stop timers there.

If this plugin crashes, the host drops its widgets and rebinds it with backoff.

### Limits

Up to 4 widgets per plugin. Widget `id` up to 32 characters, `title` up to 32, `value` and `unit`
up to 16 (longer strings are cut). The host shows at most 6 widgets in total.

## Build

Build on a machine with the Android SDK (build-tools 34.0.0, platform android-34, JDK 17) and
the debug key from `host/build.sh`:

```sh
plugins/example-sysinfo/build.sh      # -> plugins/example-sysinfo/out/example-sysinfo-debug.apk
```

## Install and enable

```sh
adb install -r plugins/example-sysinfo/out/example-sysinfo-debug.apk
# Enable it for the host (debug host build; see docs/plugins.md for the file format):
adb shell 'run-as dev.olatool.dash sh -c "mkdir -p files/plugins && echo dev.olatool.dash.plugin.sysinfo > files/plugins/enabled"'
```
