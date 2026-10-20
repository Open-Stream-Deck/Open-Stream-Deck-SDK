import { execFileSync } from "node:child_process";
import { readdirSync, readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const out = join(resolve(dirname(fileURLToPath(import.meta.url)), ".."), "dist");
const npm = process.platform === "win32" ? "npm.cmd" : "npm";
const order = [
  ...readdirSync(out).filter((name) => name.startsWith("cli-")),
  "cli",
  "create",
];

for (const folder of order) {
  const dir = join(out, folder);
  const { name, version } = JSON.parse(readFileSync(join(dir, "package.json"), "utf8"));
  let published = false;
  try {
    execFileSync(npm, ["view", `${name}@${version}`, "version"], { stdio: "pipe", shell: process.platform === "win32" });
    published = true;
  } catch {}
  if (published) {
    console.log(`${name}@${version} is already published`);
    continue;
  }
  const args = ["publish", "--access", "public"];
  if (process.env.GITHUB_ACTIONS) args.push("--provenance");
  execFileSync(npm, args, { cwd: dir, stdio: "inherit", shell: process.platform === "win32" });
}
