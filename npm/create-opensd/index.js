#!/usr/bin/env node
"use strict";

const { spawnSync } = require("node:child_process");

const args = process.argv.slice(2);
if (!args.some((arg) => !arg.startsWith("-"))) args.unshift("my-plugin");
if (!args.some((arg) => arg === "--lang" || arg.startsWith("--lang="))) args.push("--lang", "ts");

const cli = require.resolve("opensd/bin/opensd.js");
const result = spawnSync(process.execPath, [cli, "new", ...args], { stdio: "inherit" });
process.exit(result.status ?? 1);
