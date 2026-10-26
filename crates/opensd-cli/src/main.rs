mod dev;
mod inspect;
mod new;
mod project;

use std::path::PathBuf;
use std::process::ExitCode;

pub type Result<T = ()> = std::result::Result<T, String>;

const USAGE: &str = "\
Usage: opensd <command> [options]

Commands:
  new <dir> [--lang rust|js|ts] [--id <id>] [--name <name>] [--sdk-path <dir>]
                       Create a plugin project
  build [--debug]      Build the code and stage the plugin in .opensd
  pack [--out <file>]  Build and write dist/<id>-<version>.opensd
  check [<path>]       Validate the project, or a .opensd package
  inspect <file>       Show what a .opensd package contains and asks for
  dev [--endpoint <file>]
                       Run the plugin in the app and reload it on every change
  clean                Remove .opensd and dist

Rust plugins share one build cache in the user cache folder.
Set OPENSD_SHARED_TARGET=0 to build into the project's own target folder.";

const VALUED: [&str; 6] = [
    "--lang",
    "--id",
    "--name",
    "--sdk-path",
    "--out",
    "--endpoint",
];

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn option(args: &[String], name: &str) -> Option<String> {
    let flag = format!("--{name}");
    args.iter().enumerate().find_map(|(i, arg)| {
        if *arg == flag {
            args.get(i + 1).cloned()
        } else {
            arg.strip_prefix(&format!("{flag}=")).map(str::to_string)
        }
    })
}

fn flag(args: &[String], name: &str) -> bool {
    args.iter().any(|a| *a == format!("--{name}"))
}

fn positional(args: &[String]) -> Option<String> {
    let mut skip = false;
    for arg in args {
        if std::mem::take(&mut skip) {
            continue;
        }
        if arg.starts_with("--") {
            skip = VALUED.contains(&arg.as_str());
            continue;
        }
        return Some(arg.clone());
    }
    None
}

fn help() {
    println!(
        "opensd {}, Open Stream Deck plugin tool\n\n{USAGE}",
        env!("CARGO_PKG_VERSION")
    );
}

fn run(args: &[String]) -> Result {
    let Some(command) = args.first() else {
        help();
        return Ok(());
    };
    let rest = &args[1..];
    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
    match command.as_str() {
        "new" => {
            let dir = positional(rest).ok_or("Usage: opensd new <dir> [--lang rust|js|ts]")?;
            new::create(
                &cwd.join(dir),
                new::Options {
                    lang: option(rest, "lang").unwrap_or_else(|| "rust".into()),
                    id: option(rest, "id"),
                    name: option(rest, "name"),
                    sdk_path: option(rest, "sdk-path").map(|p| cwd.join(p)),
                },
            )
        }
        "build" => {
            let project = project::Project::open(&cwd)?;
            let build = if flag(rest, "debug") {
                project::Build::Debug
            } else {
                project::Build::Release
            };
            let stage = project.stage(build)?;
            project.prune_stages(&stage);
            println!(
                "Staged {} {} in {}",
                project.manifest.id,
                project.manifest.version,
                stage.display()
            );
            Ok(())
        }
        "pack" => project::Project::open(&cwd)?.pack(option(rest, "out").map(PathBuf::from)),
        "check" => match positional(rest) {
            Some(path) if path.ends_with(".opensd") => {
                let package = opensd_package::read(&cwd.join(&path)).map_err(|e| e.to_string())?;
                println!(
                    "{path}: valid package {} {}",
                    package.manifest.id, package.manifest.version
                );
                Ok(())
            }
            path => {
                let project = project::Project::open(&cwd.join(path.unwrap_or_default()))?;
                project.check()?;
                println!(
                    "{}: valid project {} {}",
                    project.root.display(),
                    project.manifest.id,
                    project.manifest.version
                );
                Ok(())
            }
        },
        "inspect" => {
            inspect::run(&cwd.join(positional(rest).ok_or("Usage: opensd inspect <file.opensd>")?))
        }
        "dev" => dev::run(&cwd, option(rest, "endpoint").map(PathBuf::from)),
        "clean" => {
            for dir in [".opensd", "dist"] {
                let path = cwd.join(dir);
                if path.exists() {
                    std::fs::remove_dir_all(&path)
                        .map_err(|e| format!("{}: {e}", path.display()))?;
                    println!("Removed {}", path.display());
                }
            }
            Ok(())
        }
        "help" | "--help" | "-h" => {
            help();
            Ok(())
        }
        "version" | "--version" | "-V" => {
            println!("opensd {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        other => Err(format!("Unknown command {other}. Run opensd help")),
    }
}
