# opensd

Rust SDK для плагинов [Open Stream Deck](https://github.com/Open-Stream-Deck/Open-Stream-Deck-SDK).

Плагин это крейт `cdylib`, собранный под `wasm32-unknown-unknown`. Он работает в изолированном воркере приложения: без файлов, процессов и сети. Плагин получает типизированные события и действует через хост.

```rust
use opensd::{Event, Host, Plugin, Result};

#[derive(Default)]
struct Counter {
    count: u32,
}

impl Plugin for Counter {
    fn event(&mut self, host: &mut Host, event: Event) -> Result {
        if let Event::Tap(key) = event {
            self.count += 1;
            host.key(&key.context).set_title(&self.count.to_string())?;
        }
        Ok(())
    }
}

opensd::export!(Counter::default());
```

Новый проект создаётся через CLI:

```bash
cargo install opensd-cli
opensd new my-plugin --lang rust
cd my-plugin
opensd dev
```

`opensd::testing::Harness` запускает плагин в обычном тесте и записывает команды, которые он отправляет. Плагины тестируются через `cargo test`, без приложения и устройства.

Документация SDK: [wiki.opensd.dev/docs/sdk](https://wiki.opensd.dev/docs/sdk). Протокол, формат пакета и лимиты описаны в [справочнике](https://wiki.opensd.dev/docs/sdk/protocol).

## Лицензия

На выбор: [MIT](LICENSE-MIT) или [Apache License 2.0](LICENSE-APACHE).
