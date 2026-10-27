// @ts-check
/// <reference path="../../crates/opensd-cli/types/opensd.d.ts" />

const stamp = () => new Date().toLocaleTimeString();

opensd.action("dev.opensd.uikit.demo", {
  appear({ key, settings }) {
    key.setTitle(String(settings.name || "UI"));
  },
  settings({ key, settings }) {
    key.setTitle(String(settings.name || "UI"));
  },
  tap({ key }) {
    key.flash("ok");
  },
  inspectorMessage({ key, payload }) {
    key.toInspector({ echo: payload ?? null, at: stamp() });
  },
});

opensd.on("settingsPageMessage", ({ payload }) => {
  opensd.toSettingsPage({ echo: payload, at: stamp() });
});
