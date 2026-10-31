package dev.olatool.dash.plugin;

import android.content.BroadcastReceiver;
import android.content.ComponentName;
import android.content.Context;
import android.content.Intent;
import android.content.IntentFilter;
import android.content.ServiceConnection;
import android.content.pm.PackageManager;
import android.content.pm.ResolveInfo;
import android.content.pm.ServiceInfo;
import android.os.Binder;
import android.os.Handler;
import android.os.IBinder;
import android.os.Looper;
import android.os.RemoteException;

import dev.olatool.dash.Trace;

import org.json.JSONArray;
import org.json.JSONException;
import org.json.JSONObject;

import java.io.File;
import java.io.FileOutputStream;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

/**
 * Plugin host (API v1). Binds only the plugins listed in files/plugins/enabled, gives each one its
 * own IDashHost, and pushes one JSON snapshot of widgets, media and navigation to the Rust UI.
 * Everything runs on the main looper. Display-only: nothing here can reach the vehicle.
 */
public final class PluginHost {
    public static final String ACTION = "dev.olatool.dash.plugin.PLUGIN";
    static final String SLOTS_META = "dev.olatool.dash.plugin.slots";
    /** Enablement file, relative to the app's files dir: one "package" or "package:slot,slot" per line. */
    static final String ENABLED = "plugins/enabled";
    static final int API_VERSION = 1;

    // Slot bits. A plugin uses a slot only if it declares it and the owner enabled it.
    static final int WIDGETS = 1, MEDIA = 2, NAVIGATION = 4;
    static final int MAX_WIDGETS = 4, MAX_TOTAL_WIDGETS = 6;
    static final int MAX_ID = 32, MAX_TITLE = 32, MAX_LONG = 48, MAX_SHORT = 16;
    private static final long BACKOFF_MIN_MS = 2000L, BACKOFF_MAX_MS = 30_000L;

    private static native void nativeSetPlugins(String json);

    private static Context app;
    private static final Handler main = new Handler(Looper.getMainLooper());
    private static final List<Entry> entries = new ArrayList<>();
    private static BroadcastReceiver packages;
    private static boolean pushQueued;
    private static long seq;
    private static MediaInfo stock; // built-in source, from StockMediaService
    private static volatile boolean stockPlaying;

    private PluginHost() {
    }

    /** Main thread. Call from MainActivity.onCreate, after VehicleBridge.start. */
    public static void start(Context ctx) {
        app = ctx.getApplicationContext();
        packages = new BroadcastReceiver() {
            @Override
            public void onReceive(Context c, Intent i) {
                refresh();
            }
        };
        IntentFilter f = new IntentFilter(Intent.ACTION_PACKAGE_ADDED);
        f.addAction(Intent.ACTION_PACKAGE_REMOVED);
        f.addAction(Intent.ACTION_PACKAGE_REPLACED);
        f.addDataScheme("package");
        app.registerReceiver(packages, f);
        refresh();
    }

    /** Main thread. Unbinds every plugin. */
    public static void stop() {
        if (app == null) {
            return;
        }
        app.unregisterReceiver(packages);
        for (Entry e : new ArrayList<>(entries)) {
            remove(e);
        }
        app = null;
    }

    /** True while the built-in source reports a playing session. Read by the keep-alive thread. */
    public static boolean stockPlaying() {
        return stockPlaying;
    }

    /** Main thread. Called by MainActivity for media, widget and host refresh actions. */
    public static void action(String action) {
        if (app == null || action == null) {
            return;
        }
        if (action.equals("plugins.refresh")) {
            refresh();
        } else if (action.startsWith("media.")) {
            MediaInfo m = displayMedia();
            if (m != null && m.owner != null) {
                sendAction(m.owner, action);
            } else {
                StockMediaService.control(action);
            }
        } else if (action.startsWith("widget.")) {
            // Key is "<package>|<widget id>", built in push(). Forward "widget.<id>" to its owner.
            String key = action.substring("widget.".length());
            int bar = key.indexOf('|');
            Entry e = bar < 0 ? null : find(key.substring(0, bar));
            String id = bar < 0 ? "" : key.substring(bar + 1);
            if (e != null && e.bound && e.widgets.containsKey(id)) {
                sendAction(e, "widget." + id);
            }
        }
    }

    /** Main thread. Built-in source update; title null means no session. */
    static void setStockMedia(String title, String artist, boolean playing) {
        stock = title == null ? null : new MediaInfo(clip(title, MAX_LONG), clip(artist, MAX_LONG), playing, ++seq, null);
        schedulePush();
    }

    // ---- enablement and binding -------------------------------------------------------------

    /** Re-reads the enablement file and reconciles entries with it. Main thread. */
    private static void refresh() {
        if (app == null) {
            return;
        }
        writeInstalled();
        Map<String, String> want = readEnabled();
        for (Entry e : new ArrayList<>(entries)) {
            String w = want.get(e.pkg);
            if (w == null || !w.equals(e.wanted)) {
                remove(e);
            }
        }
        for (Map.Entry<String, String> w : want.entrySet()) {
            Entry e = find(w.getKey());
            if (e == null) {
                e = new Entry(w.getKey(), w.getValue());
                entries.add(e);
                bind(e);
            } else if (!e.bound) {
                main.removeCallbacks(e.rebind);
                bind(e);
            }
        }
    }

    /** Inventory includes disabled plugins; duplicate services contribute one package row. */
    private static void writeInstalled() {
        try {
            PackageManager pm = app.getPackageManager();
            Map<String, Integer> slots = new LinkedHashMap<>();
            Map<String, String> labels = new LinkedHashMap<>();
            for (ResolveInfo found : pm.queryIntentServices(new Intent(ACTION), PackageManager.GET_META_DATA)) {
                ServiceInfo si = found.serviceInfo;
                if (si == null) {
                    continue;
                }
                String declared = si.metaData == null ? null : si.metaData.getString(SLOTS_META);
                // mask() treats empty enablement as all slots; empty declarations declare none.
                int known = declared == null || declared.trim().isEmpty() ? 0 : mask(declared);
                if (known == 0) {
                    continue;
                }
                Integer previous = slots.get(si.packageName);
                slots.put(si.packageName, known | (previous == null ? 0 : previous));
                String label = si.applicationInfo.loadLabel(pm).toString().trim();
                int end = label.offsetByCodePoints(0, Math.min(48, label.codePointCount(0, label.length())));
                labels.put(si.packageName, label.substring(0, end));
            }
            List<String> packages = new ArrayList<>(slots.keySet());
            packages.sort((a, b) -> {
                int order = String.CASE_INSENSITIVE_ORDER.compare(labels.get(a), labels.get(b));
                return order == 0 ? a.compareTo(b) : order;
            });
            JSONArray installed = new JSONArray();
            for (String pkg : packages) {
                List<String> names = new ArrayList<>();
                int known = slots.get(pkg);
                if ((known & WIDGETS) != 0) names.add("widgets");
                if ((known & MEDIA) != 0) names.add("media");
                if ((known & NAVIGATION) != 0) names.add("navigation");
                installed.put(new JSONObject().put("package", pkg).put("label", labels.get(pkg))
                        .put("slots", String.join(",", names)));
            }
            File dir = new File(app.getFilesDir(), "plugins");
            Files.createDirectories(dir.toPath());
            File tmp = new File(dir, "installed.json.tmp");
            try (FileOutputStream out = new FileOutputStream(tmp)) {
                out.write(installed.toString().getBytes(StandardCharsets.UTF_8));
                out.getFD().sync();
            }
            if (!tmp.renameTo(new File(dir, "installed.json"))) {
                throw new IOException("installed.json rename failed");
            }
        } catch (IOException | JSONException | RuntimeException e) {
            Trace.event("plugins: cannot write installed.json: " + e);
        }
    }

    private static Map<String, String> readEnabled() {
        Map<String, String> out = new LinkedHashMap<>();
        File f = new File(app.getFilesDir(), ENABLED);
        if (!f.isFile()) {
            return out;
        }
        try {
            for (String line : Files.readAllLines(f.toPath(), StandardCharsets.UTF_8)) {
                String s = line.trim();
                if (s.isEmpty() || s.startsWith("#")) {
                    continue;
                }
                int colon = s.indexOf(':');
                if (colon < 0) {
                    out.put(s, "");
                } else {
                    out.put(s.substring(0, colon).trim(), s.substring(colon + 1).trim());
                }
            }
        } catch (IOException e) {
            Trace.event("plugins: cannot read " + ENABLED + ": " + e);
        }
        return out;
    }

    /** Parses "widgets,media,navigation" into slot bits. Empty means every slot. */
    static int mask(String names) {
        if (names == null || names.trim().isEmpty()) {
            return WIDGETS | MEDIA | NAVIGATION;
        }
        int m = 0;
        for (String n : names.split(",")) {
            switch (n.trim()) {
                case "widgets": m |= WIDGETS; break;
                case "media": m |= MEDIA; break;
                case "navigation": m |= NAVIGATION; break;
                default: break;
            }
        }
        return m;
    }

    private static void bind(Entry e) {
        PackageManager pm = app.getPackageManager();
        List<ResolveInfo> found = pm.queryIntentServices(
                new Intent(ACTION).setPackage(e.pkg), PackageManager.GET_META_DATA);
        if (found.isEmpty()) {
            e.retry("no service (not installed?)");
            return;
        }
        ServiceInfo si = found.get(0).serviceInfo;
        String declared = si.metaData == null ? null : si.metaData.getString(SLOTS_META);
        e.slots = mask(declared) & mask(e.wanted);
        if (declared == null || declared.trim().isEmpty() || e.slots == 0) {
            Trace.event("plugin " + e.pkg + " rejected: slots declared=" + declared + " enabled=" + e.wanted);
            return;
        }
        e.uid = si.applicationInfo.uid;
        ComponentName cn = new ComponentName(si.packageName, si.name);
        if (!app.bindService(new Intent(ACTION).setComponent(cn), e.conn, Context.BIND_AUTO_CREATE)) {
            e.retry("bindService refused");
            return;
        }
        e.bound = true;
        Trace.event("plugin bind " + e.pkg + " slots=" + e.slots);
    }

    private static void remove(Entry e) {
        e.removed = true;
        main.removeCallbacks(e.rebind);
        e.unbind();
        entries.remove(e);
    }

    private static Entry find(String pkg) {
        for (Entry e : entries) {
            if (e.pkg.equals(pkg)) {
                return e;
            }
        }
        return null;
    }

    // ---- display snapshot -------------------------------------------------------------------

    private static void schedulePush() {
        if (!pushQueued) {
            pushQueued = true;
            main.post(PluginHost::push);
        }
    }

    /** Current media source: the most recent publish among bound plugins and the built-in source. */
    private static MediaInfo displayMedia() {
        MediaInfo best = stock;
        for (Entry e : entries) {
            if (e.bound && e.media != null && (best == null || e.media.seq > best.seq)) {
                best = e.media;
            }
        }
        return best;
    }

    private static Nav displayNav() {
        for (Entry e : entries) {
            if (e.bound && e.nav != null && e.nav.active) {
                return e.nav;
            }
        }
        return null;
    }

    private static void push() {
        pushQueued = false;
        if (app == null) {
            return;
        }
        stockPlaying = stock != null && stock.playing;
        try {
            JSONArray widgets = new JSONArray();
            for (Entry e : entries) {
                if (!e.bound) {
                    continue;
                }
                for (Widget w : e.widgets.values()) {
                    if (widgets.length() >= MAX_TOTAL_WIDGETS) {
                        break;
                    }
                    widgets.put(new JSONObject().put("id", e.pkg + "|" + w.id).put("title", w.title)
                            .put("value", w.value).put("unit", w.unit));
                }
            }
            MediaInfo m = displayMedia();
            JSONObject media = new JSONObject().put("title", m == null ? "" : m.title)
                    .put("artist", m == null ? "" : m.artist)
                    .put("playing", m != null && m.playing);
            Nav n = displayNav();
            JSONObject nav = new JSONObject().put("active", n != null)
                    .put("instruction", n == null ? "" : n.instruction)
                    .put("distance", n == null ? "" : n.distance)
                    .put("eta", n == null ? "" : n.eta);
            nativeSetPlugins(new JSONObject().put("widgets", widgets).put("media", media)
                    .put("nav", nav).toString());
        } catch (JSONException ex) {
            Trace.event("plugins: snapshot failed " + ex);
        }
    }

    // ---- plugin calls into the host (IDashHost) ---------------------------------------------

    private static boolean allowed(Entry e, int slot) {
        if (e.bound && !e.removed && (e.slots & slot) != 0) {
            return true;
        }
        Trace.event("plugin " + e.pkg + " not enabled for slot " + slot + ", ignored");
        return false;
    }

    private static void onWidget(Entry e, String id, String title, String value, String unit) {
        if (!allowed(e, WIDGETS) || id == null || id.isEmpty() || id.length() > MAX_ID) {
            return;
        }
        if (!e.widgets.containsKey(id) && e.widgets.size() >= MAX_WIDGETS) {
            Trace.event("plugin " + e.pkg + " widget limit " + MAX_WIDGETS + ", ignored " + id);
            return;
        }
        e.widgets.put(id, new Widget(id, clip(title, MAX_TITLE), clip(value, MAX_SHORT), clip(unit, MAX_SHORT)));
        schedulePush();
    }

    private static void onRemoveWidget(Entry e, String id) {
        if (allowed(e, WIDGETS) && e.widgets.remove(id) != null) {
            schedulePush();
        }
    }

    private static void onMedia(Entry e, String title, String artist, boolean playing) {
        if (!allowed(e, MEDIA)) {
            return;
        }
        e.media = title == null || title.isEmpty() ? null
                : new MediaInfo(clip(title, MAX_LONG), clip(artist, MAX_LONG), playing, ++seq, e);
        schedulePush();
    }

    private static void onNavigation(Entry e, boolean active, String instruction, String distance, String eta) {
        if (!allowed(e, NAVIGATION)) {
            return;
        }
        e.nav = new Nav(active, clip(instruction, MAX_LONG), clip(distance, MAX_SHORT), clip(eta, MAX_SHORT));
        schedulePush();
    }

    private static void sendAction(Entry e, String action) {
        try {
            if (e.plugin != null) {
                e.plugin.onAction(action);
            }
        } catch (RemoteException ex) {
            Trace.event("plugin " + e.pkg + " action failed " + ex);
        }
    }

    private static String clip(String s, int max) {
        if (s == null) {
            return "";
        }
        return s.length() <= max ? s : s.substring(0, max);
    }

    // ---- state -------------------------------------------------------------------------------

    private static final class Widget {
        final String id, title, value, unit;

        Widget(String id, String title, String value, String unit) {
            this.id = id;
            this.title = title;
            this.value = value;
            this.unit = unit;
        }
    }

    private static final class MediaInfo {
        final String title, artist;
        final boolean playing;
        final long seq;
        final Entry owner; // null for the built-in source

        MediaInfo(String title, String artist, boolean playing, long seq, Entry owner) {
            this.title = title;
            this.artist = artist;
            this.playing = playing;
            this.seq = seq;
            this.owner = owner;
        }
    }

    private static final class Nav {
        final boolean active;
        final String instruction, distance, eta;

        Nav(boolean active, String instruction, String distance, String eta) {
            this.active = active;
            this.instruction = instruction;
            this.distance = distance;
            this.eta = eta;
        }
    }

    /** One enabled package. Its stub and connection are its own, so calls are attributed to it. */
    private static final class Entry {
        final String pkg, wanted;
        int slots, uid;
        boolean bound, removed;
        IDashPlugin plugin;
        IBinder binder;
        long backoffMs = BACKOFF_MIN_MS;
        final Map<String, Widget> widgets = new LinkedHashMap<>();
        MediaInfo media;
        Nav nav;
        final IBinder.DeathRecipient death = () -> main.post(() -> lost("binder died"));
        final ServiceConnection conn = new Connection(this);
        final IDashHost host = new HostStub(this);
        final Runnable rebind = () -> {
            if (!removed && !bound) {
                bind(this);
            }
        };

        Entry(String pkg, String wanted) {
            this.pkg = pkg;
            this.wanted = wanted;
        }

        /** Connection dropped (crash, force-stop): clear its data, then rebind with backoff. */
        void lost(String why) {
            if (!bound) {
                return;
            }
            Trace.event("plugin lost " + pkg + ": " + why);
            unbind();
            if (!removed) {
                retry(why);
            }
        }

        void unbind() {
            bound = false;
            if (plugin != null) {
                try {
                    plugin.detach();
                } catch (RemoteException ignored) {
                    // Dead already.
                }
            }
            if (binder != null) {
                binder.unlinkToDeath(death, 0);
            }
            plugin = null;
            binder = null;
            widgets.clear();
            media = null;
            nav = null;
            try {
                app.unbindService(conn);
            } catch (IllegalArgumentException ignored) {
                // Not bound.
            }
            schedulePush();
        }

        void retry(String why) {
            Trace.event("plugin " + pkg + " retry in " + backoffMs + " ms: " + why);
            main.postDelayed(rebind, backoffMs);
            backoffMs = Math.min(backoffMs * 2, BACKOFF_MAX_MS);
        }
    }

    private static final class Connection implements ServiceConnection {
        private final Entry e;

        Connection(Entry e) {
            this.e = e;
        }

        @Override
        public void onServiceConnected(ComponentName name, IBinder service) {
            if (!e.bound) {
                return;
            }
            e.binder = service;
            try {
                service.linkToDeath(e.death, 0);
                e.plugin = IDashPlugin.Stub.asInterface(service);
                e.plugin.attach(e.host, API_VERSION);
                e.backoffMs = BACKOFF_MIN_MS;
                Trace.event("plugin attached " + e.pkg);
            } catch (RemoteException ex) {
                e.lost("attach failed " + ex);
            }
        }

        @Override
        public void onServiceDisconnected(ComponentName name) {
            e.lost("disconnected");
        }

        @Override
        public void onBindingDied(ComponentName name) {
            e.lost("binding died");
        }

        @Override
        public void onNullBinding(ComponentName name) {
            e.lost("null binding");
        }
    }

    /** The IDashHost handed only to plugin e. Calls from any other uid are dropped. */
    private static final class HostStub extends IDashHost.Stub {
        private final Entry e;

        HostStub(Entry e) {
            this.e = e;
        }

        @Override
        public int apiVersion() {
            return API_VERSION;
        }

        @Override
        public void publishWidget(String id, String title, String value, String unit) {
            if (Binder.getCallingUid() == e.uid) {
                main.post(() -> onWidget(e, id, title, value, unit));
            }
        }

        @Override
        public void removeWidget(String id) {
            if (Binder.getCallingUid() == e.uid) {
                main.post(() -> onRemoveWidget(e, id));
            }
        }

        @Override
        public void publishMedia(String title, String artist, boolean playing) {
            if (Binder.getCallingUid() == e.uid) {
                main.post(() -> onMedia(e, title, artist, playing));
            }
        }

        @Override
        public void publishNavigation(boolean active, String instruction, String distance, String eta) {
            if (Binder.getCallingUid() == e.uid) {
                main.post(() -> onNavigation(e, active, instruction, distance, eta));
            }
        }
    }
}
