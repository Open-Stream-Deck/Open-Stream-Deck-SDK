# External components

External components are files installed outside a plugin's sandbox by Open Stream Deck. The developer declares the desired layout; the app owns discovery, review, consent, backups and recovery. Components do not grant filesystem access or process execution to JS/WASM plugins.

## Package compatibility

Ordinary plugins remain SDK 1, archive format 1. Packages declaring external components use SDK 1, archive format 2. Updated readers accept both formats. Older readers reject format 2 before accepting its manifest. Do not distribute a format-2 package to an older app as a compatible update.

External components require SDK/CLI 1.3.0 and Open Stream Deck 1.1.0 or newer. `min_app` is mandatory: use `"1.1.0"` or the newer app version actually required by the plugin. Keep catalog compatibility metadata consistent with the verified manifest.

## Manifest

See [the complete example](../../examples/external-components/opensd.toml).

`targets` describe places where a component can be installed:

| Field | Meaning |
| --- | --- |
| `id` | Stable lowercase identifier containing letters, digits and hyphens |
| `name` | Name shown to users |
| `selection` | `directory` or `executable`; an executable selection uses its parent directory |
| `os` | Optional `windows`, `linux` or `macos`; omitted means any supported OS |
| `required_files` | Files expected relative to the selected root; all must exist |
| `discovery` | Optional exact discovery rules |
| `location` | Optional known base directory plus relative path |
| `after_install` | Instructions shown after successful copying |

An executable picker does not infer arbitrary parent directories. Declare validation paths relative to the executable's parent, or use a directory picker for an application installation root.

Steam discovery uses `[[targets.discovery]]`, `type = "steam"`, `app_id = "227300"`. The app reads the registered Steam location, library descriptions and the exact app manifest. It does not scan disks. Users can always choose a different directory.

For a Windows user extension directory:

```toml
[targets.location]
base = "app_data"
path = "Vendor/ExampleApp/plugins"
```

The automatic candidate is `%APPDATA%/Vendor/ExampleApp/plugins`. Use `local_app_data` for Local AppData. These identifiers are SDK tokens, not environment variable expansion. When the extension directory does not exist and there are no required files, installation can create it after consent. In that case discovery does not prove that the parent application is installed.

On Linux, `config` uses an absolute `$XDG_CONFIG_HOME` or `~/.config`, and `data` uses an absolute `$XDG_DATA_HOME` or `~/.local/share`. On macOS, `application_support` uses `~/Library/Application Support`. Unsupported bases are not mapped to other platforms. Steam discovery also reads native macOS and Linux library metadata, including the usual Flatpak Steam data directory.

Windows, Linux and macOS support `x86_64` and `aarch64` variants. Installation and ZIP export select the exact OS and architecture of the running app, without fallback or emulation. The UI lists only compatible components for setup and explains which components have no compatible build. Registry discovery and elevated installation helpers remain outside this release.

`components` contain `id`, `name`, `description`, an independent SemVer `version`, `targets` referencing target ids, and `variants`. Each variant declares `os`, `arch`, and `files` containing `source` and `destination` pairs.

Each file can set `executable = true`. Unix installation and ZIP export then use mode 0755; ordinary new files use 0644. Existing Unix permissions are preserved for ordinary replacements, recorded with originals and restored on removal or recovery. Permission changes after review invalidate the plan. Special Unix permission bits are rejected. Nothing is executed by the installer.

Source paths are relative to the plugin project. They are automatically included by `opensd pack`, deduplicated with resources, and covered by the package index. Native file signatures are allowed only for declared component sources. Destination paths are relative to the user-approved root. Both use forward slashes and the existing portable safe-path rules. A variant cannot declare case-colliding or parent/child file destinations. Different components cannot occupy overlapping installed destinations.

One component can reference several targets when they use the same relative destination layout. When layouts differ, declare separate components referencing the same source files. No installation scripts or path expressions are evaluated.

## Lifecycle

Installing or updating a plugin never copies external components automatically. The user reviews and confirms a concrete plan. A plan expires if source files, manifest, destination contents or installation records change. Only one pending plan is retained by the app.

Matching files not created by the installer remain unowned and are not removed later. Replaced originals survive subsequent updates. Disappearing owned files are removed or restored during an update. Conflicting changes require explicit consent, and their current contents are also preserved under the app's `opensd/components/backups` directory.

Installation records and original bytes are in `opensd/components/installations.json` inside the app data directory. `journal.json` records an incomplete operation. The UI offers recovery after restart. Do not delete these files to clear an error: they may be the only remaining originals. The UI receives summaries without original file contents.

Removing a JS/WASM plugin retains its external components and their records. Use the global External components button to manage components left behind. Revoking runtime permissions also does not remove a DLL or unload it from a running application.

Installation is a recoverable sequence, not a filesystem-wide atomic transaction. Junctions, reparse points and symlinks in target paths are rejected, with checks repeated before operations. This is not a guarantee against an adversarial process racing filesystem operations. No elevation is used; protected directories require manual installation. All `required_files` markers are checked against running executable paths on every platform before changes; include the application binary, including `Contents/MacOS/...` when using a macOS bundle root. Unknown process layouts still require the user to close the target application. The installer does not sign binaries, bypass OS security policies or convert Windows DLLs into native libraries.

Export creates a new ZIP with the destination structure and `OPENSD-INSTALL.txt`. Existing export files are not overwritten. It does not create an ownership record for manually copied files. Avoid a destination named `OPENSD-INSTALL.txt`, which is reserved for export instructions.

## Runtime status

The installer reports file status: installed, modified/missing, directory unavailable or update available. It does not equate installed files with a live connection. Communication, bridge identity and protocol compatibility remain checked by the plugin through existing permission-controlled APIs. No new arbitrary backend HTTP probes or JS filesystem commands are introduced.

## Store metadata

The app catalog reader accepts optional `components` entries with `name`, `version`, and `platforms`, and shows them before installation. The catalog publisher must derive these summaries from verified package manifests. The local store repository is a UI prototype; this workspace does not contain the production catalog publisher. Updating the app does not update a deployed catalog.
