use opensd::{Event, Host, Plugin, Result};
use serde::{Deserialize, Serialize};

#[derive(Default, Deserialize, Serialize)]
#[serde(default)]
struct Settings {
    count: u64,
    step: u64,
}

#[derive(Default)]
struct Counter;

impl Plugin for Counter {
    fn event(&mut self, host: &mut Host, event: Event) -> Result {
        match event {
            Event::Appear(key) | Event::Settings(key) => {
                let settings: Settings = key.settings()?;
                host.key(&key.context).set_title(&settings.count.to_string())?;
            }
            Event::Tap(key) => {
                let mut settings: Settings = key.settings()?;
                settings.count += settings.step.max(1);
                host.key(&key.context)
                    .save_settings(&settings)?
                    .set_title(&settings.count.to_string())?;
            }
            Event::LongPress(key) => {
                let settings = Settings { count: 0, ..key.settings()? };
                host.key(&key.context).save_settings(&settings)?.set_title("0")?.flash_ok()?;
            }
            _ => {}
        }
        Ok(())
    }
}

opensd::export!(Counter);

#[cfg(test)]
mod tests {
    use super::*;
    use opensd::{json, testing::Harness};

    #[test]
    fn tap_counts_and_long_press_resets() {
        let action = "{{id}}.counter";
        let mut deck = Harness::new(Counter);
        deck.appear("k1", action);
        deck.commands();
        deck.event(json!({ "type": "tap", "context": "k1", "action": action, "settings": { "count": 4, "step": 2 } })).unwrap();
        let commands = deck.commands();
        assert_eq!(commands[0]["settings"]["count"], 6);
        assert_eq!(commands[1]["title"], "6");
        deck.event(json!({ "type": "longPress", "context": "k1", "action": action, "settings": { "count": 6, "step": 2 } })).unwrap();
        assert_eq!(deck.commands()[1]["title"], "0");
    }
}
