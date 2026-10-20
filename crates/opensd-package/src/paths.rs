//! Portable relative paths inside a package.

const RESERVED: [&str; 22] = [
    "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
    "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
];

/// A relative path with forward slashes that means the same file on Windows,
/// macOS and Linux and cannot leave the package root.
pub fn safe_path(path: &str) -> bool {
    !path.is_empty()
        && path.len() <= 240
        && !path.starts_with('/')
        && path.split('/').count() <= 16
        && path.split('/').all(safe_segment)
}

fn safe_segment(segment: &str) -> bool {
    !segment.is_empty()
        && segment != "."
        && segment != ".."
        && !segment.ends_with(['.', ' '])
        && !segment
            .chars()
            .any(|c| c.is_control() || matches!(c, '\\' | ':' | '<' | '>' | '"' | '|' | '?' | '*'))
        && !RESERVED.contains(
            &segment
                .split('.')
                .next()
                .unwrap_or_default()
                .to_ascii_lowercase()
                .as_str(),
        )
}

pub(crate) fn extension(path: &str) -> String {
    path.rsplit_once('.')
        .map(|(_, ext)| ext.to_ascii_lowercase())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_escapes_and_platform_names() {
        for path in [
            "", "/x", "../x", "a/../x", "a/./b", "a//b", "a\\b", "C:/x", "a:b", "NUL", "nul.txt",
            "a./b", "a /b", "a?b", "a\u{0}b",
        ] {
            assert!(!safe_path(path), "{path:?}");
        }
        for path in [
            "plugin.wasm",
            "assets/key.svg",
            "ui/fonts/Inter Variable.woff2",
        ] {
            assert!(safe_path(path), "{path:?}");
        }
    }
}
