import { execFileSync } from "node:child_process";
import { chmodSync, copyFileSync, cpSync, existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const [archives, version] = process.argv.slice(2);
if (!archives || !version) {
  console.error("Usage: node npm/scripts/build.mjs <release archives folder> <version>");
  process.exit(1);
}

const npm = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const root = resolve(npm, "..");
const out = join(npm, "dist");
const repository = "git+https://github.com/Open-Stream-Deck/Open-Stream-Deck-SDK.git";
const licenses = ["LICENSE-MIT", "LICENSE-APACHE"];

const platforms = [
  { target: "x86_64-pc-windows-msvc", os: "win32", cpu: "x64" },
  { target: "aarch64-pc-windows-msvc", os: "win32", cpu: "arm64" },
  { target: "x86_64-apple-darwin", os: "darwin", cpu: "x64" },
  { target: "aarch64-apple-darwin", os: "darwin", cpu: "arm64" },
  { target: "x86_64-unknown-linux-musl", os: "linux", cpu: "x64" },
  { target: "aarch64-unknown-linux-musl", os: "linux", cpu: "arm64" },
];

const json = (path) => JSON.parse(readFileSync(path, "utf8"));
const write = (path, value) => writeFileSync(path, JSON.stringify(value, null, 2) + "\n");
const withLicenses = (dir) => licenses.forEach((file) => copyFileSync(join(root, file), join(dir, file)));

rmSync(out, { recursive: true, force: true });
mkdirSync(out, { recursive: true });

for (const { target, os, cpu } of platforms) {
  const windows = os === "win32";
  const archive = join(resolve(archives), `opensd-${target}${windows ? ".zip" : ".tar.gz"}`);
  if (!existsSync(archive)) throw new Error(`Missing ${archive}`);
  const dir = join(out, `cli-${os}-${cpu}`);
  const unpacked = join(dir, "unpacked");
  mkdirSync(join(dir, "bin"), { recursive: true });
  mkdirSync(unpacked, { recursive: true });
  const source = relative(unpacked, archive);
  if (process.platform === "win32") execFileSync(join(process.env.SystemRoot, "System32", "tar.exe"), ["-xf", source], { cwd: unpacked });
  else if (windows) execFileSync("unzip", ["-q", source], { cwd: unpacked });
  else execFileSync("tar", ["-xzf", source], { cwd: unpacked });
  const binary = windows ? "opensd.exe" : "opensd";
  copyFileSync(join(unpacked, binary), join(dir, "bin", binary));
  chmodSync(join(dir, "bin", binary), 0o755);
  rmSync(unpacked, { recursive: true });
  withLicenses(dir);
  writeFileSync(join(dir, "README.md"), `# @opensd/cli-${os}-${cpu}\n\nБинарник \`opensd\` для ${os} ${cpu}. Устанавливайте пакет [opensd](https://www.npmjs.com/package/opensd): он подтянет нужный бинарник сам.\n`);
  write(join(dir, "package.json"), {
    name: `@opensd/cli-${os}-${cpu}`,
    version,
    description: `Бинарник opensd для ${os} ${cpu}`,
    repository: { type: "git", url: repository, directory: "npm" },
    license: "MIT OR Apache-2.0",
    os: [os],
    cpu: [cpu],
    files: ["bin", "README.md", ...licenses],
    preferUnplugged: true,
  });
}

const cli = join(out, "opensd");
cpSync(join(npm, "opensd"), cli, { recursive: true });
cpSync(join(root, "crates/opensd-cli/types"), join(cli, "types"), { recursive: true });
withLicenses(cli);
const cliPackage = json(join(cli, "package.json"));
cliPackage.version = version;
for (const name of Object.keys(cliPackage.optionalDependencies)) cliPackage.optionalDependencies[name] = version;
write(join(cli, "package.json"), cliPackage);

const create = join(out, "create-opensd");
cpSync(join(npm, "create-opensd"), create, { recursive: true });
withLicenses(create);
const createPackage = json(join(create, "package.json"));
createPackage.version = version;
createPackage.dependencies.opensd = version;
write(join(create, "package.json"), createPackage);

console.log(`npm packages ${version} are in ${out}`);
