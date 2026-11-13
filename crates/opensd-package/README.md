# opensd-package

Формат пакета `.opensd` для плагинов [Open Stream Deck](https://github.com/Open-Stream-Deck/Open-Stream-Deck-SDK).

Пакет это обычный ZIP: `manifest.json`, `index.json` с размером и SHA-256 каждого файла, код плагина (`plugin.wasm` или `plugin.js`) и ресурсы. Исполняемые файлы ОС отклоняются.

Что есть в крейте:

- `Manifest`: типы манифеста с полной проверкой;
- `build`, `read`, `verify`: упаковка и проверка архивов;
- `read_dir`: проверка установленной или подготовленной папки плагина по индексу;
- `wasm`: проверка модуля на соответствие ABI SDK и подготовка его к упаковке.

Крейт общий для CLI `opensd` и приложения Open Stream Deck. Авторам плагинов нужен крейт [`opensd`](https://crates.io/crates/opensd) и утилита [`opensd-cli`](https://crates.io/crates/opensd-cli).

Формат пакета описан в [документации](https://wiki.opensd.dev/docs/sdk/package).

## Лицензия

На выбор: [MIT](LICENSE-MIT) или [Apache License 2.0](LICENSE-APACHE).

## Внешние компоненты

Начиная с 1.3.0 крейт проверяет `targets`, `components`, платформенные варианты и источники нативных файлов. Пакеты с компонентами используют формат 2, обычные плагины сохраняют формат 1. Для установки компонентов нужно приложение Open Stream Deck 1.1.0 или новее. Полный контракт: [COMPONENTS.md](COMPONENTS.md).
