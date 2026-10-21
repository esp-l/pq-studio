use std::path::{Path, PathBuf};

/// Reverse-DNS id shared by the Linux `.desktop` file, the Wayland/X11 app id
/// and the macOS bundle identifier.
pub const APP_ID: &str = "no.laerk.PQStudio";

/// A name of its own rather than "PHREEQC Interactive"/"PhreeqcI", so it is
/// never mistaken for the USGS product. PHREEQC is only named descriptively.
pub const APP_NAME: &str = "PQ Studio";

const ENGINE_NAME: &str = if cfg!(windows) { "phreeqc.exe" } else { "phreeqc" };

/// Files for one engine run, all inside the work dir.
pub struct RunFiles {
    pub input: PathBuf,
    pub output: PathBuf,
    pub stderr: PathBuf,
}

/// Write the engine input for `input_body` into `work_dir`.
///
/// Always prepends our own `DATABASE <path>` line and never trusts a
/// DATABASE line embedded in caller-supplied text: a stale/Windows-style
/// absolute path in that keyword was observed to make the real engine hang
/// and balloon to multiple GB of RAM rather than error cleanly.
pub fn prepare_run(input_body: &str, database_path: &Path, work_dir: &Path) -> std::io::Result<RunFiles> {
    std::fs::create_dir_all(work_dir)?;
    let files = RunFiles {
        input: work_dir.join("scratch.pqi"),
        output: work_dir.join("scratch.pqo"),
        stderr: work_dir.join("scratch.err"),
    };
    let _ = std::fs::remove_file(&files.output);

    let mut full_input = String::new();
    full_input.push_str(&format!("DATABASE {}\n", database_path.display()));
    for line in input_body.lines() {
        if line.trim_start().to_uppercase().starts_with("DATABASE") {
            continue;
        }
        full_input.push_str(line);
        full_input.push('\n');
    }
    std::fs::write(&files.input, &full_input)?;
    Ok(files)
}

/// The engine writes its ASCII-art banner to stderr along with any errors;
/// keep only the lines that explain the failure.
pub fn summarize_errors(stderr: &str) -> String {
    let relevant: Vec<&str> = stderr
        .lines()
        .map(str::trim)
        .filter(|l| {
            let lower = l.to_lowercase();
            l.contains("ERROR") || lower.contains("terminating") || lower.starts_with("stopping")
        })
        .collect();
    if !relevant.is_empty() {
        return relevant.join("\n");
    }
    let tail: Vec<&str> = stderr.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    let start = tail.len().saturating_sub(5);
    tail[start..].join("\n")
}

/// Locate the `phreeqc` engine. Order: `PHREEQC_BIN`, a copy bundled next to
/// this executable (the macOS `.app` and Windows zip layouts), `PATH`, then
/// well-known install prefixes — the last step matters because macOS GUI apps
/// get a minimal `PATH` that excludes Homebrew and `/usr/local`. On Windows
/// the prefixes are the official USGS installs, `C:\Program Files\USGS\phreeqc-*`.
pub fn find_engine() -> Option<PathBuf> {
    let env_override = std::env::var_os("PHREEQC_BIN").map(PathBuf::from);
    let mut path_dirs: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).collect())
        .unwrap_or_default();
    path_dirs.extend(usgs_windows_installs().into_iter().map(|d| d.join("bin").join("Release")));
    find_engine_in(env_override, exe_dir(), &path_dirs, home_dir())
}

/// Official USGS PHREEQC installs on Windows (`%ProgramFiles%\USGS\phreeqc-*`),
/// newest name first. Empty elsewhere.
fn usgs_windows_installs() -> Vec<PathBuf> {
    if !cfg!(windows) {
        return Vec::new();
    }
    let mut found: Vec<PathBuf> = ["ProgramFiles", "ProgramFiles(x86)"]
        .iter()
        .filter_map(std::env::var_os)
        .filter_map(|root| std::fs::read_dir(PathBuf::from(root).join("USGS")).ok())
        .flat_map(|entries| entries.filter_map(|e| e.ok().map(|e| e.path())))
        .filter(|p| {
            p.file_name()
                .is_some_and(|n| n.to_string_lossy().to_lowercase().starts_with("phreeqc-"))
        })
        .collect();
    found.sort();
    found.reverse();
    found
}

fn find_engine_in(
    env_override: Option<PathBuf>,
    exe_dir: Option<PathBuf>,
    path_dirs: &[PathBuf],
    home: Option<PathBuf>,
) -> Option<PathBuf> {
    if let Some(p) = env_override.filter(|p| p.is_file()) {
        return Some(p);
    }
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(dir) = &exe_dir {
        candidates.push(dir.join(ENGINE_NAME));
        candidates.push(dir.join("../Resources/bin").join(ENGINE_NAME));
    }
    candidates.extend(path_dirs.iter().map(|d| d.join(ENGINE_NAME)));
    if cfg!(unix) {
        for prefix in ["/usr", "/usr/local", "/opt/homebrew", "/opt/local"] {
            candidates.push(Path::new(prefix).join("bin").join(ENGINE_NAME));
        }
        if let Some(home) = &home {
            candidates.push(home.join(".local/bin").join(ENGINE_NAME));
        }
    }
    candidates.into_iter().find(|p| p.is_file())
}

/// Locate the default `phreeqc.dat`. Order: `PHREEQC_DATABASE`, a copy
/// bundled with this app, then relative to the engine's own install prefix
/// (`share/phreeqc/database` as Arch packages it, `share/doc/phreeqc/database`
/// as upstream `make install` lays it out, `database` beside `bin\Release` as
/// the USGS Windows installer does), then well-known prefixes.
pub fn find_database(engine: Option<&Path>) -> Option<PathBuf> {
    let env_override = std::env::var_os("PHREEQC_DATABASE").map(PathBuf::from);
    find_database_in(env_override, exe_dir(), engine, home_dir())
}

fn find_database_in(
    env_override: Option<PathBuf>,
    exe_dir: Option<PathBuf>,
    engine: Option<&Path>,
    home: Option<PathBuf>,
) -> Option<PathBuf> {
    if let Some(p) = env_override.filter(|p| p.is_file()) {
        return Some(p);
    }
    const LAYOUTS: [&str; 2] = ["share/phreeqc/database", "share/doc/phreeqc/database"];
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(dir) = &exe_dir {
        candidates.push(dir.join("../Resources/database/phreeqc.dat"));
        candidates.push(dir.join("database/phreeqc.dat"));
    }
    if let Some(prefix) = engine
        .and_then(real_path)
        .and_then(|e| e.parent()?.parent().map(Path::to_path_buf))
    {
        for layout in LAYOUTS {
            candidates.push(prefix.join(layout).join("phreeqc.dat"));
        }
        candidates.push(prefix.join("database/phreeqc.dat"));
        if let Some(above) = prefix.parent() {
            candidates.push(above.join("database/phreeqc.dat"));
        }
    }
    if !cfg!(unix) {
        return candidates.into_iter().find(|p| p.is_file());
    }
    let mut prefixes: Vec<PathBuf> = ["/usr", "/usr/local", "/opt/homebrew", "/opt/local"]
        .iter()
        .map(PathBuf::from)
        .collect();
    if let Some(home) = &home {
        prefixes.push(home.join(".local"));
    }
    for prefix in &prefixes {
        for layout in LAYOUTS {
            candidates.push(prefix.join(layout).join("phreeqc.dat"));
        }
    }
    candidates.into_iter().find(|p| p.is_file())
}

/// `path` with symlinks resolved. On Windows, `canonicalize` gives a
/// `\\?\C:\...` path, which turns off `/` handling and which the engine
/// may not open; drop that prefix (but keep `\\?\UNC\` network paths).
fn real_path(path: impl AsRef<Path>) -> Option<PathBuf> {
    let real = std::fs::canonicalize(path).ok()?;
    if cfg!(windows) {
        let text = real.to_string_lossy();
        if let Some(rest) = text.strip_prefix(r"\\?\").filter(|r| !r.starts_with(r"UNC\")) {
            return Some(PathBuf::from(rest));
        }
    }
    Some(real)
}

fn exe_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(real_path)
        .and_then(|e| e.parent().map(Path::to_path_buf))
}

fn home_dir() -> Option<PathBuf> {
    crate::settings::home_dir()
}

/// Per-user scratch directory for engine runs (`~/.cache/pqstudio/workdir`,
/// `~/Library/Caches/no.laerk.PQ-Studio/workdir`).
pub fn default_work_dir() -> PathBuf {
    crate::settings::cache_dir()
        .unwrap_or_else(|| std::env::temp_dir().join("pq-studio"))
        .join("workdir")
}

/// The `.dat` database files in `dir`, sorted by name (case-insensitive).
pub fn list_databases(dir: &Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| {
                    p.is_file()
                        && p.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("dat"))
                })
                .collect()
        })
        .unwrap_or_default();
    found.sort_by_key(|p| p.file_name().map(|n| n.to_string_lossy().to_lowercase()));
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn touch(path: &Path) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, "").unwrap();
    }

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("pq-studio-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn engine_found_on_path_then_database_beside_its_prefix() {
        let root = scratch("upstream");
        let engine = root.join("prefix/bin").join(ENGINE_NAME);
        touch(&engine);
        touch(&root.join("prefix/share/doc/phreeqc/database/phreeqc.dat"));

        let found = find_engine_in(None, None, &[root.join("prefix/bin")], None).unwrap();
        assert_eq!(found, engine);
        let db = find_database_in(None, None, Some(&found), None).unwrap();
        assert!(db.ends_with("share/doc/phreeqc/database/phreeqc.dat"));
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn bundled_macos_layout_wins_over_path() {
        let root = scratch("bundle");
        let macos_dir = root.join("App.app/Contents/MacOS");
        touch(&macos_dir.join(ENGINE_NAME));
        touch(&root.join("App.app/Contents/Resources/database/phreeqc.dat"));
        touch(&root.join("elsewhere/bin").join(ENGINE_NAME));

        let found =
            find_engine_in(None, Some(macos_dir.clone()), &[root.join("elsewhere/bin")], None).unwrap();
        assert_eq!(found, macos_dir.join(ENGINE_NAME));
        let db = find_database_in(None, Some(macos_dir), Some(&found), None).unwrap();
        assert!(db.ends_with("Contents/MacOS/../Resources/database/phreeqc.dat"));
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn usgs_windows_layout_finds_database_beside_bin() {
        let root = scratch("usgs-win");
        let release = root.join("phreeqc-3.8.6-17100-x64/bin/Release");
        touch(&release.join(ENGINE_NAME));
        touch(&root.join("phreeqc-3.8.6-17100-x64/database/phreeqc.dat"));

        let found = find_engine_in(None, None, &[release.clone()], None).unwrap();
        let db = find_database_in(None, None, Some(&found), None).unwrap();
        assert!(db.ends_with("phreeqc-3.8.6-17100-x64/database/phreeqc.dat"));
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn error_summary_drops_engine_banner() {
        let stderr = "Input file: in.pqi\n   |   * PHREEQC_3.8.9 *   |\nInitializing...\n\
                      ERROR: Must define at least one solution number and mixing fraction for MIX input.\n\
                      Stopping.\nERROR: Calculations terminating due to input errors.\n";
        assert_eq!(
            summarize_errors(stderr),
            "ERROR: Must define at least one solution number and mixing fraction for MIX input.\n\
             Stopping.\nERROR: Calculations terminating due to input errors."
        );
        assert_eq!(summarize_errors("a\nb\n\nc"), "a\nb\nc");
    }

    #[test]
    fn lists_only_dat_files_sorted() {
        let root = scratch("dblist");
        for name in ["wateq4f.dat", "Amm.dat", "phreeqc.dat", "README.txt"] {
            touch(&root.join(name));
        }
        let names: Vec<String> = list_databases(&root)
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, ["Amm.dat", "phreeqc.dat", "wateq4f.dat"]);
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn env_override_ignored_when_file_missing() {
        let root = scratch("override");
        let engine = root.join("bin").join(ENGINE_NAME);
        touch(&engine);
        let found = find_engine_in(
            Some(root.join("does-not-exist")),
            None,
            &[root.join("bin")],
            None,
        );
        assert_eq!(found, Some(engine));
        std::fs::remove_dir_all(&root).unwrap();
    }
}
