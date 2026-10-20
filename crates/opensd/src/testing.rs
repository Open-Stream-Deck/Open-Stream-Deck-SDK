//! Unit tests for plugins without the app: feed events, read the commands.
//!
//! ```
//! use opensd::{testing::Harness, json, Event, Host, Plugin, Result};
//!
//! struct Echo;
//! impl Plugin for Echo {
//!     fn event(&mut self, host: &mut Host, event: Event) -> Result {
//!         if let Event::KeyDown(key) = event {
//!             host.key(&key.context).set_title("down")?;
//!         }
//!         Ok(())
//!     }
//! }
//!
//! let mut deck = Harness::new(Echo);
//! deck.appear("c1", "com.example.echo.key");
//! deck.event(json!({ "type": "keyDown", "context": "c1", "action": "com.example.echo.key" })).unwrap();
//! assert_eq!(deck.commands()[0]["title"], "down");
//! ```

#![cfg(not(target_arch = "wasm32"))]

use crate::runtime::{Standalone, take_outbox};
use crate::{Host, Plugin, Result};
use serde_json::{Value, json};

/// A plugin running in the test process, with the host replaced by a recorder.
pub struct Harness {
    plugin: Standalone,
    now: u64,
}

impl Harness {
    /// Starts the plugin and delivers `init`.
    pub fn new(plugin: impl Plugin) -> Self {
        let mut harness = Self {
            plugin: Standalone::new(Box::new(plugin)),
            now: 0,
        };
        harness
            .event(json!({
                "type": "init",
                "plugin": { "id": "com.example.test", "version": "1.0.0" },
                "app": { "version": "0.2.0", "os": "windows", "language": "en" },
                "device": {
                    "id": "TEST", "name": "Test deck", "connected": true,
                    "keys": { "columns": 5, "rows": 3, "width": 103, "height": 103 },
                    "side": { "slots": 3, "width": 85, "height": 103 }
                },
                "globals": {}
            }))
            .expect("init failed");
        take_outbox();
        harness
    }

    /// Moves the host clock forward; the next events carry the new time.
    pub fn advance(&mut self, ms: u64) {
        self.now += ms;
    }

    /// Delivers one protocol event; `now` is filled in when missing.
    pub fn event(&mut self, mut event: Value) -> Result {
        if event.get("now").is_none() {
            event["now"] = json!(self.now);
        }
        self.plugin.deliver(event)
    }

    /// Delivers `appear` for an instance with empty settings.
    pub fn appear(&mut self, context: &str, action: &str) {
        self.event(json!({
            "type": "appear", "context": context, "action": action, "controller": "key",
            "position": { "column": 0, "row": 0 }, "settings": {}, "state": 0
        }))
        .expect("appear failed");
    }

    /// Commands the plugin sent since the previous call.
    pub fn commands(&mut self) -> Vec<Value> {
        take_outbox()
    }

    pub fn host(&self) -> &Host {
        self.plugin.host()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Event, Request};

    #[derive(Default)]
    struct Probe {
        taps: u32,
    }

    impl Plugin for Probe {
        fn event(&mut self, host: &mut Host, event: Event) -> Result {
            match event {
                Event::Tap(key) => {
                    self.taps += 1;
                    host.key(&key.context)
                        .set_title(&self.taps.to_string())?
                        .flash_ok()?;
                }
                Event::LongPress(_) => host.http(
                    "list",
                    &Request::get("http://127.0.0.1:9222/json/list").timeout(1000),
                )?,
                Event::Timer(id) => host.info(format!("timer {id} at {}", host.now()))?,
                Event::KeyUp(_) => return Err("boom".into()),
                _ => {}
            }
            Ok(())
        }
    }

    #[test]
    fn registry_commands_and_errors() {
        let mut deck = Harness::new(Probe::default());
        deck.appear("c1", "com.example.test.a");
        deck.appear("c2", "com.example.test.b");
        assert_eq!(deck.host().keys_of("com.example.test.a").count(), 1);
        deck.event(json!({"type":"tap","context":"c1","action":"com.example.test.a"}))
            .unwrap();
        let commands = deck.commands();
        assert_eq!(
            commands[0],
            json!({"type":"setTitle","context":"c1","title":"1"})
        );
        assert_eq!(
            commands[1],
            json!({"type":"flash","context":"c1","kind":"ok"})
        );
        deck.event(json!({"type":"longPress","context":"c1","action":"com.example.test.a"}))
            .unwrap();
        let http = &deck.commands()[0];
        assert_eq!(
            (
                http["type"].as_str(),
                http["id"].as_str(),
                http["timeout"].as_u64()
            ),
            (Some("http"), Some("list"), Some(1000))
        );
        deck.advance(250);
        deck.event(json!({"type":"timer","id":"poll"})).unwrap();
        assert_eq!(deck.commands()[0]["message"], "timer poll at 250");
        assert!(
            deck.event(json!({"type":"keyUp","context":"c1","action":"com.example.test.a"}))
                .is_err()
        );
        deck.event(json!({"type":"disappear","context":"c1","action":"com.example.test.a"}))
            .unwrap();
        assert!(!deck.host().is_visible("c1") && deck.host().is_visible("c2"));
    }
}
