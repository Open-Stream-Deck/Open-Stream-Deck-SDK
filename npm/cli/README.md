# @opensd/cli

Утилита `opensd` для плагинов Open Stream Deck в проектах на JavaScript и TypeScript.

```bash
npm create @opensd my-plugin
cd my-plugin
npm install
npx opensd dev
```

Или в существующем проекте:

```bash
npm install --save-dev @opensd/cli
npx opensd pack
```

Глобально:

```bash
npm install -g @opensd/cli
opensd new my-plugin
```

Пакет ставит готовый бинарник `opensd` для Windows, macOS или Linux и содержит типы API плагина: `types/opensd.d.ts` и `types/inspector.d.ts`.

Документация: https://wiki.opensd.dev/docs/sdk

## Лицензия

На выбор: MIT или Apache-2.0.
