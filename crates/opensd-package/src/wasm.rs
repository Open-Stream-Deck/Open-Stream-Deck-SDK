//! Checks a WebAssembly module against the SDK ABI, and prepares it for packing.
//!
//! A plugin module imports only `opensd.send`, exports `memory`, `opensd_abi`,
//! `opensd_alloc`, `opensd_free` and `opensd_event`, and declares one
//! non-shared 32-bit memory whose maximum is at most [`MAX_MEMORY_PAGES`].

use crate::Result;

/// 64 KiB pages: 1024 pages = 64 MiB.
pub const MAX_MEMORY_PAGES: u32 = 1024;
pub const MAX_MODULE_BYTES: usize = 16 * 1024 * 1024;
const MAGIC: &[u8; 8] = b"\0asm\x01\0\0\0";
const REQUIRED_EXPORTS: [(&str, u8); 5] = [
    ("memory", 2),
    ("opensd_abi", 0),
    ("opensd_alloc", 0),
    ("opensd_free", 0),
    ("opensd_event", 0),
];

/// What a valid module looks like from the outside.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModuleInfo {
    pub memory_min: u32,
    pub memory_max: Option<u32>,
}

struct Section {
    id: u8,
    start: usize,
    end: usize,
    body: std::ops::Range<usize>,
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8], at: usize) -> Self {
        Self { bytes, at }
    }

    fn byte(&mut self) -> Result<u8> {
        let Some(&b) = self.bytes.get(self.at) else {
            bail!("Truncated WebAssembly module")
        };
        self.at += 1;
        Ok(b)
    }

    fn u32(&mut self) -> Result<u32> {
        let mut value: u64 = 0;
        for shift in (0..35).step_by(7) {
            let b = self.byte()?;
            value |= u64::from(b & 0x7f) << shift;
            if b & 0x80 == 0 {
                ensure!(
                    value <= u64::from(u32::MAX),
                    "Malformed WebAssembly integer"
                );
                return Ok(value as u32);
            }
        }
        bail!("Malformed WebAssembly integer")
    }

    fn name(&mut self) -> Result<&'a str> {
        let len = self.u32()? as usize;
        let end = self.at.checked_add(len).filter(|&e| e <= self.bytes.len());
        let Some(end) = end else {
            bail!("Truncated WebAssembly name")
        };
        let text = std::str::from_utf8(&self.bytes[self.at..end])
            .map_err(|_| crate::Error("WebAssembly name is not UTF-8".into()))?;
        self.at = end;
        Ok(text)
    }

    fn limits(&mut self) -> Result<(u8, u32, Option<u32>)> {
        let flags = self.byte()?;
        let min = self.u32()?;
        let max = if flags & 1 == 1 {
            Some(self.u32()?)
        } else {
            None
        };
        Ok((flags, min, max))
    }
}

fn sections(bytes: &[u8]) -> Result<Vec<Section>> {
    ensure!(
        bytes.len() <= MAX_MODULE_BYTES,
        "WebAssembly module exceeds 16 MiB"
    );
    ensure!(
        bytes.starts_with(MAGIC),
        "Not a WebAssembly 1.0 core module"
    );
    let mut out = Vec::new();
    let mut reader = Reader::new(bytes, MAGIC.len());
    while reader.at < bytes.len() {
        let start = reader.at;
        let id = reader.byte()?;
        let size = reader.u32()? as usize;
        let body = reader.at
            ..reader
                .at
                .checked_add(size)
                .filter(|&e| e <= bytes.len())
                .ok_or_else(|| crate::Error("Truncated WebAssembly section".into()))?;
        reader.at = body.end;
        out.push(Section {
            id,
            start,
            end: body.end,
            body,
        });
    }
    Ok(out)
}

/// Validates the module against the SDK ABI.
pub fn inspect(bytes: &[u8]) -> Result<ModuleInfo> {
    let mut memory = None;
    let mut exports = Vec::new();
    for section in sections(bytes)? {
        let mut r = Reader::new(bytes, section.body.start);
        match section.id {
            2 => {
                for _ in 0..r.u32()? {
                    let module = r.name()?;
                    let name = r.name()?;
                    let kind = r.byte()?;
                    ensure!(
                        module == "opensd" && name == "send" && kind == 0,
                        "Forbidden import {module}.{name}: a plugin may import only opensd.send"
                    );
                    r.u32()?;
                }
            }
            5 => {
                ensure!(r.u32()? == 1, "A plugin module declares exactly one memory");
                let (flags, min, max) = r.limits()?;
                ensure!(
                    flags & !1 == 0,
                    "Shared and 64-bit memories are not supported"
                );
                memory = Some((min, max));
            }
            7 => {
                for _ in 0..r.u32()? {
                    let name = r.name()?.to_string();
                    let kind = r.byte()?;
                    r.u32()?;
                    exports.push((name, kind));
                }
            }
            _ => {}
        }
    }
    for (name, kind) in REQUIRED_EXPORTS {
        ensure!(
            exports.iter().any(|(n, k)| n == name && *k == kind),
            "Missing export {name}; build the plugin with the opensd SDK"
        );
    }
    let Some((min, max)) = memory else {
        bail!("The module must define its own memory")
    };
    ensure!(min <= MAX_MEMORY_PAGES, "Initial memory exceeds 64 MiB");
    Ok(ModuleInfo {
        memory_min: min,
        memory_max: max,
    })
}

/// Validates a module that is about to be installed: the ABI, plus a memory
/// maximum within the limit (the packer sets one).
pub fn verify(bytes: &[u8]) -> Result<ModuleInfo> {
    let info = inspect(bytes)?;
    ensure!(
        info.memory_max.is_some_and(|max| max <= MAX_MEMORY_PAGES),
        "Module memory must declare a maximum of at most 64 MiB; repack it with opensd pack"
    );
    Ok(info)
}

/// Returns the module ready for a package: the memory maximum clamped to the
/// limit and DWARF debug sections removed. Other sections are kept as they are.
pub fn prepare(bytes: &[u8]) -> Result<Vec<u8>> {
    let info = inspect(bytes)?;
    let max = info
        .memory_max
        .unwrap_or(MAX_MEMORY_PAGES)
        .min(MAX_MEMORY_PAGES)
        .max(info.memory_min);
    let mut out = Vec::with_capacity(bytes.len());
    out.extend_from_slice(MAGIC);
    for section in sections(bytes)? {
        match section.id {
            0 => {
                let name = Reader::new(bytes, section.body.start).name()?;
                if !name.starts_with(".debug") {
                    out.extend_from_slice(&bytes[section.start..section.end]);
                }
            }
            5 => {
                let mut body = Vec::new();
                leb(&mut body, 1);
                body.push(1);
                leb(&mut body, info.memory_min);
                leb(&mut body, max);
                out.push(5);
                leb(&mut out, body.len() as u32);
                out.extend_from_slice(&body);
            }
            _ => out.extend_from_slice(&bytes[section.start..section.end]),
        }
    }
    verify(&out)?;
    Ok(out)
}

fn leb(out: &mut Vec<u8>, mut value: u32) {
    loop {
        let byte = (value & 0x7f) as u8;
        value >>= 7;
        if value == 0 {
            out.push(byte);
            return;
        }
        out.push(byte | 0x80);
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn module(min: u32, max: Option<u32>, import: (&str, &str)) -> Vec<u8> {
        fn section(out: &mut Vec<u8>, id: u8, body: &[u8]) {
            out.push(id);
            leb(out, body.len() as u32);
            out.extend_from_slice(body);
        }
        fn name(out: &mut Vec<u8>, text: &str) {
            leb(out, text.len() as u32);
            out.extend_from_slice(text.as_bytes());
        }
        let mut m = MAGIC.to_vec();
        section(
            &mut m,
            1,
            &[
                4, 0x60, 2, 0x7f, 0x7f, 1, 0x7f, 0x60, 0, 1, 0x7f, 0x60, 1, 0x7f, 1, 0x7f, 0x60, 2,
                0x7f, 0x7f, 0,
            ],
        );
        let mut imports = vec![1];
        name(&mut imports, import.0);
        name(&mut imports, import.1);
        imports.extend([0, 0]);
        section(&mut m, 2, &imports);
        section(&mut m, 3, &[4, 1, 2, 3, 0]);
        let mut memory = vec![1];
        match max {
            Some(max) => {
                memory.push(1);
                leb(&mut memory, min);
                leb(&mut memory, max);
            }
            None => {
                memory.push(0);
                leb(&mut memory, min);
            }
        }
        section(&mut m, 5, &memory);
        let mut exports = vec![5];
        for (n, kind, index) in [
            ("memory", 2, 0),
            ("opensd_abi", 0, 1),
            ("opensd_alloc", 0, 2),
            ("opensd_free", 0, 3),
            ("opensd_event", 0, 4),
        ] {
            name(&mut exports, n);
            exports.extend([kind, index]);
        }
        section(&mut m, 7, &exports);
        section(
            &mut m,
            10,
            &[
                4, 4, 0, 0x41, 1, 0x0b, 4, 0, 0x41, 0, 0x0b, 2, 0, 0x0b, 4, 0, 0x41, 0, 0x0b,
            ],
        );
        let mut custom = Vec::new();
        name(&mut custom, ".debug_info");
        custom.extend([1, 2, 3]);
        section(&mut m, 0, &custom);
        m
    }

    #[test]
    fn prepare_sets_memory_limit_and_strips_debug() {
        let raw = module(17, None, ("opensd", "send"));
        assert_eq!(inspect(&raw).unwrap().memory_max, None);
        assert!(verify(&raw).is_err());
        let ready = prepare(&raw).unwrap();
        assert_eq!(
            verify(&ready).unwrap(),
            ModuleInfo {
                memory_min: 17,
                memory_max: Some(MAX_MEMORY_PAGES)
            }
        );
        assert!(ready.len() < raw.len(), "debug section removed");
        let small = prepare(&module(2, Some(16), ("opensd", "send"))).unwrap();
        assert_eq!(verify(&small).unwrap().memory_max, Some(16));
    }

    #[test]
    fn rejects_foreign_imports_and_huge_memory() {
        assert!(inspect(&module(1, None, ("env", "abort"))).is_err());
        assert!(inspect(&module(1, None, ("wasi_snapshot_preview1", "fd_write"))).is_err());
        assert!(inspect(&module(2000, None, ("opensd", "send"))).is_err());
        assert!(inspect(b"MZ\x90\x00").is_err());
    }
}
