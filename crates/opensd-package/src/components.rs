use crate::{Result, safe_path};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentTarget {
    pub id: String,
    pub name: String,
    pub selection: TargetSelection,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub os: Option<ComponentOs>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_files: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub discovery: Vec<TargetDiscovery>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<KnownLocation>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub after_install: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetSelection {
    Directory,
    Executable,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum TargetDiscovery {
    Steam { app_id: String },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KnownLocation {
    pub base: KnownDirectory,
    pub path: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnownDirectory {
    AppData,
    LocalAppData,
    Config,
    Data,
    ApplicationSupport,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalComponent {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub targets: Vec<String>,
    pub variants: Vec<ComponentVariant>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentVariant {
    pub os: ComponentOs,
    pub arch: ComponentArch,
    pub files: Vec<ComponentFile>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ComponentOs {
    Windows,
    Linux,
    Macos,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComponentArch {
    X86_64,
    Aarch64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentFile {
    pub source: String,
    pub destination: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub executable: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComponentPlatform {
    pub os: ComponentOs,
    pub arch: ComponentArch,
}

impl ComponentPlatform {
    pub fn current() -> Option<Self> {
        let os = match std::env::consts::OS {
            "windows" => ComponentOs::Windows,
            "linux" => ComponentOs::Linux,
            "macos" => ComponentOs::Macos,
            _ => return None,
        };
        let arch = match std::env::consts::ARCH {
            "x86_64" => ComponentArch::X86_64,
            "aarch64" => ComponentArch::Aarch64,
            _ => return None,
        };
        Some(Self { os, arch })
    }
}

impl ExternalComponent {
    pub fn variant(&self, platform: ComponentPlatform) -> Option<&ComponentVariant> {
        self.variants
            .iter()
            .find(|v| v.os == platform.os && v.arch == platform.arch)
    }
}

pub fn validate_components(
    targets: &[ComponentTarget],
    components: &[ExternalComponent],
) -> Result<()> {
    ensure!(
        targets.len() <= 16 && components.len() <= 16,
        "At most 16 targets and components are allowed"
    );
    ensure!(
        targets.is_empty() == components.is_empty(),
        "targets and components must be declared together"
    );
    let mut target_ids = BTreeSet::new();
    for target in targets {
        validate_id(&target.id)?;
        validate_text(&target.name, 64)?;
        ensure!(
            target_ids.insert(&target.id),
            "Duplicate target {}",
            target.id
        );
        ensure!(
            target.after_install.len() <= 1024,
            "after_install exceeds 1024 bytes"
        );
        ensure!(
            target.required_files.len() <= 32 && target.discovery.len() <= 8,
            "Too many target validation or discovery rules"
        );
        for path in &target.required_files {
            ensure!(safe_path(path), "Unsafe target validation path {path}");
        }
        if let Some(location) = &target.location {
            ensure!(
                safe_path(&location.path),
                "Unsafe known-directory path {}",
                location.path
            );
        }
        for discovery in &target.discovery {
            match discovery {
                TargetDiscovery::Steam { app_id } => ensure!(
                    !app_id.is_empty()
                        && app_id.len() <= 10
                        && app_id.bytes().all(|b| b.is_ascii_digit()),
                    "Steam app_id must contain 1 to 10 digits"
                ),
            }
        }
    }
    let mut component_ids = BTreeSet::new();
    for component in components {
        validate_id(&component.id)?;
        validate_text(&component.name, 64)?;
        validate_text(&component.description, 512)?;
        ensure!(
            component_ids.insert(&component.id),
            "Duplicate component {}",
            component.id
        );
        ensure!(
            crate::manifest::valid_semver(&component.version),
            "Invalid component version {}",
            component.version
        );
        ensure!(
            !component.targets.is_empty() && component.targets.len() <= 16,
            "A component needs 1 to 16 targets"
        );
        let mut used_targets = BTreeSet::new();
        for target in &component.targets {
            ensure!(
                target_ids.contains(target),
                "Unknown component target {target}"
            );
            ensure!(
                used_targets.insert(target),
                "Duplicate component target {target}"
            );
        }
        ensure!(
            !component.variants.is_empty() && component.variants.len() <= 6,
            "A component needs 1 to 6 variants"
        );
        let mut platforms = BTreeSet::new();
        for variant in &component.variants {
            ensure!(
                targets
                    .iter()
                    .any(|t| component.targets.contains(&t.id)
                        && t.os.is_none_or(|os| os == variant.os)),
                "Component variant has no matching target"
            );
            ensure!(
                platforms.insert((variant.os, variant.arch)),
                "Duplicate component platform"
            );
            ensure!(
                !variant.files.is_empty() && variant.files.len() <= 128,
                "A component variant needs 1 to 128 files"
            );
            let mut destinations = BTreeSet::new();
            for file in &variant.files {
                ensure!(
                    safe_path(&file.source),
                    "Unsafe component source {}",
                    file.source
                );
                ensure!(
                    safe_path(&file.destination),
                    "Unsafe component destination {}",
                    file.destination
                );
                let path = file.destination.to_lowercase();
                ensure!(
                    path != "opensd-install.txt",
                    "OPENSD-INSTALL.txt is reserved for export instructions"
                );
                ensure!(
                    !destinations.iter().any(|other: &String| other == &path
                        || other.starts_with(&format!("{path}/"))
                        || path.starts_with(&format!("{other}/"))),
                    "Overlapping component destination {}",
                    file.destination
                );
                destinations.insert(path);
            }
        }
    }
    Ok(())
}

fn validate_id(id: &str) -> Result<()> {
    ensure!(
        !id.is_empty()
            && id.len() <= 64
            && id
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-'),
        "Component and target ids must use lowercase letters, digits and hyphens"
    );
    Ok(())
}

fn validate_text(value: &str, max: usize) -> Result<()> {
    ensure!(
        !value.trim().is_empty() && value.len() <= max && !value.chars().any(char::is_control),
        "Invalid component text (limit {max} bytes)"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selects_exact_platform_without_falling_back_to_windows() {
        let mut component: ExternalComponent = serde_json::from_value(serde_json::json!({
            "id": "bridge", "name": "Bridge", "description": "Native bridge", "version": "1.0.0",
            "targets": ["app"], "variants": []
        }))
        .unwrap();
        for os in [ComponentOs::Windows, ComponentOs::Linux, ComponentOs::Macos] {
            for arch in [ComponentArch::X86_64, ComponentArch::Aarch64] {
                component.variants.push(ComponentVariant {
                    os,
                    arch,
                    files: vec![ComponentFile {
                        source: "payload/bridge".into(),
                        destination: "plugins/bridge".into(),
                        executable: true,
                    }],
                });
            }
        }
        for variant in &component.variants {
            let platform = ComponentPlatform {
                os: variant.os,
                arch: variant.arch,
            };
            assert_eq!(component.variant(platform), Some(variant));
        }
        component.variants.retain(|v| v.os == ComponentOs::Windows);
        assert!(
            component
                .variant(ComponentPlatform {
                    os: ComponentOs::Linux,
                    arch: ComponentArch::X86_64
                })
                .is_none()
        );
        let target: ComponentTarget = serde_json::from_value(serde_json::json!({
            "id": "app", "name": "App", "selection": "directory", "os": "macos"
        }))
        .unwrap();
        assert!(validate_components(&[target], &[component]).is_err());
    }
}
