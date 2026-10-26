//! Events the app sends to a plugin.

use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::collections::BTreeMap;

/// Where an action sits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Controller {
    /// A key of the main grid.
    Key,
    /// A slot of the side strip.
    Side,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
pub struct Position {
    pub column: u32,
    pub row: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
pub struct Grid {
    pub columns: u32,
    pub rows: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
pub struct Strip {
    pub slots: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
pub struct Device {
    pub id: String,
    pub name: String,
    pub connected: bool,
    #[serde(default)]
    pub keys: Grid,
    #[serde(default)]
    pub side: Strip,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
pub struct PluginInfo {
    pub id: String,
    pub version: String,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
pub struct AppInfo {
    pub version: String,
    pub os: String,
    pub language: String,
}

/// The first event of every start.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Init {
    pub plugin: PluginInfo,
    pub app: AppInfo,
    pub device: Option<Device>,
    pub globals: Value,
}

/// An event about one action instance (one key).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct KeyEvent {
    /// Opaque id of the instance; commands about this key use it.
    pub context: String,
    pub action: String,
    pub controller: Option<Controller>,
    pub position: Option<Position>,
    /// Instance settings as currently saved.
    pub settings: Value,
    pub state: u32,
    /// `holdRepeat`: 1 for the first repeat.
    pub count: u32,
    /// `holdEnd`: how long the key was held, in milliseconds.
    pub duration: u64,
    /// `inspectorMessage`: what the settings page sent.
    pub payload: Value,
}

impl KeyEvent {
    /// The instance settings as a typed value; missing fields take their defaults
    /// when the type uses `#[serde(default)]`.
    pub fn settings<T: DeserializeOwned>(&self) -> crate::Result<T> {
        Ok(serde_json::from_value(if self.settings.is_null() {
            Value::Object(Default::default())
        } else {
            self.settings.clone()
        })?)
    }

    pub fn payload<T: DeserializeOwned>(&self) -> crate::Result<T> {
        Ok(serde_json::from_value(self.payload.clone())?)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
pub struct HttpResponse {
    pub status: u16,
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    #[serde(default)]
    pub body: String,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
pub struct AppProcess {
    pub pid: u32,
    /// Command-line arguments, without the executable.
    #[serde(default)]
    pub args: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
pub struct AppStatus {
    pub installed: bool,
    #[serde(default)]
    pub processes: Vec<AppProcess>,
}

impl AppStatus {
    pub fn running(&self) -> bool {
        !self.processes.is_empty()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
pub struct ProfileInfo {
    pub id: String,
    pub name: String,
}

/// What the deck shows.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Navigation {
    pub profile: ProfileInfo,
    /// Counting from 0.
    pub page: u32,
    pub pages: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    Init(Init),
    /// The deck was connected, disconnected or replaced.
    Device(Option<Device>),
    Language(String),
    Appear(KeyEvent),
    Disappear(KeyEvent),
    KeyDown(KeyEvent),
    KeyUp(KeyEvent),
    /// Released before any long press or hold.
    Tap(KeyEvent),
    /// Held for `gestures.long_press`; the key is still down.
    LongPress(KeyEvent),
    HoldStart(KeyEvent),
    HoldRepeat(KeyEvent),
    HoldEnd(KeyEvent),
    /// The user changed the instance settings.
    Settings(KeyEvent),
    /// Plugin-wide settings changed outside the plugin.
    Globals(Value),
    /// After `Init` and whenever the deck shows another profile or page.
    Navigation(Navigation),
    /// The computer woke up from sleep.
    SystemWake,
    InspectorOpen(KeyEvent),
    InspectorClose(KeyEvent),
    InspectorMessage(KeyEvent),
    SettingsPageOpen,
    SettingsPageClose,
    SettingsPageMessage(Value),
    Timer(String),
    Http {
        id: String,
        result: Result<HttpResponse, String>,
    },
    WsOpen {
        id: String,
    },
    WsMessage {
        id: String,
        data: String,
    },
    WsClose {
        id: String,
        reason: String,
    },
    AppStatus {
        id: String,
        status: AppStatus,
    },
    /// Result of `app_launch`, `app_quit`, `clipboard`, keyboard commands.
    Done {
        id: String,
        result: Result<(), String>,
    },
    /// The operation needs a permission the user has not granted.
    Denied {
        id: String,
        message: String,
    },
    /// An event this SDK version does not know yet.
    Other(Value),
}

impl Event {
    /// The instance an event is about, if any.
    pub fn key(&self) -> Option<&KeyEvent> {
        match self {
            Event::Appear(k)
            | Event::Disappear(k)
            | Event::KeyDown(k)
            | Event::KeyUp(k)
            | Event::Tap(k)
            | Event::LongPress(k)
            | Event::HoldStart(k)
            | Event::HoldRepeat(k)
            | Event::HoldEnd(k)
            | Event::Settings(k)
            | Event::InspectorOpen(k)
            | Event::InspectorClose(k)
            | Event::InspectorMessage(k) => Some(k),
            _ => None,
        }
    }
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct Raw {
    #[serde(rename = "type")]
    kind: String,
    now: u64,
    context: String,
    action: String,
    controller: Option<Controller>,
    position: Option<Position>,
    settings: Value,
    state: u32,
    count: u32,
    duration: u64,
    payload: Value,
    id: String,
    status: u16,
    headers: BTreeMap<String, String>,
    body: String,
    error: Option<String>,
    data: String,
    reason: String,
    installed: bool,
    processes: Vec<AppProcess>,
    message: String,
    device: Option<Device>,
    language: String,
    plugin: PluginInfo,
    app: AppInfo,
    globals: Value,
    profile: ProfileInfo,
    page: u32,
    pages: u32,
}

pub(crate) fn parse(value: Value) -> (Event, u64) {
    let raw: Raw = match serde_json::from_value(value.clone()) {
        Ok(raw) => raw,
        Err(_) => return (Event::Other(value), 0),
    };
    let now = raw.now;
    let key = |raw: Raw| KeyEvent {
        context: raw.context,
        action: raw.action,
        controller: raw.controller,
        position: raw.position,
        settings: raw.settings,
        state: raw.state,
        count: raw.count,
        duration: raw.duration,
        payload: raw.payload,
    };
    let event = match raw.kind.as_str() {
        "init" => Event::Init(Init {
            plugin: raw.plugin,
            app: raw.app,
            device: raw.device,
            globals: raw.globals,
        }),
        "device" => Event::Device(raw.device),
        "language" => Event::Language(raw.language),
        "appear" => Event::Appear(key(raw)),
        "disappear" => Event::Disappear(key(raw)),
        "keyDown" => Event::KeyDown(key(raw)),
        "keyUp" => Event::KeyUp(key(raw)),
        "tap" => Event::Tap(key(raw)),
        "longPress" => Event::LongPress(key(raw)),
        "holdStart" => Event::HoldStart(key(raw)),
        "holdRepeat" => Event::HoldRepeat(key(raw)),
        "holdEnd" => Event::HoldEnd(key(raw)),
        "settings" => Event::Settings(key(raw)),
        "globals" => Event::Globals(raw.settings),
        "navigation" => Event::Navigation(Navigation {
            profile: raw.profile,
            page: raw.page,
            pages: raw.pages,
        }),
        "systemWake" => Event::SystemWake,
        "inspectorOpen" => Event::InspectorOpen(key(raw)),
        "inspectorClose" => Event::InspectorClose(key(raw)),
        "inspectorMessage" => Event::InspectorMessage(key(raw)),
        "settingsPageOpen" => Event::SettingsPageOpen,
        "settingsPageClose" => Event::SettingsPageClose,
        "settingsPageMessage" => Event::SettingsPageMessage(raw.payload),
        "timer" => Event::Timer(raw.id),
        "httpResponse" => Event::Http {
            result: match raw.error {
                Some(error) => Err(error),
                None => Ok(HttpResponse {
                    status: raw.status,
                    headers: raw.headers,
                    body: raw.body,
                }),
            },
            id: raw.id,
        },
        "wsOpen" => Event::WsOpen { id: raw.id },
        "wsMessage" => Event::WsMessage {
            id: raw.id,
            data: raw.data,
        },
        "wsClose" => Event::WsClose {
            id: raw.id,
            reason: raw.reason,
        },
        "appStatus" => Event::AppStatus {
            id: raw.id,
            status: AppStatus {
                installed: raw.installed,
                processes: raw.processes,
            },
        },
        "done" => Event::Done {
            result: raw.error.map_or(Ok(()), Err),
            id: raw.id,
        },
        "denied" => Event::Denied {
            id: raw.id,
            message: raw.message,
        },
        _ => Event::Other(value),
    };
    (event, now)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn decodes_keys_brokers_and_unknown_events() {
        let (event, now) = parse(
            json!({"type":"holdRepeat","now":120,"context":"c1","action":"a","settings":{"x":1},"count":3}),
        );
        assert_eq!(now, 120);
        let Event::HoldRepeat(key) = event else {
            panic!()
        };
        assert_eq!((key.context.as_str(), key.count), ("c1", 3));
        let Event::Http { result: Err(e), .. } =
            parse(json!({"type":"httpResponse","id":"r","error":"refused"})).0
        else {
            panic!()
        };
        assert_eq!(e, "refused");
        let Event::Http { result: Ok(r), .. } =
            parse(json!({"type":"httpResponse","id":"r","status":200,"body":"[]"})).0
        else {
            panic!()
        };
        assert_eq!(r.status, 200);
        assert!(matches!(
            parse(json!({"type":"future","x":1})).0,
            Event::Other(_)
        ));
        assert!(matches!(
            parse(json!({"type":"done","id":"c"})).0,
            Event::Done { result: Ok(()), .. }
        ));
        let Event::Navigation(nav) = parse(
            json!({"type":"navigation","profile":{"id":"p1","name":"Игры"},"page":1,"pages":3}),
        )
        .0
        else {
            panic!()
        };
        assert_eq!(
            (nav.profile.name.as_str(), nav.page, nav.pages),
            ("Игры", 1, 3)
        );
        assert_eq!(parse(json!({"type":"systemWake"})).0, Event::SystemWake);
    }

    #[test]
    fn typed_settings_fill_defaults() {
        #[derive(Deserialize, Default)]
        #[serde(default)]
        struct S {
            count: u32,
            label: String,
        }
        let key = KeyEvent {
            settings: json!({"count": 4}),
            ..Default::default()
        };
        let s: S = key.settings().unwrap();
        assert_eq!((s.count, s.label.as_str()), (4, ""));
        let empty = KeyEvent::default();
        assert_eq!(empty.settings::<S>().unwrap().count, 0);
    }
}
