# Changelog

## 1.0.2

- `opensd new --lang ts` подсказывает `npm run dev` вместо `opensd dev`: без глобальной установки CLI команда `opensd` в проекте не находится.

## 1.0.1

- Пакеты npm переименованы: `opensd` стал `@opensd/cli`, `create-opensd` стал `@opensd/create`. Новый проект: `npm create @opensd my-plugin`. Под старыми именами пакеты не публиковались.
- Шаблон TypeScript ставит `@opensd/cli`.
- Скрипт сборки npm-пакетов работает на Windows.

## 1.0.0

Первый стабильный выпуск SDK.

- Формат пакета `.opensd`: ZIP с `manifest.json`, индексом целостности `index.json`, кодом и ресурсами.
- Плагины на Rust, JavaScript, TypeScript и любом языке, который собирается в WebAssembly по ABI с одним импортом `opensd.send`.
- Исполнение в изолированном воркере движка WebView: без файлов, процессов, окна приложения и сети вне разрешений.
- Жесты, которые считает хост: `tap`, `longPress`, `holdStart`, `holdRepeat`, `holdEnd`.
- Поля настроек в манифесте, которые приложение рисует само; мост `/_opensd/inspector.js` для HTML-страниц с привязкой полей через `data-setting`.
- Разрешения `loopback`, `network`, `clipboard`, `launch` с согласием пользователя, привязанным к дайджесту запроса.
- Крейт `opensd` с типизированными событиями, `Host`, `Commands` и тестовым стендом `testing::Harness`.
- CLI `opensd`: `new`, `build`, `pack`, `check`, `inspect`, `dev`, `clean`. Общий кеш сборки для всех Rust-плагинов.
- Пакеты npm, готовые бинарники для Windows, macOS и Linux, установка одной командой.
