//! UI kit and bridge for HTML pages of Open Stream Deck plugins.
//!
//! The app serves these files under `/_opensd/` to every plugin page, so a
//! page links `/_opensd/inspector.css` and `/_opensd/inspector.js` and looks
//! like the app without shipping its own styles. The kit is described in the
//! [UI kit guide](https://wiki.opensd.dev/docs/sdk/ui).

/// URL prefix the app serves the kit under.
pub const PREFIX: &str = "/_opensd/";

const FILES: [(&str, &str, &[u8]); 4] = [
    (
        "inspector.css",
        "text/css; charset=utf-8",
        include_bytes!("../assets/inspector.css"),
    ),
    (
        "inspector.js",
        "text/javascript; charset=utf-8",
        include_bytes!("../assets/inspector.js"),
    ),
    (
        "inter-latin.woff2",
        "font/woff2",
        include_bytes!("../assets/inter-latin.woff2"),
    ),
    (
        "inter-cyrillic.woff2",
        "font/woff2",
        include_bytes!("../assets/inter-cyrillic.woff2"),
    ),
];

/// Returns the MIME type and body of a kit file by its URL path,
/// such as `/_opensd/inspector.css`, or `None` for any other path.
pub fn file(path: &str) -> Option<(&'static str, &'static [u8])> {
    let name = path.strip_prefix(PREFIX)?;
    FILES
        .iter()
        .find(|(n, ..)| *n == name)
        .map(|(_, mime, body)| (*mime, *body))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serves_only_kit_files() {
        let (mime, css) = file("/_opensd/inspector.css").unwrap();
        assert!(mime.starts_with("text/css") && css.starts_with(b"@font-face"));
        assert!(
            file("/_opensd/inspector.js").is_some() && file("/_opensd/inter-latin.woff2").is_some()
        );
        for bad in [
            "/_opensd/other.js",
            "/inspector.css",
            "/_opensd/",
            "/_opensd/../inspector.css",
        ] {
            assert!(file(bad).is_none(), "{bad}");
        }
    }
}
