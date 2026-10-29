// Ola Dash SDK plugin API v1. Implemented by a plugin's bound service.
//
// A plugin is an ordinary APK exporting a service with
//   <action android:name="dev.olatool.dash.plugin.PLUGIN" />
// and meta-data "dev.olatool.dash.plugin.slots" = comma list of widgets,media,navigation.
// The host binds only plugins the owner enabled. See docs/plugins.md.
package dev.olatool.dash.plugin;

import dev.olatool.dash.plugin.IDashHost;

interface IDashPlugin {
    /** Host is ready; keep the IDashHost to publish. hostApiVersion >= 1. */
    oneway void attach(IDashHost host, int hostApiVersion);

    /** Host is going away; stop publishing. */
    oneway void detach();

    /**
     * A user action routed to this plugin: "media.play-pause", "media.next",
     * "media.previous", or "widget.<id>" when one of its widgets was tapped.
     */
    oneway void onAction(String action);
}
