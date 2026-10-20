use crate::Result;
use opensd_package::{Manifest, PackageOptions, RuntimeKind};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub const CONFIG: &str = "opensd.toml";
const SKIP: [&str; 6] = [
    ".git",
    ".opensd",
    "dist",
    "target",
    "node_modules",
    ".DS_Store",
];

pub struct Project {
    pub root: PathBuf,
    pub manifest: Manifest,
}

impl Project {
    pub fn open(dir: &Path) -> Result<Self> {
        let path = dir.join(CONFIG);
        let text = fs::read_to_string(&path).map_err(|_| {
            format!(
                "No {CONFIG} in {}. Run opensd inside a plugin project",
                dir.display()
            )
        })?;
        let manifest: Manifest = toml::from_str(&text).map_err(|e| format!("{CONFIG}: {e}"))?;
        manifest.validate().map_err(|e| format!("{CONFIG}: {e}"))?;
        let root = plain(dir.canonicalize().map_err(|e| e.to_string())?);
        Ok(Self { root, manifest })
    }

    pub fn check(&self) -> Result {
        let files = self.resources()?;
        for path in self.manifest.referenced_files() {
            if path != self.manifest.runtime.main && !files.contains_key(path) {
                return Err(format!(
                    "{CONFIG} refers to {path}, which is not in package.include"
                ));
            }
        }
        Ok(())
    }

    pub fn files(&self, release: bool) -> Result<BTreeMap<String, Vec<u8>>> {
        let mut files = self.resources()?;
        let code = match self.manifest.runtime.kind {
            RuntimeKind::Wasm => self.build_rust(release)?,
            RuntimeKind::Js => self.build_js()?,
        };
        files.insert(self.manifest.runtime.main.clone(), code);
        Ok(files)
    }

    pub fn stage(&self, release: bool) -> Result<PathBuf> {
        let mut files = self.files(release)?;
        if self.manifest.runtime.kind == RuntimeKind::Wasm {
            let main = &self.manifest.runtime.main;
            let prepared =
                opensd_package::wasm::prepare(&files[main]).map_err(|e| format!("{main}: {e}"))?;
            files.insert(main.clone(), prepared);
        }
        let mut manifest = self.manifest.clone();
        manifest.package = None;
        for path in manifest.referenced_files() {
            if !files.contains_key(path) {
                return Err(format!(
                    "{CONFIG} refers to {path}, which is not in package.include"
                ));
            }
        }
        let base = self.root.join(".opensd");
        fs::create_dir_all(&base).map_err(|e| e.to_string())?;
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let stage = base.join(format!("stage-{stamp}"));
        for (name, data) in &files {
            write(&stage.join(name), data)?;
        }
        write(
            &stage.join("manifest.json"),
            &serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?,
        )?;
        Ok(stage)
    }

    pub fn prune_stages(&self, keep: &Path) {
        let Ok(entries) = fs::read_dir(self.root.join(".opensd")) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path != keep && entry.file_name().to_string_lossy().starts_with("stage-") {
                let _ = fs::remove_dir_all(path);
            }
        }
    }

    pub fn pack(&self, out: Option<PathBuf>) -> Result {
        let files = self.files(true)?;
        let bytes = opensd_package::build(&self.manifest, files).map_err(|e| e.to_string())?;
        let out = out.unwrap_or_else(|| {
            self.root.join("dist").join(format!(
                "{}-{}.{}",
                self.manifest.id,
                self.manifest.version,
                opensd_package::EXTENSION
            ))
        });
        write(&out, &bytes)?;
        println!(
            "{}  {}  sha256 {}",
            out.display(),
            human_size(bytes.len() as u64),
            opensd_package::sha256(&bytes)
        );
        Ok(())
    }

    fn resources(&self) -> Result<BTreeMap<String, Vec<u8>>> {
        let defaults: Vec<String> = PackageOptions::DEFAULT_INCLUDE
            .iter()
            .map(|s| s.to_string())
            .collect();
        let include = self
            .manifest
            .package
            .as_ref()
            .and_then(|p| p.include.clone())
            .unwrap_or(defaults);
        let mut files = BTreeMap::new();
        for entry in include
            .iter()
            .map(String::as_str)
            .chain(["LICENSE", "NOTICE"])
        {
            let path = self.root.join(entry);
            if path.is_dir() {
                collect(&self.root, &path, &mut files)?;
            } else if path.is_file() {
                files.insert(
                    entry.to_string(),
                    fs::read(&path).map_err(|e| format!("{entry}: {e}"))?,
                );
            }
        }
        Ok(files)
    }

    fn build_rust(&self, release: bool) -> Result<Vec<u8>> {
        let target_dir = shared_target();
        let mut build = cargo(&self.root, target_dir.as_deref());
        build.args(["build", "--lib", "--target", "wasm32-unknown-unknown"]);
        if release {
            build.arg("--release");
        }
        let status = build
            .status()
            .map_err(|e| format!("Cannot run cargo: {e}"))?;
        if !status.success() {
            return Err("cargo build failed. Install the target with: rustup target add wasm32-unknown-unknown".into());
        }
        let output = cargo(&self.root, target_dir.as_deref())
            .args(["metadata", "--no-deps", "--format-version", "1"])
            .output()
            .map_err(|e| e.to_string())?;
        if !output.status.success() {
            return Err("cargo metadata failed".into());
        }
        let metadata: serde_json::Value =
            serde_json::from_slice(&output.stdout).map_err(|e| e.to_string())?;
        let manifest_path = self.root.join("Cargo.toml");
        let package = metadata["packages"]
            .as_array()
            .and_then(|list| {
                list.iter().find(|p| {
                    p["manifest_path"]
                        .as_str()
                        .and_then(|m| Path::new(m).canonicalize().ok())
                        == manifest_path.canonicalize().ok()
                })
            })
            .ok_or("Cargo.toml of this project is not in cargo metadata")?;
        let libs: Vec<_> = package["targets"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|t| {
                t["crate_types"]
                    .as_array()
                    .is_some_and(|k| k.iter().any(|c| c == "cdylib"))
            })
            .collect();
        let [lib] = libs.as_slice() else {
            return Err(
                "The crate needs exactly one library target with crate-type = [\"cdylib\"]".into(),
            );
        };
        let name = lib["name"].as_str().unwrap_or_default().replace('-', "_");
        let target = metadata["target_directory"]
            .as_str()
            .ok_or("cargo metadata has no target directory")?;
        let wasm = Path::new(target)
            .join("wasm32-unknown-unknown")
            .join(if release { "release" } else { "debug" })
            .join(format!("{name}.wasm"));
        fs::read(&wasm).map_err(|e| format!("{}: {e}", wasm.display()))
    }

    fn build_js(&self) -> Result<Vec<u8>> {
        let package = self.root.join("package.json");
        if package.is_file() {
            let text = fs::read_to_string(&package).map_err(|e| e.to_string())?;
            let json: serde_json::Value =
                serde_json::from_str(&text).map_err(|e| format!("package.json: {e}"))?;
            if json["scripts"]["build"].is_string() {
                let npm = if cfg!(windows) { "npm.cmd" } else { "npm" };
                let status = Command::new(npm)
                    .args(["run", "--silent", "build"])
                    .current_dir(&self.root)
                    .status()
                    .map_err(|e| format!("Cannot run npm: {e}"))?;
                if !status.success() {
                    return Err("npm run build failed".into());
                }
            }
        }
        let main = &self.manifest.runtime.main;
        fs::read(self.root.join(main)).map_err(|e| format!("{main}: {e}"))
    }
}

fn cargo(root: &Path, target_dir: Option<&Path>) -> Command {
    let mut command = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()));
    command.current_dir(root);
    if let Some(dir) = target_dir {
        command.env("CARGO_TARGET_DIR", dir);
    }
    command.env("CARGO_INCREMENTAL", "0");
    command
}

fn shared_target() -> Option<PathBuf> {
    if std::env::var_os("CARGO_TARGET_DIR").is_some()
        || std::env::var("OPENSD_SHARED_TARGET").is_ok_and(|v| v == "0")
    {
        return None;
    }
    let base = if cfg!(windows) {
        std::env::var_os("LOCALAPPDATA").map(PathBuf::from)
    } else if cfg!(target_os = "macos") {
        std::env::var_os("HOME").map(|h| PathBuf::from(h).join("Library/Caches"))
    } else {
        std::env::var_os("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))
    }?;
    Some(base.join("opensd").join("target"))
}

fn collect(root: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) -> Result {
    let mut entries: Vec<_> = fs::read_dir(dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?
        .flatten()
        .collect();
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') || SKIP.contains(&name.as_str()) {
            continue;
        }
        let path = entry.path();
        let kind = entry.file_type().map_err(|e| e.to_string())?;
        if kind.is_symlink() {
            return Err(format!(
                "{} is a link; packages carry plain files only",
                path.display()
            ));
        }
        if kind.is_dir() {
            collect(root, &path, out)?;
        } else if kind.is_file() {
            let rel = path
                .strip_prefix(root)
                .map_err(|e| e.to_string())?
                .to_string_lossy()
                .replace('\\', "/");
            if !opensd_package::safe_path(&rel) {
                return Err(format!("{rel} is not a portable file name"));
            }
            out.insert(
                rel,
                fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?,
            );
        }
    }
    Ok(())
}

fn write(path: &Path, data: &[u8]) -> Result {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    }
    fs::write(path, data).map_err(|e| format!("{}: {e}", path.display()))
}

pub fn plain(path: PathBuf) -> PathBuf {
    match path.to_str().and_then(|p| p.strip_prefix(r"\\?\")) {
        Some(rest) if !rest.starts_with("UNC") => PathBuf::from(rest),
        _ => path,
    }
}

pub fn human_size(bytes: u64) -> String {
    match bytes {
        b if b >= 1 << 20 => format!("{:.1} MiB", b as f64 / f64::from(1 << 20)),
        b if b >= 1 << 10 => format!("{:.1} KiB", b as f64 / 1024.0),
        b => format!("{b} B"),
    }
}
