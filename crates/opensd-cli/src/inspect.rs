use crate::Result;
use crate::project::human_size;
use opensd_package::{RuntimeKind, wasm};
use std::path::Path;

pub fn run(path: &Path) -> Result {
    let package = opensd_package::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let m = &package.manifest;
    println!("{} {} by {}", m.name, m.version, m.author);
    println!("id        {}", m.id);
    if !m.description.is_empty() {
        println!("about     {}", m.description);
    }
    if let Some(homepage) = &m.homepage {
        println!("homepage  {homepage}");
    }
    if let Some(license) = &m.license {
        println!("license   {license}");
    }
    let main = &package.files[&m.runtime.main];
    match m.runtime.kind {
        RuntimeKind::Wasm => {
            let info = wasm::verify(main).map_err(|e| e.to_string())?;
            println!(
                "code      {} (WebAssembly, {}, memory {}..{} MiB, imports only opensd.send)",
                m.runtime.main,
                human_size(main.len() as u64),
                u64::from(info.memory_min) * 64 / 1024,
                u64::from(info.memory_max.unwrap_or(0)) * 64 / 1024
            );
        }
        RuntimeKind::Js => println!(
            "code      {} (JavaScript source, {})",
            m.runtime.main,
            human_size(main.len() as u64)
        ),
    }
    println!();
    println!("Permissions");
    let p = &m.permissions;
    if p.is_empty() {
        println!("  none: the plugin cannot reach anything outside its sandbox");
    }
    for range in &p.loopback {
        println!("  connect to programs on this computer, ports {range}");
    }
    for origin in &p.network {
        println!("  network access to {origin}");
    }
    if p.clipboard {
        println!("  write text to the clipboard");
    }
    if p.keyboard {
        println!("  press keys and type text");
    }
    if p.navigation {
        println!("  switch profiles and pages");
    }
    for target in &p.launch {
        let args = if target.args.is_empty() {
            String::new()
        } else {
            format!(" with {}", target.args.join(" "))
        };
        println!("  start and close {}{args}", target.name);
        for path in target
            .windows
            .iter()
            .chain(&target.macos)
            .chain(&target.linux)
        {
            println!("    {path}");
        }
    }
    println!("  digest {}", m.permissions_digest());
    println!();
    println!("Actions");
    for action in &m.actions {
        let controllers: Vec<_> = action
            .controllers
            .iter()
            .map(|c| format!("{c:?}").to_lowercase())
            .collect();
        println!(
            "  {}  {}  [{}]",
            action.id,
            action.name,
            controllers.join(", ")
        );
    }
    println!();
    println!("Files (all match index.json)");
    let mut total = 0;
    for (name, data) in &package.files {
        total += data.len() as u64;
        println!(
            "  {:>10}  {}  {name}",
            human_size(data.len() as u64),
            &opensd_package::sha256(data)[..12]
        );
    }
    println!("  {:>10}  total", human_size(total));
    Ok(())
}
