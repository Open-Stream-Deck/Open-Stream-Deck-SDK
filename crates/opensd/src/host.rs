//! What a plugin can ask of the app.

use crate::event::{Controller, Device, Event, Position};
use crate::{Result, runtime};
use serde::Serialize;
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;

/// Title placement on a key.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Align {
    Top,
    Middle,
    Bottom,
}

/// How a plugin title is drawn. Unset fields keep the key's style; a title the
/// user typed is always shown in the user's own style.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct TitleStyle {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub align: Option<Align>,
    /// Font size in pixels, 6..=32.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u32>,
    /// `#rrggbb`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bold: Option<bool>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    Debug,
    Info,
    Warn,
    Error,
}

/// An HTTP request through the host. Loopback URLs need the `loopback`
/// permission, other URLs need a matching `network` origin.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Request {
    pub url: String,
    /// `GET` when empty.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub method: String,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub headers: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    /// Milliseconds; the host clamps it to 100..=30000 and uses 5000 by default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u32>,
}

impl Request {
    pub fn get(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            ..Self::default()
        }
    }

    pub fn post(url: impl Into<String>, body: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            method: "POST".into(),
            body: Some(body.into()),
            ..Self::default()
        }
    }

    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(name.into(), value.into());
        self
    }

    pub fn timeout(mut self, ms: u32) -> Self {
        self.timeout = Some(ms);
        self
    }
}

/// A visible action instance, as the host announced it with `appear`.
#[derive(Clone, Debug, PartialEq)]
pub struct KeyInfo {
    pub context: String,
    pub action: String,
    pub controller: Option<Controller>,
    pub position: Option<Position>,
}

/// Commands to the app. Zero-sized and `Copy`: keep one in any struct that
/// needs to act, without threading [`Host`] through every call.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Commands;

/// The plugin's view of the app: what is visible, plus every command of
/// [`Commands`] through `Deref`.
#[derive(Default)]
pub struct Host {
    commands: Commands,
    now: u64,
    plugin_id: String,
    language: String,
    device: Option<Device>,
    globals: Value,
    keys: BTreeMap<String, KeyInfo>,
}

impl Host {
    pub(crate) fn observe(&mut self, event: &Event, now: u64) {
        self.now = self.now.max(now);
        match event {
            Event::Init(init) => {
                self.plugin_id = init.plugin.id.clone();
                self.language = init.app.language.clone();
                self.device = init.device.clone();
                self.globals = init.globals.clone();
                self.keys.clear();
            }
            Event::Device(device) => self.device = device.clone(),
            Event::Language(language) => self.language = language.clone(),
            Event::Globals(settings) => self.globals = settings.clone(),
            Event::Appear(key) => {
                self.keys.insert(
                    key.context.clone(),
                    KeyInfo {
                        context: key.context.clone(),
                        action: key.action.clone(),
                        controller: key.controller,
                        position: key.position,
                    },
                );
            }
            _ => {}
        }
    }

    pub(crate) fn forget(&mut self, event: &Event) {
        if let Event::Disappear(key) = event {
            self.keys.remove(&key.context);
        }
    }

    /// Monotonic milliseconds since the plugin started, as of the current event.
    pub fn now(&self) -> u64 {
        self.now
    }

    pub fn plugin_id(&self) -> &str {
        &self.plugin_id
    }

    /// Interface language of the app, such as `ru`, `en` or `zh`.
    pub fn language(&self) -> &str {
        &self.language
    }

    pub fn device(&self) -> Option<&Device> {
        self.device.as_ref()
    }

    /// Plugin-wide settings as last saved or received.
    pub fn globals(&self) -> &Value {
        &self.globals
    }

    /// Visible instances of one action.
    pub fn keys_of<'a>(&'a self, action: &'a str) -> impl Iterator<Item = &'a KeyInfo> + 'a {
        self.keys.values().filter(move |k| k.action == action)
    }

    /// Every visible instance of the plugin.
    pub fn keys(&self) -> impl Iterator<Item = &KeyInfo> {
        self.keys.values()
    }

    pub fn is_visible(&self, context: &str) -> bool {
        self.keys.contains_key(context)
    }

    /// The command handle, to keep in plugin structs.
    pub fn commands(&self) -> Commands {
        self.commands
    }

    /// Saves plugin-wide settings and remembers them as [`globals`](Self::globals).
    pub fn set_globals<T: Serialize>(&mut self, settings: &T) -> Result {
        let settings = serde_json::to_value(settings)?;
        self.globals = settings.clone();
        self.commands
            .send(json!({ "type": "setGlobals", "settings": settings }))
    }
}

impl std::ops::Deref for Host {
    type Target = Commands;

    fn deref(&self) -> &Commands {
        &self.commands
    }
}

impl Commands {
    /// Commands about one key. Commands for a key that has just disappeared are
    /// dropped by the host without an error.
    pub fn key<'a>(&self, context: &'a str) -> Key<'a> {
        Key { context }
    }

    /// Sends a raw protocol command.
    pub fn send(&self, command: Value) -> Result {
        runtime::send(&command)
    }

    /// Saves plugin-wide settings. [`Host::set_globals`] also updates `Host::globals`.
    pub fn set_globals<T: Serialize>(&self, settings: &T) -> Result {
        self.send(json!({ "type": "setGlobals", "settings": settings }))
    }

    /// Sends a message to the plugin settings window, if it is open.
    pub fn to_settings_page<T: Serialize>(&self, payload: &T) -> Result {
        self.send(json!({ "type": "toSettingsPage", "payload": payload }))
    }

    pub fn log(&self, level: Level, message: impl AsRef<str>) -> Result {
        self.send(json!({ "type": "log", "level": level, "message": message.as_ref() }))
    }

    pub fn debug(&self, message: impl AsRef<str>) -> Result {
        self.log(Level::Debug, message)
    }

    pub fn info(&self, message: impl AsRef<str>) -> Result {
        self.log(Level::Info, message)
    }

    pub fn warn(&self, message: impl AsRef<str>) -> Result {
        self.log(Level::Warn, message)
    }

    pub fn error(&self, message: impl AsRef<str>) -> Result {
        self.log(Level::Error, message)
    }

    /// Opens an `https://` link in the browser. Works within 3 seconds of a
    /// user action with this plugin (a key press, a settings page message).
    pub fn open_url(&self, url: &str) -> Result {
        self.send(json!({ "type": "openUrl", "url": url }))
    }

    /// Fires `Event::Timer(id)` once after `ms`. Reusing an id replaces the timer.
    pub fn set_timer(&self, id: &str, ms: u32) -> Result {
        self.send(json!({ "type": "setTimer", "id": id, "ms": ms, "repeat": false }))
    }

    /// Fires `Event::Timer(id)` every `ms` until [`clear_timer`](Self::clear_timer).
    pub fn set_interval(&self, id: &str, ms: u32) -> Result {
        self.send(json!({ "type": "setTimer", "id": id, "ms": ms, "repeat": true }))
    }

    pub fn clear_timer(&self, id: &str) -> Result {
        self.send(json!({ "type": "clearTimer", "id": id }))
    }

    /// Sends a request; the reply arrives as `Event::Http` with the same id.
    pub fn http(&self, id: &str, request: &Request) -> Result {
        let mut command = serde_json::to_value(request)?;
        command["type"] = json!("http");
        command["id"] = json!(id);
        self.send(command)
    }

    pub fn http_get(&self, id: &str, url: &str) -> Result {
        self.http(id, &Request::get(url))
    }

    /// Opens a WebSocket: `Event::WsOpen`, then `WsMessage`s, then `WsClose`.
    pub fn ws_open(&self, id: &str, url: &str) -> Result {
        self.send(json!({ "type": "wsOpen", "id": id, "url": url }))
    }

    pub fn ws_send(&self, id: &str, data: &str) -> Result {
        self.send(json!({ "type": "wsSend", "id": id, "data": data }))
    }

    pub fn ws_close(&self, id: &str) -> Result {
        self.send(json!({ "type": "wsClose", "id": id }))
    }

    /// Whether a declared application is installed and running: `Event::AppStatus`.
    pub fn app_status(&self, id: &str, app: &str) -> Result {
        self.send(json!({ "type": "appStatus", "id": id, "app": app }))
    }

    /// Starts a declared application; `args` fill its `{0}`..`{9}`: `Event::Done`.
    pub fn app_launch(&self, id: &str, app: &str, args: &[u64]) -> Result {
        self.send(json!({ "type": "appLaunch", "id": id, "app": app, "args": args }))
    }

    /// Asks a declared application to close: `Event::Done`.
    pub fn app_quit(&self, id: &str, app: &str) -> Result {
        self.send(json!({ "type": "appQuit", "id": id, "app": app }))
    }

    /// Writes text to the clipboard: `Event::Done`.
    pub fn clipboard(&self, id: &str, text: &str) -> Result {
        self.send(json!({ "type": "clipboard", "id": id, "text": text }))
    }
}

/// Commands about one key.
pub struct Key<'a> {
    context: &'a str,
}

impl Key<'_> {
    fn command(&self, kind: &str, mut fields: Map<String, Value>) -> Result<&Self> {
        fields.insert("type".into(), json!(kind));
        fields.insert("context".into(), json!(self.context));
        runtime::send(&Value::Object(fields))?;
        Ok(self)
    }

    fn fields(value: Value) -> Map<String, Value> {
        match value {
            Value::Object(map) => map,
            _ => Map::new(),
        }
    }

    pub fn set_title(&self, title: &str) -> Result<&Self> {
        self.command("setTitle", Self::fields(json!({ "title": title })))
    }

    pub fn set_title_styled(&self, title: &str, style: &TitleStyle) -> Result<&Self> {
        self.command(
            "setTitle",
            Self::fields(json!({ "title": title, "style": style })),
        )
    }

    /// Back to the title from the manifest.
    pub fn reset_title(&self) -> Result<&Self> {
        self.command("setTitle", Self::fields(json!({ "title": null })))
    }

    /// A path inside the package or a `data:image/…;base64,` URI (see [`crate::image`]).
    pub fn set_image(&self, image: &str) -> Result<&Self> {
        self.command("setImage", Self::fields(json!({ "image": image })))
    }

    /// Back to the image from the manifest.
    pub fn reset_image(&self) -> Result<&Self> {
        self.command("setImage", Self::fields(json!({ "image": null })))
    }

    pub fn set_state(&self, state: u32) -> Result<&Self> {
        self.command("setState", Self::fields(json!({ "state": state })))
    }

    pub fn flash_ok(&self) -> Result<&Self> {
        self.command("flash", Self::fields(json!({ "kind": "ok" })))
    }

    pub fn flash_alert(&self) -> Result<&Self> {
        self.command("flash", Self::fields(json!({ "kind": "alert" })))
    }

    pub fn save_settings<T: Serialize>(&self, settings: &T) -> Result<&Self> {
        let settings = serde_json::to_value(settings)?;
        self.command("setSettings", Self::fields(json!({ "settings": settings })))
    }

    /// Sends a message to this key's settings page, if it is open.
    pub fn to_inspector<T: Serialize>(&self, payload: &T) -> Result<&Self> {
        let payload = serde_json::to_value(payload)?;
        self.command("toInspector", Self::fields(json!({ "payload": payload })))
    }
}
