# Changelog

## 1.2.0

Страницы плагинов получают новый кит только в приложении, собранном с `opensd-ui` 1.2.0.

- Крейт `opensd-ui`: UI кит и мост для HTML-страниц плагина. Приложение отдаёт его по `/_opensd/inspector.css` и `/_opensd/inspector.js`. Дизайн повторяет приложение, но кит живёт отдельно и не зависит от стилей приложения: классы кита это публичный API SDK.
- Без классов в стиле приложения рисуются поля, `select`, `textarea`, переключатели, радио, слайдеры, цвет, файлы, кнопки, `progress`, `details`, `dialog`, таблицы и текст. Светлая и тёмная тема, шрифт Inter.
- `select` превращается в выпадающий список приложения с группами, подсказками `data-hint`, клавиатурой и поиском по первой букве. `input type="color"` открывает свою палитру с полем насыщенности, полосой оттенка, готовыми цветами и HEX. Поле `data-hotkey` записывает сочетание клавиш, `.stepper` добавляет кнопки к числу, `.tabs` переключает вкладки, `.dropzone` принимает файл перетаскиванием.
- Компоненты `.segmented`, `.chips` (один или несколько вариантов), `.swatches`, `.field-row`, `.input-group`, `.card`, `.list`, `.alert`, `.badge`, `.status`, `.skeleton`, `.empty`, `.grid`, `.stack`, `.cluster`.
- Свои полосы прокрутки вместо системных: у страницы, `textarea`, `pre`, списков и блоков `.scroll`, с затуханием краёв.
- Несколько чекбоксов с одним `data-setting` сохраняются массивом значений.
- `opensd.toast(text, kind)`, `opensd.confirm(options)`, `opensd.readFile(file, as)`, подсказки `data-tip`, диалоги через `data-open` и `data-close`. Типы в `inspector.d.ts`.
- Пример `examples/ui-kit` со всеми компонентами.

## 1.1.0

Новые возможности требуют приложения, собранного с `opensd-package` 1.1.0. Плагин с разрешениями `keyboard` или `navigation` не установится в старое приложение: манифест не пройдёт проверку.

- Разрешение `keyboard`: `hotkey`, `key_down`, `key_up`, `type_text` в Rust и `opensd.keyboard` в JavaScript. Сочетания пишутся как `Ctrl + Shift + M` или `ctrl+shift+m`, буквы нажимаются по положению клавиши при любой раскладке, удержанные клавиши хост отпускает при остановке плагина.
- Разрешение `navigation`: `switch_profile`, `switch_page` и `opensd.switchProfile`, `opensd.switchPage`. Событие `navigation` с текущим профилем и страницей приходит после `init` и при каждой смене, `Host::navigation()` и `opensd.navigation` хранят последнее.
- `app_watch`, `app_unwatch` и `opensd.app.watch`: статус объявленной программы сразу и при каждом её запуске или закрытии.
- Событие `systemWake` после выхода компьютера из сна.
- `set_image` и `setImage` принимают адрес `http(s)://`, разрешённый `loopback` или `network`: картинку скачивает и кэширует приложение.
- `ws_open` и `opensd.ws` к адресам из `network` идут через брокер приложения, без CORS и заголовка `Origin`.
- `Request::binary()` в Rust и `binary` в `opensd.http`: тело ответа приходит в base64. Нужна версия приложения, которая проводит `network` через брокер.
- `opensd dev` собирает Rust-плагин профилем `opensd-dev` без LTO и с инкрементальным кешем, пересборка после сохранения в разы быстрее.
- `opensd dev` дочитывает журнал плагина с последней позиции, а не перечитывает файл целиком.

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
