# Plugins (API v1)

> **Status: implemented in the host (0.1, experimental).** The AIDL below is final for v1 and lives in `sdk/plugin-api/aidl/`. The host binds owner-enabled plugins and shows their widgets, media and navigation. A worked example is in [plugins/example-sysinfo](../plugins/example-sysinfo/README.md).

A plugin is an ordinary Android APK. It publishes data into named slots on the dash. The dash shows that data through the host's own screens and theme contract. Plugins are display-only: there is no method in the API that can command the vehicle.

## Declaring a plugin

In the plugin's `AndroidManifest.xml`:

```xml
<service android:name=".DashService" android:exported="true">
    <intent-filter>
        <action android:name="dev.olatool.dash.plugin.PLUGIN" />
    </intent-filter>
    <meta-data android:name="dev.olatool.dash.plugin.slots" android:value="widgets,media,navigation" />
</service>
```

- The service action is `dev.olatool.dash.plugin.PLUGIN`.
- The meta-data `dev.olatool.dash.plugin.slots` is a comma-separated list of the slots the plugin uses: `widgets`, `media`, `navigation`.
- The service implements `IDashPlugin`.

## Owner enablement

While parked, open Settings → Plugins to turn installed plugins Off or On; changes bind or unbind immediately. Installing an APK is not enough. On enables every declared slot unless an existing enablement line restricts it; that restriction is preserved. Off removes the plugin's enablement lines.

The enablement file is `files/plugins/enabled` in the host's private data directory (`/data/data/dev.olatool.dash/files/plugins/enabled`). One plugin per line:

```
# package                       -> every slot the plugin declares
dev.example.dash.weather
# package:slot,slot             -> only the listed slots (must also be declared)
dev.example.dash.nav:navigation
```

On a debug build, `run-as` writes the file as the app's own user, so the host can read it:

```sh
adb shell 'run-as dev.olatool.dash sh -c "mkdir -p files/plugins && echo dev.example.dash.weather > files/plugins/enabled"'
```

The host reads the file at start, after a pane toggle, and whenever a package is added, removed or replaced. After editing it through adb, restart the host to apply it. Lines starting with `#` and blank lines are ignored.

### Built-in media source and the listener grant

Now-playing data for the `Media` card comes from the host itself (`StockMediaService`), not from a plugin. Android only shows a media session to an app with notification-listener access, so the owner grants it once:

```sh
adb shell cmd notification allow_listener dev.olatool.dash/dev.olatool.dash.plugin.StockMediaService
```

Without the grant the card stays empty and the controls do nothing. Revoke it with `disallow_listener` and the same component.

When a plugin publishes media, that plugin's data is shown instead, and the most recent publish wins. Play, pause, next and previous go to the same source as the card.

## Limits

| Item | Limit |
|---|---|
| Widgets per plugin | 4 (a fifth new `id` is ignored) |
| Widgets shown in total | 6 (themes typically show 2 or 3) |
| `id` | 32 characters, longer is ignored |
| `title` | 32 characters, cut |
| `value`, `unit`, `distance`, `eta` | 16 characters, cut |
| `media` title and artist, navigation instruction | 48 characters, cut |

Calls from a plugin for a slot it did not declare or was not enabled for are dropped and logged as `plugin <pkg> not enabled for slot ...`. Calls from any uid other than the plugin's own are dropped.

## Interfaces

Both files live in `sdk/plugin-api/aidl/dev/olatool/dash/plugin/`.

### `IDashPlugin` (implemented by the plugin)

| Method | Purpose |
|---|---|
| `oneway void attach(IDashHost host, int hostApiVersion)` | The host is ready. Keep the `IDashHost` and publish through it. `hostApiVersion` is at least 1. |
| `oneway void detach()` | The host is going away. Stop publishing. |
| `oneway void onAction(String action)` | A user action routed to this plugin. |

`onAction` strings:

- `media.play-pause`, `media.next`, `media.previous`: sent when the owner uses the media card and the plugin is enabled for `media`.
- `widget.<id>`: sent when a widget published by the plugin is tapped.

### `IDashHost` (implemented by the host)

| Method | Purpose |
|---|---|
| `int apiVersion()` | The plugin API version the host implements. Returns 1. |
| `oneway void publishWidget(String id, String title, String value, String unit)` | Add or update a value card. `id` is unique within the plugin. |
| `oneway void removeWidget(String id)` | Remove a value card. |
| `oneway void publishMedia(String title, String artist, boolean playing)` | Now-playing info. Shown only for plugins enabled for `media`. |
| `oneway void publishNavigation(boolean active, String instruction, String distance, String eta)` | Turn-by-turn. Shown only for plugins enabled for `navigation`. |

All calls are `oneway`. A plugin should not assume a publish has reached the screen when the call returns.

## Lifecycle

1. The owner enables the plugin. The host binds its service.
2. The host calls `attach(host, hostApiVersion)`.
3. The plugin publishes widgets, media or navigation data through `host`.
4. The host forwards user actions with `onAction`.
5. On unbind or shutdown, the host calls `detach()`. The plugin must stop publishing.

If the plugin process dies or is force-stopped, the host drops everything that plugin published (widgets, media, navigation) and rebinds it after a backoff of 2 s, doubling up to 30 s. The rebind starts the plugin again, and it gets `attach()` once more. The host logs `plugin lost`, `plugin retry` and `plugin attached` under the `calls-*.log` session log. A plugin that is not installed is retried with the same backoff (capped at 30 s), logging `plugin ... retry` each time. The next package broadcast also triggers a rescan, so an install is picked up at once.

The host's `IDashHost` handed to each plugin is its own object, so the host knows which plugin a call came from.

Themes see a widget's `id` as `<package>|<plugin id>`, which the host composes. Treat `Widget.id` as opaque and pass it back to `Plugins.widget-clicked` unchanged.

## Guarantees

- **Display-only.** No method sends a vehicle command, and no theme can call into a plugin except through `Plugins.widget-clicked` and the media and navigation contract fields.
- **No overlay control.** A plugin cannot hide or cover the host safety overlay.
- **Owner-enabled only.** Nothing is bound without owner enablement.
- **Additive changes only.** New methods are added to the end of an interface. Adding one bumps the API version here and in the AIDL comment.

## Versioning

API version: **1**. The host reports it through `IDashHost.apiVersion()`. A plugin built for v1 should check `hostApiVersion` in `attach()` and degrade if it is lower than it needs.

## Worked example (non-normative)

The complete, buildable version is [plugins/example-sysinfo](../plugins/example-sysinfo/README.md). The core of it:

```java
@Override
public void attach(IDashHost host, int hostApiVersion) {
    this.host = host;
    host.publishWidget("range", "Range", "74", "km");
}

@Override
public void onAction(String action) {
    if ("widget.range".equals(action)) {
        // Display-only: refresh the value, never command the vehicle.
    }
}
```
