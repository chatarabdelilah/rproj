//! Read-only, shallow discovery shared by the project browsers.
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

/// Private matching-worker query. Configuration resolution stays in the CLI.
#[doc(hidden)]
pub const PROJECTS_CONTEXT_VERSION: u32 = 1;
#[doc(hidden)]
#[derive(Serialize, Deserialize)]
pub struct ProjectsContext {
    pub protocol: u32,
    pub root: PathBuf,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Source {
    ProjectsFolder,
    LaunchFolder,
    Both,
}

#[derive(Clone, Debug)]
pub struct ProjectEntry {
    pub path: PathBuf,
    pub name: String,
    pub source: Source,
}

#[derive(Default)]
pub struct Discovery {
    pub entries: Vec<ProjectEntry>,
    pub warnings: Vec<String>,
}

fn recognized(path: &Path) -> bool {
    path.join("rproj.toml").is_file() || path.join("default.project.json").is_file()
}

fn linked(metadata: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        metadata.file_type().is_symlink()
    }
}

pub fn discover(
    root: &std::result::Result<PathBuf, String>,
    launch_dir: Option<&Path>,
) -> Discovery {
    let mut result = Discovery::default();
    let mut paths = BTreeMap::new();
    match root {
        Ok(root) => match fs::read_dir(root) {
            Ok(entries) => {
                let mut skipped_links = 0;
                for entry in entries {
                    let entry = match entry {
                        Ok(entry) => entry,
                        Err(error) => {
                            result
                                .warnings
                                .push(format!("Could not read a project entry: {error}"));
                            continue;
                        }
                    };
                    match fs::symlink_metadata(entry.path()) {
                        Ok(metadata) if linked(&metadata) => skipped_links += 1,
                        Ok(metadata) if metadata.is_dir() && recognized(&entry.path()) => {
                            add_path(
                                &entry.path(),
                                Source::ProjectsFolder,
                                &mut paths,
                                &mut result.warnings,
                            );
                        }
                        Ok(_) => {}
                        Err(error) => result
                            .warnings
                            .push(format!("{}: {error}", entry.path().display())),
                    }
                }
                if skipped_links > 0 {
                    result.warnings.push(format!("Skipped {skipped_links} linked entries; only direct project directories are listed."));
                }
            }
            Err(error) => result.warnings.push(format!(
                "Could not read projects folder {}: {error}",
                root.display()
            )),
        },
        Err(error) => result.warnings.push(error.clone()),
    }
    if let Some(path) = launch_dir.filter(|path| recognized(path)) {
        add_path(path, Source::LaunchFolder, &mut paths, &mut result.warnings);
    }
    result.entries = paths
        .into_iter()
        .map(|(path, source)| {
            let name = path
                .file_name()
                .unwrap_or(path.as_os_str())
                .to_string_lossy()
                .into_owned();
            ProjectEntry { path, name, source }
        })
        .collect();
    result.entries.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then_with(|| a.path.cmp(&b.path))
    });
    result
}

fn add_path(
    path: &Path,
    source: Source,
    paths: &mut BTreeMap<PathBuf, Source>,
    warnings: &mut Vec<String>,
) {
    match fs::canonicalize(path) {
        Ok(path) => {
            paths
                .entry(path)
                .and_modify(|existing| {
                    if *existing != source {
                        *existing = Source::Both;
                    }
                })
                .or_insert(source);
        }
        Err(error) => warnings.push(format!("{}: {error}", path.display())),
    }
}
