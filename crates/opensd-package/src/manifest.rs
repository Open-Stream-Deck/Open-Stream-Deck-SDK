//! The plugin manifest: `opensd.toml` in a project, `manifest.json` in a package.
//! Both use the same keys, so one set of types serves the CLI and the app.

use crate::paths::{extension, safe_path};
use crate::{Result, SDK};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub const MAX_ACTIONS: usize = 64;
pub const MAX_STATES: usize = 16;
pub const MAX_FIELDS: usize = 32;
const IMAGE_TYPES: [&str; 6] = ["png", "jpg", "jpeg", "svg", "webp", "gif"];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub sdk: u32,
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// Oldest app version the plugin works with.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_app: Option<String>,
    pub runtime: Runtime,
    #[serde(default, skip_serializing_if = "Ui::is_empty")]
    pub ui: Ui,
    /// Project-only packing options; the packer drops them from `manifest.json`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub package: Option<PackageOptions>,
    #[serde(default, skip_serializing_if = "Permissions::is_empty")]
    pub permissions: Permissions,
    pub actions: Vec<Action>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Runtime {
    #[serde(rename = "type")]
    pub kind: RuntimeKind,
    pub main: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RuntimeKind {
    Wasm,
    Js,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ui {
    /// Default key settings page for every action.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inspector: Option<String>,
    /// Plugin-wide settings window.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settings: Option<String>,
}

impl Ui {
    pub fn is_empty(&self) -> bool {
        self.inspector.is_none() && self.settings.is_none()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageOptions {
    /// Files and folders copied into the package besides the code.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub include: Option<Vec<String>>,
}

impl PackageOptions {
    pub const DEFAULT_INCLUDE: [&'static str; 2] = ["assets", "ui"];
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Permissions {
    /// Loopback TCP ports for the host broker: `"9222"` or `"9222-9322"`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub loopback: Vec<String>,
    /// Origins the plugin may reach with fetch and WebSocket.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub network: Vec<String>,
    /// Writing text to the clipboard.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub clipboard: bool,
    /// Applications the plugin may query, watch, start and close.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub launch: Vec<LaunchTarget>,
    /// Pressing keys and typing text in the focused application.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub keyboard: bool,
    /// Switching the deck's profile and page.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub navigation: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LaunchTarget {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub windows: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub macos: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub linux: Vec<String>,
    /// Fixed arguments; `{0}`..`{9}` take numbers at run time.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Action {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(default = "default_controllers")]
    pub controllers: Vec<Controller>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inspector: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<Field>,
    /// Settings a new instance starts with.
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub settings: Map<String, Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub states: Vec<State>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<TitleStyle>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gestures: Option<Gestures>,
}

fn default_controllers() -> Vec<Controller> {
    vec![Controller::Key]
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Controller {
    /// A key of the main grid.
    Key,
    /// A slot of the side strip.
    Side,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct State {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_title: Option<bool>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TitleStyle {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub align: Option<TitleAlign>,
    /// Font size in pixels, 6..=32.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<u32>,
    /// `#rrggbb`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bold: Option<bool>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TitleAlign {
    Top,
    Middle,
    Bottom,
}

/// Thresholds of host-computed gestures, in milliseconds.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Gestures {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub long_press: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hold: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repeat: Option<u32>,
}

/// A field of the settings form the app draws for an action.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Field {
    pub id: String,
    pub label: String,
    #[serde(rename = "type")]
    pub kind: FieldKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<FieldOption>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step: Option<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FieldKind {
    Text,
    Textarea,
    Number,
    Select,
    Toggle,
    Color,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldOption {
    pub value: String,
    pub label: String,
}

impl Manifest {
    /// Parses and validates `manifest.json`.
    pub fn from_json(bytes: &[u8]) -> Result<Self> {
        let manifest: Manifest = serde_json::from_slice(bytes)
            .map_err(|e| crate::Error(format!("manifest.json: {e}")))?;
        manifest.validate()?;
        Ok(manifest)
    }

    /// Files the manifest refers to; each must be in the package.
    pub fn referenced_files(&self) -> Vec<&str> {
        let mut files = vec![self.runtime.main.as_str()];
        files.extend(self.icon.as_deref());
        files.extend(self.ui.inspector.as_deref());
        files.extend(self.ui.settings.as_deref());
        for action in &self.actions {
            files.extend(action.icon.as_deref());
            files.extend(action.inspector.as_deref());
            files.extend(action.states.iter().filter_map(|s| s.image.as_deref()));
        }
        files
    }

    /// Stable digest of the requested permissions; user consent is bound to it.
    pub fn permissions_digest(&self) -> String {
        let text = serde_json::to_string(&self.permissions).unwrap_or_default();
        format!("{:x}", Sha256::digest(text.as_bytes()))
    }

    pub fn action(&self, id: &str) -> Option<&Action> {
        self.actions.iter().find(|a| a.id == id)
    }

    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.sdk == SDK,
            "This plugin targets SDK {}, the app supports SDK {SDK}",
            self.sdk
        );
        ensure!(
            valid_id(&self.id),
            "id must be a lowercase reverse-domain name such as com.example.plugin"
        );
        text("name", &self.name, 64)?;
        text("author", &self.author, 64)?;
        ensure!(
            self.description.len() <= 512,
            "description is longer than 512 characters"
        );
        ensure!(
            valid_semver(&self.version),
            "version must be SemVer, e.g. 1.0.0"
        );
        if let Some(min) = &self.min_app {
            ensure!(valid_semver(min), "min_app must be SemVer, e.g. 0.2.0");
        }
        if let Some(url) = &self.homepage {
            ensure!(
                url.starts_with("https://")
                    && url.len() <= 256
                    && !url.contains(char::is_whitespace),
                "homepage must be an https:// link"
            );
        }
        let main = &self.runtime.main;
        file("runtime.main", main)?;
        let expected = match self.runtime.kind {
            RuntimeKind::Wasm => "wasm",
            RuntimeKind::Js => "js",
        };
        ensure!(
            extension(main) == expected,
            "runtime.main of a {expected} plugin must be a .{expected} file"
        );
        if let Some(icon) = &self.icon {
            image("icon", icon)?;
        }
        if let Some(page) = &self.ui.inspector {
            html("ui.inspector", page)?;
        }
        if let Some(page) = &self.ui.settings {
            html("ui.settings", page)?;
        }
        if let Some(include) = self.package.as_ref().and_then(|p| p.include.as_ref()) {
            for entry in include {
                file("package.include", entry)?;
            }
        }
        self.permissions.validate()?;
        ensure!(
            !self.actions.is_empty() && self.actions.len() <= MAX_ACTIONS,
            "A plugin declares 1 to {MAX_ACTIONS} actions"
        );
        let mut ids = BTreeSet::new();
        for action in &self.actions {
            action.validate(&self.id)?;
            ensure!(ids.insert(&action.id), "Duplicate action id {}", action.id);
        }
        Ok(())
    }
}

impl Action {
    fn validate(&self, plugin: &str) -> Result<()> {
        let id = &self.id;
        ensure!(
            valid_id(id)
                && id.len() > plugin.len() + 1
                && id.starts_with(plugin)
                && id[plugin.len()..].starts_with('.'),
            "Action id {id} must start with the plugin id and a dot"
        );
        text(&format!("{id}: name"), &self.name, 64)?;
        ensure!(
            self.description.len() <= 512,
            "{id}: description is longer than 512 characters"
        );
        if let Some(icon) = &self.icon {
            image(&format!("{id}: icon"), icon)?;
        }
        if let Some(page) = &self.inspector {
            html(&format!("{id}: inspector"), page)?;
        }
        let controllers: BTreeSet<_> = self.controllers.iter().collect();
        ensure!(
            !controllers.is_empty() && controllers.len() == self.controllers.len(),
            "{id}: controllers must list key and/or side once each"
        );
        ensure!(
            self.states.len() <= MAX_STATES,
            "{id}: at most {MAX_STATES} states"
        );
        for state in &self.states {
            if let Some(image_path) = &state.image {
                image(&format!("{id}: state image"), image_path)?;
            }
            if let Some(title) = &state.title {
                ensure!(
                    title.chars().count() <= 512,
                    "{id}: state title is too long"
                );
            }
        }
        if let Some(style) = &self.title {
            style
                .validate()
                .map_err(|e| crate::Error(format!("{id}: title: {e}")))?;
        }
        if let Some(g) = &self.gestures {
            for (name, value) in [
                ("long_press", g.long_press),
                ("hold", g.hold),
                ("repeat", g.repeat),
            ] {
                if let Some(ms) = value {
                    ensure!(
                        (50..=10_000).contains(&ms),
                        "{id}: gestures.{name} must be 50..=10000 ms"
                    );
                }
            }
            ensure!(
                g.repeat.is_none() || g.hold.is_some(),
                "{id}: gestures.repeat needs gestures.hold"
            );
            ensure!(
                g.long_press.is_some() || g.hold.is_some(),
                "{id}: gestures needs long_press or hold"
            );
        }
        ensure!(
            self.fields.len() <= MAX_FIELDS,
            "{id}: at most {MAX_FIELDS} fields"
        );
        let mut keys = BTreeSet::new();
        for field in &self.fields {
            field
                .validate()
                .map_err(|e| crate::Error(format!("{id}: field {}: {e}", field.id)))?;
            ensure!(keys.insert(&field.id), "{id}: duplicate field {}", field.id);
        }
        Ok(())
    }
}

impl TitleStyle {
    pub fn validate(&self) -> Result<()> {
        if let Some(size) = self.size {
            ensure!((6..=32).contains(&size), "size must be 6..=32");
        }
        if let Some(color) = &self.color {
            ensure!(valid_color(color), "color must be #rrggbb");
        }
        Ok(())
    }
}

impl Field {
    fn validate(&self) -> Result<()> {
        ensure!(
            !self.id.is_empty()
                && self.id.len() <= 64
                && self
                    .id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-'),
            "id may use letters, digits, _ and -"
        );
        text("label", &self.label, 64)?;
        for value in [&self.placeholder, &self.hint].into_iter().flatten() {
            ensure!(
                value.chars().count() <= 256,
                "placeholder and hint are limited to 256 characters"
            );
        }
        let default = self.default.as_ref();
        match self.kind {
            FieldKind::Select => {
                ensure!(
                    !self.options.is_empty() && self.options.len() <= 64,
                    "select needs 1 to 64 options"
                );
                let values: BTreeSet<_> = self.options.iter().map(|o| &o.value).collect();
                ensure!(
                    values.len() == self.options.len(),
                    "select options must have unique values"
                );
                if let Some(d) = default {
                    ensure!(
                        d.as_str().is_some_and(|d| values.contains(&d.to_string())),
                        "default must be one of the option values"
                    );
                }
            }
            FieldKind::Number => {
                if let (Some(min), Some(max)) = (self.min, self.max)
                    && min > max
                {
                    bail!("min must not exceed max");
                }
                ensure!(self.step.is_none_or(|s| s > 0.0), "step must be positive");
                ensure!(
                    default.is_none_or(Value::is_number),
                    "default must be a number"
                );
            }
            FieldKind::Toggle => ensure!(
                default.is_none_or(Value::is_boolean),
                "default must be true or false"
            ),
            FieldKind::Color => ensure!(
                default.is_none_or(|d| d.as_str().is_some_and(valid_color)),
                "default must be #rrggbb"
            ),
            FieldKind::Text | FieldKind::Textarea => {
                ensure!(
                    default.is_none_or(Value::is_string),
                    "default must be a string"
                )
            }
        }
        ensure!(
            self.kind == FieldKind::Select || self.options.is_empty(),
            "options apply to select fields only"
        );
        Ok(())
    }
}

impl Permissions {
    pub fn is_empty(&self) -> bool {
        self.loopback.is_empty()
            && self.network.is_empty()
            && !self.clipboard
            && self.launch.is_empty()
            && !self.keyboard
            && !self.navigation
    }

    /// Whether `port` lies in a declared loopback range.
    pub fn allows_port(&self, port: u16) -> bool {
        self.loopback
            .iter()
            .filter_map(|r| parse_port_range(r))
            .any(|(from, to)| (from..=to).contains(&port))
    }

    pub fn target(&self, id: &str) -> Option<&LaunchTarget> {
        self.launch.iter().find(|t| t.id == id)
    }

    fn validate(&self) -> Result<()> {
        ensure!(self.loopback.len() <= 16, "At most 16 loopback ranges");
        for range in &self.loopback {
            match parse_port_range(range) {
                Some((from, to)) if from >= 1024 && to - from < 1000 => {}
                _ => bail!(
                    "Loopback range {range} must lie in 1024..=65535 and span at most 1000 ports"
                ),
            }
        }
        ensure!(self.network.len() <= 32, "At most 32 network origins");
        for origin in &self.network {
            ensure!(
                valid_origin(origin),
                "Network origin {origin} must look like https://api.example.com, wss://*.example.com or http://*:8123"
            );
        }
        ensure!(self.launch.len() <= 8, "At most 8 launch targets");
        let mut ids = BTreeSet::new();
        for target in &self.launch {
            ensure!(
                valid_slug(&target.id) && ids.insert(&target.id),
                "Launch target id {} must be unique, lowercase letters, digits and -",
                target.id
            );
            text("launch name", &target.name, 64)?;
            let paths: Vec<_> = target
                .windows
                .iter()
                .chain(&target.macos)
                .chain(&target.linux)
                .collect();
            ensure!(
                !paths.is_empty(),
                "Launch target {} lists no executable",
                target.id
            );
            for path in paths {
                ensure!(
                    !path.is_empty()
                        && path.len() <= 260
                        && !path.contains("..")
                        && !path.chars().any(|c| c.is_control() || c == '"'),
                    "Unsafe launch path {path}"
                );
            }
            ensure!(
                target.args.len() <= 16,
                "Launch target {} has too many arguments",
                target.id
            );
            for arg in &target.args {
                ensure!(
                    valid_arg_template(arg),
                    "Launch argument {arg} may only use {{0}}..{{9}} placeholders"
                );
            }
        }
        Ok(())
    }
}

/// `"9222"` or `"9222-9322"`.
pub fn parse_port_range(range: &str) -> Option<(u16, u16)> {
    let (from, to) = range.split_once('-').unwrap_or((range, range));
    let (from, to): (u16, u16) = (from.trim().parse().ok()?, to.trim().parse().ok()?);
    (from <= to).then_some((from, to))
}

/// Fills `{n}` placeholders of validated launch arguments with numbers.
pub fn fill_args(templates: &[String], values: &[u64]) -> Result<Vec<String>> {
    templates
        .iter()
        .map(|template| {
            let mut out = String::new();
            let mut rest = template.as_str();
            while let Some(open) = rest.find('{') {
                out.push_str(&rest[..open]);
                let index = usize::from(rest.as_bytes()[open + 1] - b'0');
                let Some(value) = values.get(index) else {
                    bail!("Missing launch value {{{index}}}");
                };
                out.push_str(&value.to_string());
                rest = &rest[open + 3..];
            }
            out.push_str(rest);
            Ok(out)
        })
        .collect()
}

fn valid_arg_template(arg: &str) -> bool {
    let bytes = arg.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'{' if bytes.get(i + 1).is_some_and(u8::is_ascii_digit)
                && bytes.get(i + 2) == Some(&b'}') =>
            {
                i += 3
            }
            b'{' | b'}' | 0 => return false,
            _ => i += 1,
        }
    }
    arg.len() <= 256
}

/// `scheme://host[:port]` with an optional `*` host or `*.` subdomain wildcard.
pub fn valid_origin(origin: &str) -> bool {
    let Some((scheme, rest)) = origin.split_once("://") else {
        return false;
    };
    if !matches!(scheme, "https" | "wss" | "http" | "ws") || rest.is_empty() || origin.len() > 200 {
        return false;
    }
    let (host, port) = match rest.rsplit_once(':') {
        Some((host, port)) => (host, Some(port)),
        None => (rest, None),
    };
    let port_ok = port.is_none_or(|p| {
        p == "*" || (!p.is_empty() && p.len() <= 5 && p.bytes().all(|b| b.is_ascii_digit()))
    });
    let label = |l: &str| {
        !l.is_empty()
            && l.len() <= 63
            && l.bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    };
    let host_ok = host == "*"
        || host
            .strip_prefix("*.")
            .unwrap_or(host)
            .split('.')
            .all(label);
    port_ok && host_ok
}

pub fn valid_id(id: &str) -> bool {
    id.len() <= 128
        && id.split('.').count() >= 2
        && id.split('.').all(|part| {
            !part.is_empty()
                && !part.starts_with('-')
                && !part.ends_with('-')
                && part
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        })
}

fn valid_slug(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

fn valid_color(color: &str) -> bool {
    color.len() == 7 && color.starts_with('#') && color[1..].bytes().all(|b| b.is_ascii_hexdigit())
}

/// `MAJOR.MINOR.PATCH` with an optional `-pre` and `+build`.
pub fn valid_semver(version: &str) -> bool {
    let core = version.split(['-', '+']).next().unwrap_or_default();
    let parts: Vec<_> = core.split('.').collect();
    version.len() <= 64
        && parts.len() == 3
        && parts.iter().all(|p| {
            !p.is_empty()
                && p.bytes().all(|b| b.is_ascii_digit())
                && (p.len() == 1 || !p.starts_with('0'))
        })
        && version
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'+'))
}

/// Compares two valid SemVer strings by their numeric core.
pub fn semver_at_least(version: &str, min: &str) -> bool {
    let core = |v: &str| -> Vec<u64> {
        v.split(['-', '+'])
            .next()
            .unwrap_or_default()
            .split('.')
            .map(|p| p.parse().unwrap_or(0))
            .collect()
    };
    core(version) >= core(min)
}

fn text(name: &str, value: &str, max: usize) -> Result<()> {
    ensure!(!value.trim().is_empty(), "{name} is required");
    ensure!(
        value.chars().count() <= max,
        "{name} is longer than {max} characters"
    );
    ensure!(
        !value.chars().any(char::is_control),
        "{name} contains control characters"
    );
    Ok(())
}

fn file(name: &str, path: &str) -> Result<()> {
    ensure!(
        safe_path(path),
        "{name}: {path} is not a portable relative path"
    );
    Ok(())
}

fn image(name: &str, path: &str) -> Result<()> {
    file(name, path)?;
    ensure!(
        IMAGE_TYPES.contains(&extension(path).as_str()),
        "{name}: {path} must be png, jpg, svg, webp or gif"
    );
    Ok(())
}

fn html(name: &str, path: &str) -> Result<()> {
    file(name, path)?;
    ensure!(
        extension(path) == "html",
        "{name}: {path} must be an .html file"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    pub(crate) fn sample() -> Manifest {
        serde_json::from_value(json!({
            "sdk": 1, "id": "com.example.test", "name": "Test", "version": "1.0.0", "author": "Me",
            "runtime": { "type": "wasm", "main": "plugin.wasm" },
            "actions": [{ "id": "com.example.test.one", "name": "One" }]
        }))
        .unwrap()
    }

    #[test]
    fn minimal_manifest_is_valid_and_defaults_to_key() {
        let m = sample();
        m.validate().unwrap();
        assert_eq!(m.actions[0].controllers, [Controller::Key]);
        assert_eq!(m.referenced_files(), ["plugin.wasm"]);
    }

    #[test]
    fn rejects_foreign_actions_and_bad_runtime() {
        let mut m = sample();
        m.actions[0].id = "com.other.one".into();
        assert!(m.validate().is_err());
        let mut m = sample();
        m.runtime.main = "plugin.js".into();
        assert!(m.validate().is_err());
        let mut m = sample();
        m.sdk = 2;
        assert!(m.validate().is_err());
    }

    #[test]
    fn gestures_and_fields_are_checked() {
        let mut m = sample();
        m.actions[0].gestures = Some(Gestures {
            repeat: Some(100),
            ..Default::default()
        });
        assert!(m.validate().is_err(), "repeat without hold");
        m.actions[0].gestures = Some(Gestures {
            hold: Some(400),
            repeat: Some(100),
            ..Default::default()
        });
        m.validate().unwrap();
        m.actions[0].fields = vec![Field {
            id: "mode".into(),
            label: "Mode".into(),
            kind: FieldKind::Select,
            placeholder: None,
            hint: None,
            default: Some(json!("b")),
            options: vec![FieldOption {
                value: "a".into(),
                label: "A".into(),
            }],
            min: None,
            max: None,
            step: None,
        }];
        assert!(m.validate().is_err(), "default outside options");
    }

    #[test]
    fn permissions_scope_and_digest() {
        let mut m = sample();
        m.permissions.loopback = vec!["9222-9322".into()];
        m.permissions.launch = vec![LaunchTarget {
            id: "app".into(),
            name: "App".into(),
            windows: vec!["%LOCALAPPDATA%/App/app.exe".into()],
            args: vec!["--port={0}".into(), "--flag".into()],
            ..Default::default()
        }];
        m.validate().unwrap();
        assert!(m.permissions.allows_port(9300) && !m.permissions.allows_port(9400));
        assert_eq!(
            fill_args(&m.permissions.launch[0].args, &[9222]).unwrap(),
            ["--port=9222", "--flag"]
        );
        let before = m.permissions_digest();
        m.permissions.clipboard = true;
        assert_ne!(before, m.permissions_digest());
        let before = m.permissions_digest();
        m.permissions.keyboard = true;
        assert_ne!(before, m.permissions_digest());
        assert!(
            !Permissions {
                navigation: true,
                ..Default::default()
            }
            .is_empty()
        );
        m.permissions.loopback = vec!["80".into()];
        assert!(m.validate().is_err());
        for bad in ["{x}", "{10}", "a}", "{0"] {
            assert!(!valid_arg_template(bad), "{bad}");
        }
    }

    #[test]
    fn origins() {
        for good in [
            "https://api.example.com",
            "wss://*.example.com",
            "http://*:8123",
            "http://192.168.1.10:8123",
            "https://*",
        ] {
            assert!(valid_origin(good), "{good}");
        }
        for bad in [
            "ftp://x.com",
            "https://",
            "https://x.com/path",
            "https://x.com:99999x",
            "https://a b.com",
            "https://x.com;",
        ] {
            assert!(!valid_origin(bad), "{bad}");
        }
    }

    #[test]
    fn semver_rules() {
        assert!(valid_semver("1.0.0") && valid_semver("1.2.3-beta.1+build"));
        assert!(!valid_semver("1.0") && !valid_semver("01.0.0") && !valid_semver("1.0.0 "));
        assert!(semver_at_least("0.2.0", "0.1.9") && !semver_at_least("0.1.0", "0.2.0"));
    }
}
