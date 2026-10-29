package dev.olatool.dash.plugin;

import android.content.ComponentName;
import android.media.MediaMetadata;
import android.media.session.MediaController;
import android.media.session.MediaSessionManager;
import android.media.session.PlaybackState;
import android.os.Handler;
import android.os.Looper;
import android.service.notification.NotificationListenerService;

import dev.olatool.dash.Trace;

import java.util.Collections;
import java.util.List;

/**
 * Built-in media source. Reads the active media session and sends it transport controls. Android
 * only exposes sessions to a granted notification listener, so the owner grants it once:
 *   adb shell cmd notification allow_listener dev.olatool.dash/dev.olatool.dash.plugin.StockMediaService
 * Runs on the main looper, so all state here is main-thread only.
 */
public final class StockMediaService extends NotificationListenerService {
    private static StockMediaService live;

    private final Handler main = new Handler(Looper.getMainLooper());
    private MediaSessionManager sessions;
    private MediaController current;

    private final MediaController.Callback callback = new MediaController.Callback() {
        @Override
        public void onPlaybackStateChanged(PlaybackState state) {
            publish();
        }

        @Override
        public void onMetadataChanged(MediaMetadata metadata) {
            publish();
        }

        @Override
        public void onSessionDestroyed() {
            pick(Collections.<MediaController>emptyList());
        }
    };

    private final MediaSessionManager.OnActiveSessionsChangedListener listener =
            this::pick;

    @Override
    public void onListenerConnected() {
        live = this;
        sessions = getSystemService(MediaSessionManager.class);
        try {
            sessions.addOnActiveSessionsChangedListener(listener, component());
            pick(sessions.getActiveSessions(component()));
        } catch (SecurityException e) {
            Trace.event("media: session access refused " + e.getMessage());
        }
    }

    @Override
    public void onListenerDisconnected() {
        live = null;
        if (sessions != null) {
            sessions.removeOnActiveSessionsChangedListener(listener);
        }
        pick(Collections.<MediaController>emptyList());
    }

    private ComponentName component() {
        return new ComponentName(this, StockMediaService.class);
    }

    /** Prefers a playing session, otherwise the first one. */
    private void pick(List<MediaController> list) {
        MediaController next = null;
        for (MediaController c : list) {
            if (next == null) {
                next = c;
            }
            if (isPlaying(c)) {
                next = c;
                break;
            }
        }
        if (next != current) {
            if (current != null) {
                current.unregisterCallback(callback);
            }
            current = next;
            if (current != null) {
                current.registerCallback(callback, main);
            }
        }
        publish();
    }

    private void publish() {
        if (current == null) {
            PluginHost.setStockMedia(null, null, false);
            return;
        }
        MediaMetadata md = current.getMetadata();
        String title = md == null ? null : md.getString(MediaMetadata.METADATA_KEY_TITLE);
        String artist = md == null ? null : md.getString(MediaMetadata.METADATA_KEY_ARTIST);
        PluginHost.setStockMedia(title, artist, isPlaying(current));
    }

    private static boolean isPlaying(MediaController c) {
        PlaybackState s = c.getPlaybackState();
        return s != null && s.getState() == PlaybackState.STATE_PLAYING;
    }

    /** Transport control for the active session. Called on the main thread by PluginHost. */
    static void control(String action) {
        StockMediaService s = live;
        if (s == null || s.current == null) {
            return;
        }
        MediaController.TransportControls t = s.current.getTransportControls();
        switch (action) {
            case "media.play-pause":
                if (isPlaying(s.current)) {
                    t.pause();
                } else {
                    t.play();
                }
                break;
            case "media.next":
                t.skipToNext();
                break;
            case "media.previous":
                t.skipToPrevious();
                break;
            default:
                break;
        }
    }
}
