#!/usr/bin/env node
"use strict";

const { spawnSync } = require("node:child_process");

const name = `@opensd/cli-${process.platform}-${process.arch}`;
const file = process.platform === "win32" ? "opensd.exe" : "opensd";

let binary;
try {
  binary = require.resolve(`${name}/bin/${file}`);
} catch {
  console.error(`opensd: no prebuilt binary for ${process.platform}-${process.arch}.`);
  console.error("Install it with Rust instead: cargo install opensd-cli");
  process.exit(1);
}

const result = spawnSync(binary, process.argv.slice(2), { stdio: "inherit" });
if (result.error) {
  console.error(`opensd: ${result.error.message}`);
  process.exit(1);
}
process.exit(result.status ?? 1);
