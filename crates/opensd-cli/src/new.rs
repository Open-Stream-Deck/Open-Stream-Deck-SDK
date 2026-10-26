use crate::Result;
use std::fs;
use std::path::{Path, PathBuf};

macro_rules! template {
    ($path:literal) => {
        include_str!(concat!("../templates/", $path))
    };
}

const ICON: &str = template!("shared/assets/icon.svg");
const KEY: &str = template!("shared/assets/key.svg");
const ACTIONS: &str = template!("shared/actions.toml");
const TYPES: &str = include_str!("../types/opensd.d.ts");

pub struct Options {
    pub lang: String,
    pub id: Option<String>,
    pub name: Option<String>,
    pub sdk_path: Option<PathBuf>,
}

pub fn create(dir: &Path, options: Options) -> Result {
    let occupied = dir.exists()
        && fs::read_dir(dir)
            .map(|mut entries| entries.next().is_some())
            .unwrap_or(true);
    if occupied {
        return Err(format!("{} already exists and is not empty", dir.display()));
    }
    let slug = slug(dir.file_name().and_then(|n| n.to_str()).unwrap_or_default());
    let id = options.id.unwrap_or_else(|| format!("com.example.{slug}"));
    if !opensd_package::valid_id(&id) {
        return Err(format!(
            "{id} is not a valid id; use a lowercase reverse-domain name such as com.example.{slug}"
        ));
    }
    let name = options.name.unwrap_or_else(|| title(&slug));
    let author = std::env::var("USERNAME")
        .or_else(|_| std::env::var("USER"))
        .unwrap_or_else(|_| "Plugin Author".into());
    let dependency = sdk_dependency(options.sdk_path.as_deref())?;
    let fill = |text: &str| {
        text.replace("{{id}}", &id)
            .replace("{{name}}", &name.replace('"', "'"))
            .replace("{{author}}", &author.replace('"', "'"))
            .replace("{{crate}}", &slug.replace('-', "_"))
            .replace("{{package}}", &slug)
            .replace("{{sdk}}", &dependency)
    };
    let manifest = |head: &str| fill(head) + &fill(ACTIONS);
    let files: Vec<(&str, String)> = match options.lang.as_str() {
        "rust" | "rs" => vec![
            ("opensd.toml", manifest(template!("rust/opensd.toml"))),
            ("Cargo.toml", fill(template!("rust/Cargo.toml.template"))),
            ("src/lib.rs", fill(template!("rust/src/lib.rs"))),
            (".gitignore", template!("rust/gitignore").into()),
        ],
        "js" | "javascript" => vec![
            ("opensd.toml", manifest(template!("js/opensd.toml"))),
            ("plugin.js", fill(template!("js/plugin.js"))),
            ("jsconfig.json", template!("js/jsconfig.json").into()),
            ("opensd.d.ts", TYPES.into()),
            (".gitignore", template!("js/gitignore").into()),
        ],
        "ts" | "typescript" => vec![
            ("opensd.toml", manifest(template!("ts/opensd.toml"))),
            ("src/plugin.ts", fill(template!("ts/src/plugin.ts"))),
            ("package.json", fill(template!("ts/package.json"))),
            ("tsconfig.json", template!("ts/tsconfig.json").into()),
            ("opensd.d.ts", TYPES.into()),
            (".gitignore", template!("ts/gitignore").into()),
        ],
        other => return Err(format!("Unknown language {other}; use rust, js or ts")),
    };
    let assets = [("assets/icon.svg", ICON), ("assets/key.svg", KEY)];
    for (path, text) in files.iter().map(|(p, t)| (*p, t.as_str())).chain(assets) {
        let target = dir.join(path);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::write(&target, text).map_err(|e| format!("{}: {e}", target.display()))?;
    }
    let next = match options.lang.as_str() {
        "rust" | "rs" => "rustup target add wasm32-unknown-unknown, then opensd dev",
        "ts" | "typescript" => "npm install, then npm run dev",
        _ => "opensd dev",
    };
    println!(
        "Created {name} ({id}) in {}\nNext: cd into it and run {next}",
        dir.display()
    );
    Ok(())
}

fn slug(folder: &str) -> String {
    let slug = folder
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    if slug.is_empty() {
        "plugin".into()
    } else {
        slug
    }
}

fn title(slug: &str) -> String {
    slug.split('-')
        .filter_map(|word| {
            let mut chars = word.chars();
            chars
                .next()
                .map(|first| first.to_ascii_uppercase().to_string() + chars.as_str())
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn sdk_dependency(local: Option<&Path>) -> Result<String> {
    let Some(local) = local else {
        let mut version = env!("CARGO_PKG_VERSION").split('.');
        let (major, minor) = (version.next().unwrap_or("1"), version.next().unwrap_or("0"));
        return Ok(format!("\"{major}.{minor}\""));
    };
    let path = local
        .canonicalize()
        .map_err(|e| format!("{}: {e}", local.display()))?;
    let crate_dir = [path.join("crates/opensd"), path.clone()]
        .into_iter()
        .find(|dir| {
            fs::read_to_string(dir.join("Cargo.toml"))
                .is_ok_and(|text| text.contains("name = \"opensd\""))
        })
        .ok_or_else(|| {
            format!(
                "{} is not the opensd crate or the SDK repository",
                local.display()
            )
        })?;
    let text = crate_dir
        .to_string_lossy()
        .trim_start_matches(r"\\?\")
        .replace('\\', "/");
    Ok(format!("{{ path = {text:?} }}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_from_folders() {
        assert_eq!(slug("My Cool_Plugin!"), "my-cool-plugin");
        assert_eq!(slug("..."), "plugin");
        assert_eq!(title("my-cool-plugin"), "My Cool Plugin");
    }

    #[test]
    fn projects_validate_in_every_language() {
        let root = std::env::temp_dir().join(format!("opensd-new-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        for lang in ["rust", "js", "ts"] {
            let dir = root.join(format!("demo-{lang}"));
            let options = Options {
                lang: lang.into(),
                id: None,
                name: None,
                sdk_path: None,
            };
            create(&dir, options).unwrap();
            let project = crate::project::Project::open(&dir).unwrap();
            assert_eq!(project.manifest.id, format!("com.example.demo-{lang}"));
            project.check().unwrap();
        }
        let cargo = fs::read_to_string(root.join("demo-rust/Cargo.toml")).unwrap();
        assert!(cargo.contains("opensd = \"1.1\""), "{cargo}");
        let package = fs::read_to_string(root.join("demo-ts/package.json")).unwrap();
        assert!(package.contains("\"name\": \"demo-ts\""), "{package}");
        fs::remove_dir_all(root).unwrap();
    }
}
