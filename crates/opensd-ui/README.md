# opensd-ui

UI кит и мост для HTML-страниц плагинов Open Stream Deck: страницы настроек кнопки и окна плагина.

Приложение отдаёт файлы крейта каждой странице плагина по адресам `/_opensd/inspector.css` и `/_opensd/inspector.js`. Страница подключает их и выглядит как приложение без своего CSS:

```html
<link rel="stylesheet" href="/_opensd/inspector.css">
<script src="/_opensd/inspector.js"></script>
```

Хост вызывает `opensd_ui::file(path)` и получает тип и содержимое файла:

```rust
if let Some((mime, body)) = opensd_ui::file(request.uri().path()) {
    // ответить 200 с этим телом
}
```

Компоненты и API описаны в [вики](https://wiki.opensd.dev/docs/sdk/ui). Пример со всеми компонентами лежит в [examples/ui-kit](https://github.com/Open-Stream-Deck/Open-Stream-Deck-SDK/tree/main/examples/ui-kit).

Код под MIT или Apache 2.0 на выбор. Шрифт Inter в `assets` распространяется по SIL Open Font License 1.1, текст лицензии в `assets/OFL.txt`.
