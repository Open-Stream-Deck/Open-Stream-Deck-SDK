declare namespace opensdInspector {
  type Json = null | boolean | number | string | Json[] | { [key: string]: Json };
  type JsonObject = { [key: string]: Json };

  interface Context {
    /** "inspector" for a key settings page, "settings" for the plugin settings window. */
    kind: "inspector" | "settings";
    plugin: { id: string; version: string };
    /** The key being edited; absent in the settings window. */
    action?: string;
    context?: string;
    settings: JsonObject;
    globals: JsonObject;
    theme: "dark" | "light";
    language: string;
  }
}

declare const opensd: {
  /** Resolves once the app has connected the page. */
  readonly ready: Promise<opensdInspector.Context>;
  /** Current key settings (the inspector) or `{}` (the settings window). */
  readonly settings: opensdInspector.JsonObject;
  readonly globals: opensdInspector.JsonObject;
  readonly theme: "dark" | "light";
  readonly language: string;
  /** Saves the key settings; the plugin gets a `settings` event. */
  saveSettings(settings: opensdInspector.JsonObject): void;
  /** Saves plugin-wide settings; the plugin gets a `globals` event. */
  saveGlobals(settings: opensdInspector.JsonObject): void;
  /** Sends a message to the plugin: `inspectorMessage` or `settingsPageMessage`. */
  send(payload: opensdInspector.Json): void;
  /** Opens the plugin settings window. */
  openSettings(): void;
  /** Opens an https:// link in the browser. */
  openUrl(url: string): void;
  on(type: "message", fn: (payload: opensdInspector.Json) => void): void;
  on(type: "settings" | "globals", fn: (settings: opensdInspector.JsonObject) => void): void;
  on(type: "theme", fn: (theme: "dark" | "light") => void): void;
};
