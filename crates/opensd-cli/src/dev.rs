use crate::Result;
use crate::project::{Build, Project};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::fs;
use std::io::{Read, Seek, SeekFrom, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

const APP_ID: &str = "com.whxtelxs.open-stream-deck";
const DISCOVERY: &str = "opensd-dev.json";
const IGNORED: [&str; 7] = [
    ".git",
    ".opensd",
    "dist",
    "target",
    "node_modules",
    ".idea",
    ".vscode",
];

struct Endpoint {
    port: u16,
    token: String,
}

fn endpoint(path: Option<PathBuf>) -> Result<Endpoint> {
    let candidates: Vec<PathBuf> = match path {
        Some(path) => vec![path],
        None => {
            let home = std::env::var_os("HOME").map(PathBuf::from);
            [
                std::env::var_os("APPDATA").map(|p| PathBuf::from(p).join(APP_ID)),
                home.as_ref()
                    .map(|h| h.join("Library/Application Support").join(APP_ID)),
                std::env::var_os("XDG_DATA_HOME").map(|p| PathBuf::from(p).join(APP_ID)),
                home.as_ref().map(|h| h.join(".local/share").join(APP_ID)),
            ]
            .into_iter()
            .flatten()
            .map(|dir| dir.join(DISCOVERY))
            .collect()
        }
    };
    let file = candidates
        .iter()
        .find(|p| p.is_file())
        .ok_or("Turn on developer mode in Open Stream Deck: Settings, Plugins. Or pass --endpoint <opensd-dev.json>")?;
    let value: Value = serde_json::from_slice(&fs::read(file).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    Ok(Endpoint {
        port: value["port"]
            .as_u64()
            .and_then(|p| u16::try_from(p).ok())
            .ok_or("Invalid developer endpoint")?,
        token: value["token"]
            .as_str()
            .ok_or("Invalid developer endpoint")?
            .to_string(),
    })
}

fn call(ep: &Endpoint, command: &str, payload: Value) -> Result<Value> {
    let mut stream = TcpStream::connect(("127.0.0.1", ep.port))
        .map_err(|e| format!("Open Stream Deck is not reachable: {e}"))?;
    stream.set_read_timeout(Some(Duration::from_secs(30))).ok();
    let body = json!({ "token": ep.token, "command": command, "payload": payload }).to_string();
    let request = format!(
        "POST /dev HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream
        .write_all(request.as_bytes())
        .map_err(|e| e.to_string())?;
    let mut response = Vec::new();
    stream
        .take(1 << 20)
        .read_to_end(&mut response)
        .map_err(|e| e.to_string())?;
    let start = response
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .ok_or("Malformed answer from the app")?
        + 4;
    let value: Value = serde_json::from_slice(&response[start..]).map_err(|e| e.to_string())?;
    if value["ok"] != true {
        return Err(format!(
            "App: {}",
            value["error"].as_str().unwrap_or("unknown error")
        ));
    }
    Ok(value["result"].clone())
}

fn snapshot(root: &Path) -> BTreeMap<PathBuf, SystemTime> {
    fn visit(dir: &Path, out: &mut BTreeMap<PathBuf, SystemTime>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let name = entry.file_name();
            if IGNORED.iter().any(|skip| name == *skip) {
                continue;
            }
            match entry.file_type() {
                Ok(t) if t.is_dir() => visit(&entry.path(), out),
                Ok(t) if t.is_file() => {
                    if let Ok(modified) = entry.metadata().and_then(|m| m.modified()) {
                        out.insert(entry.path(), modified);
                    }
                }
                _ => {}
            }
        }
    }
    let mut out = BTreeMap::new();
    visit(root, &mut out);
    out
}

fn tail(path: &Path, offset: u64) -> u64 {
    let Ok(mut file) = fs::File::open(path) else {
        return offset;
    };
    let len = file.metadata().map_or(0, |m| m.len());
    let from = if len < offset { 0 } else { offset };
    if len == from || file.seek(SeekFrom::Start(from)).is_err() {
        return from;
    }
    let mut data = Vec::new();
    if file.read_to_end(&mut data).is_err() {
        return from;
    }
    print!("{}", String::from_utf8_lossy(&data));
    from + data.len() as u64
}

pub fn run(cwd: &Path, endpoint_file: Option<PathBuf>) -> Result {
    let ep = endpoint(endpoint_file)?;
    let mut project = Project::open(cwd)?;
    let id = project.manifest.id.clone();
    let stage = project.stage(Build::Dev)?;
    let linked = call(&ep, "link", json!({ "id": id, "root": stage }))?;
    project.prune_stages(&stage);
    let mut log = linked["log"].as_str().map(PathBuf::from);
    let mut offset = log
        .as_ref()
        .and_then(|p| fs::metadata(p).ok())
        .map_or(0, |m| m.len());
    println!(
        "{id} runs in Open Stream Deck. Watching {} for changes; Ctrl+C ends the session.",
        project.root.display()
    );
    let mut before = snapshot(&project.root);
    let mut beat = Instant::now();
    loop {
        std::thread::sleep(Duration::from_millis(300));
        if beat.elapsed() >= Duration::from_secs(3) {
            beat = Instant::now();
            if let Err(e) = call(&ep, "heartbeat", json!({ "id": id })) {
                eprintln!("{e}; linking again");
                match project
                    .stage(Build::Dev)
                    .and_then(|stage| call(&ep, "link", json!({ "id": id, "root": stage })))
                {
                    Ok(value) => {
                        log = value["log"].as_str().map(PathBuf::from);
                        if let Some(stage) = value["root"].as_str() {
                            project.prune_stages(Path::new(stage));
                        }
                    }
                    Err(e) => eprintln!("{e}"),
                }
            }
        }
        if let Some(path) = &log {
            offset = tail(path, offset);
        }
        let now = snapshot(&project.root);
        if now == before {
            continue;
        }
        std::thread::sleep(Duration::from_millis(150));
        match Project::open(cwd).and_then(|next| {
            let stage = next.stage(Build::Dev)?;
            call(
                &ep,
                "link",
                json!({ "id": next.manifest.id, "root": stage }),
            )?;
            next.prune_stages(&stage);
            Ok(next)
        }) {
            Ok(next) => {
                if next.manifest.id != id {
                    return Err("The plugin id changed; restart opensd dev".into());
                }
                project = next;
                println!("Reloaded");
            }
            Err(e) => eprintln!("{e}\nThe previous build keeps running."),
        }
        before = snapshot(&project.root);
    }
}
