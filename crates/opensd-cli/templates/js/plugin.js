// @ts-check
/// <reference path="./opensd.d.ts" />

opensd.action("{{id}}.counter", {
  appear({ key, settings }) {
    key.setTitle(String(settings.count ?? 0));
  },
  settings({ key, settings }) {
    key.setTitle(String(settings.count ?? 0));
  },
  tap({ key, settings }) {
    const count = Number(settings.count ?? 0) + Math.max(1, Number(settings.step ?? 1));
    key.saveSettings({ ...settings, count }).setTitle(String(count));
  },
  longPress({ key, settings }) {
    key.saveSettings({ ...settings, count: 0 }).setTitle("0").flash("ok");
  },
});
