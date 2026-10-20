# Open Stream Deck SDK

SDK для плагинов Open Stream Deck. Плагин пишется на Rust, JavaScript, TypeScript или любом языке, который собирается в WebAssembly, и распространяется одним файлом `.opensd`.

[![crates.io](https://img.shields.io/crates/v/opensd.svg)](https://crates.io/crates/opensd)
[![docs.rs](https://img.shields.io/docsrs/opensd)](https://docs.rs/opensd)
[![npm](https://img.shields.io/npm/v/@opensd/cli.svg)](https://www.npmjs.com/package/@opensd/cli)
[![docs](https://img.shields.io/badge/docs-wiki.opensd.dev-blue)](https://wiki.opensd.dev/docs/sdk)
[![CI](https://github.com/Open-Stream-Deck/Open-Stream-Deck-SDK/actions/workflows/ci.yml/badge.svg)](https://github.com/Open-Stream-Deck/Open-Stream-Deck-SDK/actions/workflows/ci.yml)

**Документация SDK: [wiki.opensd.dev/docs/sdk](https://wiki.opensd.dev/docs/sdk)**

## Принципы

- **Один файл без исполняемых файлов ОС.** `.opensd` это обычный ZIP: манифест в JSON, код на JavaScript в исходном виде или модуль WebAssembly, ресурсы. Его можно распаковать любым архиватором, а `opensd inspect` покажет, что внутри и что плагин просит.
- **Одна песочница для всех языков.** Плагин работает в изолированном воркере движка WebView приложения. У него нет файлов, процессов, окна приложения и сети, кроме того, что пользователь разрешил явно.
- **Ноль в простое.** Хост не опрашивает плагины. Событие приходит, только когда что-то произошло, а таймер плагин включает сам и только когда он нужен.
- **Одинаково на Windows, macOS и Linux.** Код плагина не зависит от ОС.

## Установка

Готовый бинарник, без Rust и Node.js:

```powershell
irm https://raw.githubusercontent.com/Open-Stream-Deck/Open-Stream-Deck-SDK/main/install.ps1 | iex
```

```bash
curl -fsSL https://raw.githubusercontent.com/Open-Stream-Deck/Open-Stream-Deck-SDK/main/install.sh | sh
```

Через Rust:

```bash
cargo install opensd-cli
cargo binstall opensd-cli
```

Через npm, для проектов на JavaScript и TypeScript:

```bash
npm create @opensd my-plugin
```

Архивы для всех платформ и файл `SHA256SUMS` лежат в [релизах](https://github.com/Open-Stream-Deck/Open-Stream-Deck-SDK/releases).

## Быстрый старт

```bash
opensd new my-plugin --lang rust
opensd new my-plugin --lang js
opensd new my-plugin --lang ts
```

Внутри проекта:

```bash
opensd dev
```

`opensd dev` собирает плагин и запускает его в приложении (включите «Режим разработчика» в «Настройки», «Плагины»). При каждом сохранении плагин пересобирается и перезапускается, журнал выводится в терминал.

```bash
opensd pack
```

`opensd pack` создаёт `dist/<id>-<version>.opensd`. Файл устанавливается через «Настройки», «Плагины», «Установить из файла».

## Минимальный плагин

JavaScript:

```js
opensd.action("com.example.hello.key", {
  tap({ key }) {
    key.setTitle("Hello").flash("ok");
  },
});
```

Rust:

```rust
use opensd::{Event, Host, Plugin, Result};

#[derive(Default)]
struct Hello;

impl Plugin for Hello {
    fn event(&mut self, host: &mut Host, event: Event) -> Result {
        if let Event::Tap(key) = event {
            host.key(&key.context).set_title("Hello")?.flash_ok()?;
        }
        Ok(())
    }
}

opensd::export!(Hello);
```

## Что умеет плагин

- Реагировать на нажатия, отпускания, короткое и долгое нажатие, удержание с повтором. Жесты считает хост по порогам из манифеста.
- Менять подпись со стилем, картинку и состояние кнопки, ставить отметку `ok` или `alert`.
- Хранить настройки каждой кнопки и общие настройки плагина. Простые формы настроек описываются полями в манифесте, и приложение рисует их само.
- Показывать свою страницу настроек кнопки и отдельное окно настроек плагина на HTML.
- Работать с сетью, локальными программами, буфером обмена и запуском приложений через разрешения, которые подтверждает пользователь.

## Структура

| Путь | Назначение |
| --- | --- |
| `crates/opensd` | Rust SDK для плагинов, крейт [`opensd`](https://crates.io/crates/opensd) |
| `crates/opensd-package` | Формат `.opensd`: манифест, проверка, упаковка. Общий для CLI и приложения |
| `crates/opensd-cli` | CLI `opensd`, шаблоны проектов и типы JS API |
| `npm/` | Пакеты `@opensd/cli` и `@opensd/create` для npm |
| `install.sh`, `install.ps1` | Установка готового бинарника |

## Документация

Вся документация SDK находится в вики: [wiki.opensd.dev/docs/sdk](https://wiki.opensd.dev/docs/sdk).

| Раздел | О чём |
| --- | --- |
| [Быстрый старт](https://wiki.opensd.dev/docs/sdk/quickstart) | Установка CLI, первый проект, запуск и упаковка |
| [Rust](https://wiki.opensd.dev/docs/sdk/rust) | Плагин на Rust, крейт `opensd` |
| [JavaScript и TypeScript](https://wiki.opensd.dev/docs/sdk/javascript) | Глобальный объект `opensd` |
| [Настройки](https://wiki.opensd.dev/docs/sdk/settings) | Поля в манифесте, страница кнопки, окно плагина |
| [CLI](https://wiki.opensd.dev/docs/sdk/cli) | Команды `opensd` и режим разработчика |
| [Манифест](https://wiki.opensd.dev/docs/sdk/manifest), [протокол](https://wiki.opensd.dev/docs/sdk/protocol), [пакет](https://wiki.opensd.dev/docs/sdk/package) | Справочник |
| [Исполнение](https://wiki.opensd.dev/docs/sdk/runtime), [разрешения](https://wiki.opensd.dev/docs/sdk/permissions), [ABI](https://wiki.opensd.dev/docs/sdk/abi) | Песочница, лимиты, доступ к системе, WebAssembly для любого языка |
| [Безопасность](https://wiki.opensd.dev/docs/sdk/security) | Модель угроз и границы гарантий |
| [Миграция](https://wiki.opensd.dev/docs/sdk/migration) | Переход с SDK 0.2, Elgato и StreamDock |

Справка по API крейта `opensd` публикуется на [docs.rs](https://docs.rs/opensd).

## Разработка

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
```

## Лицензия

На выбор: [MIT](LICENSE-MIT) или [Apache License 2.0](LICENSE-APACHE).
