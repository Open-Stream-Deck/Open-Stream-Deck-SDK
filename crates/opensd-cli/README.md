# opensd-cli

`opensd` создаёт, собирает, проверяет и упаковывает плагины [Open Stream Deck](https://github.com/Open-Stream-Deck/Open-Stream-Deck-SDK) на Rust, JavaScript и TypeScript.

```bash
cargo install opensd-cli
```

Готовые бинарники для Windows, macOS и Linux прикладываются к каждому [релизу](https://github.com/Open-Stream-Deck/Open-Stream-Deck-SDK/releases), их ставит и `cargo binstall opensd-cli`.

| Команда | Что делает |
| --- | --- |
| `opensd new <dir> --lang rust\|js\|ts` | Создаёт проект плагина |
| `opensd dev` | Запускает плагин в приложении и перезапускает при каждом изменении |
| `opensd pack` | Пишет `dist/<id>-<version>.opensd` |
| `opensd check [<path>]` | Проверяет проект или готовый пакет |
| `opensd inspect <file>` | Показывает, что лежит в пакете и какие разрешения он просит |
| `opensd build`, `opensd clean` | Готовит плагин к запуску, удаляет результаты сборки |

Rust-плагины собираются в общий кеш в папке пользователя, поэтому зависимости компилируются один раз на машину.

Все команды и флаги описаны в [документации](https://wiki.opensd.dev/docs/sdk/cli).

## Лицензия

На выбор: [MIT](LICENSE-MIT) или [Apache License 2.0](LICENSE-APACHE).
