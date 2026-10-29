// Ola Dash SDK plugin API v1. Implemented by the dash host; handed to plugins in
// IDashPlugin.attach(). Display-only: there is no way to command the vehicle.
// Additive changes only; bump API_VERSION in docs/plugins.md when adding a method.
package dev.olatool.dash.plugin;

interface IDashHost {
    /** Plugin API version the host implements (1). */
    int apiVersion();

    /** Add or update a value card. id is unique within the plugin. */
    oneway void publishWidget(String id, String title, String value, String unit);

    oneway void removeWidget(String id);

    /** Now-playing info. Only plugins enabled for the "media" slot are shown. */
    oneway void publishMedia(String title, String artist, boolean playing);

    /** Turn-by-turn. Only plugins enabled for the "navigation" slot are shown. */
    oneway void publishNavigation(boolean active, String instruction, String distance, String eta);
}
