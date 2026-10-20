//! Building, verifying and extracting `.opensd` archives.

use crate::manifest::{Manifest, RuntimeKind, semver_at_least};
use crate::paths::safe_path;
use crate::{EXTENSION, Error, FORMAT_VERSION, Result, wasm};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{Cursor, Read, Write};
use std::path::Path;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

pub const MAX_ENTRIES: usize = 2048;
pub const MAX_TOTAL_BYTES: u64 = 64 * 1024 * 1024;
pub const MAX_FILE_BYTES: u64 = 16 * 1024 * 1024;
pub const MAX_SCRIPT_BYTES: usize = 8 * 1024 * 1024;
pub const MANIFEST: &str = "manifest.json";
pub const INDEX: &str = "index.json";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileDigest {
    pub size: u64,
    pub sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Index {
    pub format: String,
    pub format_version: u32,
    pub id: String,
    pub version: String,
    pub files: BTreeMap<String, FileDigest>,
}

/// A package whose every file matched its index.
#[derive(Clone, Debug)]
pub struct Package {
    pub manifest: Manifest,
    pub index: Index,
    /// Every file except `index.json`, including `manifest.json`.
    pub files: BTreeMap<String, Vec<u8>>,
}

pub fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// Builds an archive from the plugin files (the code and resources, without
/// `manifest.json` and `index.json`). The WASM module is prepared for the
/// sandbox; the result is verified before it is returned.
pub fn build(manifest: &Manifest, mut files: BTreeMap<String, Vec<u8>>) -> Result<Vec<u8>> {
    let mut manifest = manifest.clone();
    manifest.package = None;
    manifest.validate()?;
    ensure!(
        !files.contains_key(MANIFEST) && !files.contains_key(INDEX),
        "{MANIFEST} and {INDEX} are written by the packer"
    );
    if manifest.runtime.kind == RuntimeKind::Wasm {
        let main = &manifest.runtime.main;
        let module = files
            .get(main)
            .ok_or_else(|| Error(format!("Missing {main}")))?;
        let prepared = wasm::prepare(module)?;
        files.insert(main.clone(), prepared);
    }
    files.insert(MANIFEST.into(), serde_json::to_vec_pretty(&manifest)?);
    check_files(&manifest, &files)?;
    let index = Index {
        format: EXTENSION.into(),
        format_version: FORMAT_VERSION,
        id: manifest.id.clone(),
        version: manifest.version.clone(),
        files: files
            .iter()
            .map(|(name, bytes)| {
                (
                    name.clone(),
                    FileDigest {
                        size: bytes.len() as u64,
                        sha256: sha256(bytes),
                    },
                )
            })
            .collect(),
    };
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .unix_permissions(0o644);
    writer.start_file(MANIFEST, options)?;
    writer.write_all(&files[MANIFEST])?;
    writer.start_file(INDEX, options)?;
    writer.write_all(&serde_json::to_vec_pretty(&index)?)?;
    for (name, bytes) in files.iter().filter(|(name, _)| *name != MANIFEST) {
        writer.start_file(name.as_str(), options)?;
        writer.write_all(bytes)?;
    }
    let bytes = writer.finish()?.into_inner();
    verify(&bytes)?;
    Ok(bytes)
}

/// Reads and verifies a package file.
pub fn read(path: &Path) -> Result<Package> {
    ensure!(
        fs::metadata(path)?.len() <= MAX_TOTAL_BYTES + 1024 * 1024,
        "Package exceeds 64 MiB"
    );
    verify(&fs::read(path)?)
}

/// Verifies an archive: layout, paths, index, hashes, manifest and code.
pub fn verify(bytes: &[u8]) -> Result<Package> {
    let mut archive = ZipArchive::new(Cursor::new(bytes))?;
    ensure!(
        archive.len() >= 3 && archive.len() <= MAX_ENTRIES,
        "A package holds 3 to {MAX_ENTRIES} files"
    );
    let mut files = BTreeMap::new();
    let mut seen = BTreeSet::new();
    let mut total = 0u64;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let name = entry.name()?.to_string();
        ensure!(
            !entry.is_dir() && safe_path(&name),
            "Unsafe archive entry {name}"
        );
        ensure!(
            seen.insert(name.to_lowercase()),
            "Duplicate or case-colliding entry {name}"
        );
        let mode = entry.unix_mode().unwrap_or(0) & 0o170000;
        ensure!(
            mode == 0 || mode == 0o100000,
            "Links and special entries are forbidden: {name}"
        );
        ensure!(entry.size() <= MAX_FILE_BYTES, "{name} exceeds 16 MiB");
        total = total.saturating_add(entry.size());
        ensure!(total <= MAX_TOTAL_BYTES, "Package contents exceed 64 MiB");
        let mut data = Vec::with_capacity(entry.size() as usize);
        entry
            .by_ref()
            .take(MAX_FILE_BYTES + 1)
            .read_to_end(&mut data)?;
        ensure!(data.len() as u64 == entry.size(), "Size mismatch in {name}");
        files.insert(name, data);
    }
    let index_bytes = files
        .remove(INDEX)
        .ok_or_else(|| Error(format!("Missing {INDEX}")))?;
    let index: Index =
        serde_json::from_slice(&index_bytes).map_err(|e| Error(format!("{INDEX}: {e}")))?;
    ensure!(
        index.format == EXTENSION && index.format_version == FORMAT_VERSION,
        "Unsupported package format {} {}",
        index.format,
        index.format_version
    );
    ensure!(
        files.len() == index.files.len(),
        "The archive and its index list different files"
    );
    for (name, data) in &files {
        let digest = index
            .files
            .get(name)
            .ok_or_else(|| Error(format!("{name} is not in the index")))?;
        ensure!(
            digest.size == data.len() as u64 && digest.sha256 == sha256(data),
            "{name} does not match its hash"
        );
    }
    let manifest = Manifest::from_json(
        files
            .get(MANIFEST)
            .ok_or_else(|| Error(format!("Missing {MANIFEST}")))?,
    )?;
    ensure!(
        manifest.package.is_none(),
        "{MANIFEST} must not contain packing options"
    );
    ensure!(
        manifest.id == index.id && manifest.version == index.version,
        "The manifest and the index name different plugins"
    );
    check_files(&manifest, &files)?;
    Ok(Package {
        manifest,
        index,
        files,
    })
}

fn check_files(manifest: &Manifest, files: &BTreeMap<String, Vec<u8>>) -> Result<()> {
    ensure!(
        files.len() < MAX_ENTRIES,
        "A package holds at most {MAX_ENTRIES} files"
    );
    let mut total = 0u64;
    let mut seen = BTreeSet::new();
    for (name, data) in files {
        ensure!(safe_path(name), "Unsafe file path {name}");
        ensure!(
            seen.insert(name.to_lowercase()),
            "Case-colliding file {name}"
        );
        ensure!(data.len() as u64 <= MAX_FILE_BYTES, "{name} exceeds 16 MiB");
        total += data.len() as u64;
        ensure!(
            !native_executable(data),
            "{name} is a native executable; packages carry no native code"
        );
    }
    ensure!(total <= MAX_TOTAL_BYTES, "Package contents exceed 64 MiB");
    for path in manifest.referenced_files() {
        ensure!(
            files.contains_key(path),
            "The manifest refers to {path}, which is not in the package"
        );
    }
    let main = &files[&manifest.runtime.main];
    match manifest.runtime.kind {
        RuntimeKind::Wasm => {
            wasm::verify(main)?;
        }
        RuntimeKind::Js => {
            ensure!(
                main.len() <= MAX_SCRIPT_BYTES,
                "{} exceeds 8 MiB",
                manifest.runtime.main
            );
            let text = std::str::from_utf8(main)
                .map_err(|_| Error(format!("{} is not UTF-8", manifest.runtime.main)))?;
            ensure!(
                !text.contains('\0'),
                "{} contains NUL characters",
                manifest.runtime.main
            );
        }
    }
    Ok(())
}

fn native_executable(data: &[u8]) -> bool {
    const MAGICS: [&[u8]; 6] = [
        b"MZ",
        b"\x7fELF",
        b"\xcf\xfa\xed\xfe",
        b"\xce\xfa\xed\xfe",
        b"\xfe\xed\xfa\xcf",
        b"\xca\xfe\xba\xbe",
    ];
    MAGICS.iter().any(|magic| data.starts_with(magic))
}

impl Package {
    /// Whether the app at `version` may run this plugin.
    pub fn compatible_with(&self, app_version: &str) -> Result<()> {
        if let Some(min) = &self.manifest.min_app {
            ensure!(
                semver_at_least(app_version, min),
                "{} needs Open Stream Deck {min} or newer",
                self.manifest.name
            );
        }
        Ok(())
    }

    /// Writes the files, including `index.json`, into a new directory.
    pub fn extract(&self, destination: &Path) -> Result<()> {
        ensure!(
            !destination.exists(),
            "{} already exists",
            destination.display()
        );
        fs::create_dir_all(destination)?;
        let result = (|| -> Result<()> {
            for (name, data) in &self.files {
                let file = destination.join(name);
                if let Some(parent) = file.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(file, data)?;
            }
            fs::write(
                destination.join(INDEX),
                serde_json::to_vec_pretty(&self.index)?,
            )?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_dir_all(destination);
        }
        result
    }
}

/// Reads the manifest of an installed or staged plugin folder and checks the
/// files it refers to. Installed folders also have an index; when it is there,
/// every listed file must still match it.
pub fn read_dir(root: &Path) -> Result<Manifest> {
    let manifest = Manifest::from_json(&fs::read(root.join(MANIFEST))?)?;
    ensure!(
        manifest.package.is_none(),
        "{MANIFEST} must not contain packing options"
    );
    let index_path = root.join(INDEX);
    let mut files = BTreeMap::new();
    if index_path.is_file() {
        let index: Index = serde_json::from_slice(&fs::read(&index_path)?)?;
        ensure!(
            index.id == manifest.id && index.version == manifest.version,
            "Installed files belong to another plugin"
        );
        for (name, digest) in &index.files {
            ensure!(safe_path(name), "Unsafe file path {name}");
            let data = fs::read(root.join(name)).map_err(|e| Error(format!("{name}: {e}")))?;
            ensure!(
                data.len() as u64 == digest.size && sha256(&data) == digest.sha256,
                "{name} was modified after installation"
            );
            files.insert(name.clone(), data);
        }
    } else {
        for path in manifest.referenced_files() {
            let data = fs::read(root.join(path)).map_err(|e| Error(format!("{path}: {e}")))?;
            files.insert(path.to_string(), data);
        }
    }
    for path in manifest.referenced_files() {
        ensure!(files.contains_key(path), "Missing {path}");
    }
    let main = &files[&manifest.runtime.main];
    match manifest.runtime.kind {
        RuntimeKind::Wasm => {
            wasm::verify(main)?;
        }
        RuntimeKind::Js => ensure!(
            std::str::from_utf8(main).is_ok(),
            "{} is not UTF-8",
            manifest.runtime.main
        ),
    }
    Ok(manifest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wasm::tests::module;

    fn manifest() -> Manifest {
        serde_json::from_value(serde_json::json!({
            "sdk": 1, "id": "com.example.test", "name": "Test", "version": "1.0.0", "author": "Me",
            "icon": "assets/icon.svg",
            "runtime": { "type": "wasm", "main": "plugin.wasm" },
            "actions": [{ "id": "com.example.test.one", "name": "One" }]
        }))
        .unwrap()
    }

    fn files() -> BTreeMap<String, Vec<u8>> {
        BTreeMap::from([
            (
                "plugin.wasm".to_string(),
                module(1, None, ("opensd", "send")),
            ),
            ("assets/icon.svg".to_string(), b"<svg/>".to_vec()),
        ])
    }

    #[test]
    fn build_verify_extract_roundtrip() {
        let bytes = build(&manifest(), files()).unwrap();
        let package = verify(&bytes).unwrap();
        assert_eq!(package.manifest.id, "com.example.test");
        assert!(package.files.contains_key(MANIFEST));
        let dir = std::env::temp_dir().join(format!("opensd-roundtrip-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        package.extract(&dir).unwrap();
        assert!(package.extract(&dir).is_err(), "never overwrites");
        assert_eq!(read_dir(&dir).unwrap().id, "com.example.test");
        fs::write(dir.join("assets/icon.svg"), b"<svg>changed</svg>").unwrap();
        assert!(
            read_dir(&dir).is_err(),
            "tampering after install is detected"
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn rejects_missing_files_and_native_code() {
        let mut f = files();
        f.remove("assets/icon.svg");
        assert!(build(&manifest(), f).is_err());
        let mut f = files();
        f.insert("assets/tool.dat".into(), b"MZ\x90\x00 disguised".to_vec());
        assert!(build(&manifest(), f).is_err());
        let mut f = files();
        f.insert("plugin.wasm".into(), module(1, None, ("env", "abort")));
        assert!(build(&manifest(), f).is_err());
    }

    #[test]
    fn rejects_tampered_archives() {
        let package = verify(&build(&manifest(), files()).unwrap()).unwrap();
        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        for (name, data) in &package.files {
            writer
                .start_file(name.as_str(), SimpleFileOptions::default())
                .unwrap();
            let data = if name == "assets/icon.svg" {
                b"<svg>evil</svg>".as_slice()
            } else {
                data
            };
            writer.write_all(data).unwrap();
        }
        writer
            .start_file(INDEX, SimpleFileOptions::default())
            .unwrap();
        writer
            .write_all(&serde_json::to_vec(&package.index).unwrap())
            .unwrap();
        let error = verify(&writer.finish().unwrap().into_inner()).unwrap_err();
        assert!(error.0.contains("does not match"), "{error}");
    }

    #[test]
    fn js_packages_carry_readable_source() {
        let mut m = manifest();
        m.runtime.kind = RuntimeKind::Js;
        m.runtime.main = "plugin.js".into();
        let f = BTreeMap::from([
            (
                "plugin.js".to_string(),
                b"opensd.on('keyDown', () => {});".to_vec(),
            ),
            ("assets/icon.svg".to_string(), b"<svg/>".to_vec()),
        ]);
        let package = verify(&build(&m, f).unwrap()).unwrap();
        assert_eq!(
            package.files["plugin.js"],
            b"opensd.on('keyDown', () => {});"
        );
    }
}
