/// <reference path="../opensd.d.ts" />

type Settings = { count?: number; step?: number };

opensd.action<Settings>("{{id}}.counter", {
  appear({ key, settings }) {
    key.setTitle(String(settings.count ?? 0));
  },
  settings({ key, settings }) {
    key.setTitle(String(settings.count ?? 0));
  },
  tap({ key, settings }) {
    const count = (settings.count ?? 0) + Math.max(1, settings.step ?? 1);
    key.saveSettings({ ...settings, count }).setTitle(String(count));
  },
  longPress({ key, settings }) {
    key.saveSettings({ ...settings, count: 0 }).setTitle("0").flash("ok");
  },
});
