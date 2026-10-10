use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, ensure};
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};

/// Read-only project facts. Versioned independently from project file schemas.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSnapshot {
    pub schema_version: u32,
    pub root: PathBuf,
    pub name: Option<String>,
    pub kind: ProjectKind,
    pub files: ProjectFiles,
    pub saved_packages: Vec<String>,
    pub packages: Vec<PackageDependency>,
    pub tools: Vec<ToolPin>,
    pub actions: ProjectActions,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectKind {
    Rproj,
    Rojo,
    Unrecognized,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectFiles {
    pub rproj: FileSnapshot,
    pub rojo: FileSnapshot,
    pub wally: FileSnapshot,
    pub wally_lock: FileSnapshot,
    pub rokit: FileSnapshot,
    pub check_script: FileSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSnapshot {
    pub path: PathBuf,
    pub state: FileState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileState {
    Missing,
    Ready,
    Invalid,
    Unreadable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    pub severity: DiagnosticSeverity,
    pub code: String,
    pub path: PathBuf,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageDependency {
    pub alias: String,
    pub realm: Realm,
    /// The original declaration, including the package name and requirement.
    pub declaration: String,
    pub package: Option<String>,
    pub version_requirement: Option<String>,
    pub resolution: ResolutionState,
    pub resolved: Option<ResolvedPackage>,
    pub presence: PackagePresence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Realm {
    Shared,
    Server,
    Dev,
}

impl Realm {
    pub fn directory(self) -> &'static str {
        match self {
            Self::Shared => "Packages",
            Self::Server => "ServerPackages",
            Self::Dev => "DevPackages",
        }
    }

    pub fn section(self) -> &'static str {
        match self {
            Self::Shared => "dependencies",
            Self::Server => "server-dependencies",
            Self::Dev => "dev-dependencies",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolutionState {
    Resolved,
    Unlocked,
    Stale,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedPackage {
    pub package: String,
    pub version: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackagePresence {
    pub state: PresenceState,
    pub path: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PresenceState {
    /// An alias module and package index exist, not proof of a working install.
    Present,
    Missing,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolPin {
    pub alias: String,
    pub source: String,
    pub version_requirement: Option<String>,
    /// PATH discovery only: inspection never executes a tool or a shim.
    pub executable: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectActions {
    pub watch: ActionAvailability,
    pub check: ActionAvailability,
    pub test: ActionAvailability,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionAvailability {
    /// Configuration allows this action; its runner still checks prerequisites.
    pub available: bool,
    pub reason: Option<String>,
}

impl ActionAvailability {
    fn yes() -> Self {
        Self {
            available: true,
            reason: None,
        }
    }

    fn no(reason: impl Into<String>) -> Self {
        Self {
            available: false,
            reason: Some(reason.into()),
        }
    }
}

#[derive(Debug, Default, Deserialize)]
struct SavedProject {
    #[serde(default)]
    mode: String,
    #[serde(default)]
    package_workflow: PackageWorkflow,
    #[serde(default)]
    packages: Vec<String>,
    #[serde(default)]
    dropped: Vec<String>,
    #[serde(default)]
    capabilities: BTreeMap<String, String>,
}

#[derive(Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum PackageWorkflow {
    #[default]
    Wally,
    None,
}

/// Inspect an existing directory without executing tools, adopting it, or writing files.
/// Invalid project files become diagnostics; an inaccessible root is an error.
pub fn inspect(path: &Path) -> Result<ProjectSnapshot> {
    let root = path
        .canonicalize()
        .with_context(|| format!("cannot inspect {}", path.display()))?;
    ensure!(
        root.is_dir(),
        "{} is not a project directory",
        root.display()
    );
    fs::read_dir(&root)
        .with_context(|| format!("cannot read project directory {}", root.display()))?;
    let mut diagnostics = Vec::new();
    let (mut rproj_file, rproj) = read_toml(&root, "rproj.toml", &mut diagnostics);
    let (mut rojo_file, rojo) = read_json(&root, "default.project.json", &mut diagnostics);
    let (mut wally_file, wally) = read_toml(&root, "wally.toml", &mut diagnostics);
    let (mut lock_file, lock) = read_toml(&root, "wally.lock", &mut diagnostics);
    let (mut rokit_file, rokit) = read_toml(&root, "rokit.toml", &mut diagnostics);
    let (check_file, _) = read_text(&root, ".lute/check.luau", &mut diagnostics);

    let saved = rproj.and_then(|value| match value.try_into::<SavedProject>() {
        Ok(saved) => Some(saved),
        Err(_) => {
            invalid(
                &mut rproj_file,
                &mut diagnostics,
                "invalid_rproj",
                "Invalid rproj field types or unsupported package workflow; inspect rproj.toml"
                    .into(),
            );
            None
        }
    });
    let name = rojo
        .as_ref()
        .and_then(|value| value.get("name"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned);
    if let Some(rojo) = &rojo {
        let valid = rojo.as_object().is_some()
            && rojo.get("tree").is_some_and(serde_json::Value::is_object)
            && rojo.get("name").is_none_or(serde_json::Value::is_string);
        if !valid {
            invalid(
                &mut rojo_file,
                &mut diagnostics,
                "invalid_rojo",
                "Expected a Rojo project object with a tree object and an optional string name"
                    .into(),
            );
        }
    }
    let locked = lock
        .as_ref()
        .and_then(|value| parse_lock(value, wally.as_ref(), &mut lock_file, &mut diagnostics));
    let packages = wally
        .as_ref()
        .map(|value| {
            parse_dependencies(
                value,
                &root,
                &mut wally_file,
                &lock_file,
                locked.as_ref(),
                &mut diagnostics,
            )
        })
        .unwrap_or_default();
    let tools = rokit
        .as_ref()
        .map(|value| parse_tools(value, &mut rokit_file, &mut diagnostics))
        .unwrap_or_default();
    let files = ProjectFiles {
        rproj: rproj_file,
        rojo: rojo_file,
        wally: wally_file,
        wally_lock: lock_file,
        rokit: rokit_file,
        check_script: check_file,
    };
    let kind = if saved.is_some() {
        ProjectKind::Rproj
    } else if files.rojo.state == FileState::Ready {
        ProjectKind::Rojo
    } else {
        ProjectKind::Unrecognized
    };
    let actions = actions(&files, saved.as_ref());
    let saved_packages = saved
        .map(|saved| {
            // Reading these fields validates their shape while preserving the full
            // decision graph's authority over creation and configuration operations.
            let _ = (saved.mode, saved.dropped);
            saved.packages
        })
        .unwrap_or_default();
    Ok(ProjectSnapshot {
        schema_version: 1,
        root,
        name,
        kind,
        files,
        saved_packages,
        packages,
        tools,
        actions,
        diagnostics,
    })
}

fn read_text(
    root: &Path,
    relative: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> (FileSnapshot, Option<String>) {
    let mut file = FileSnapshot {
        path: root.join(relative),
        state: FileState::Ready,
    };
    match read_configuration(&file.path) {
        Ok(text) => (file, Some(text)),
        Err(error) => {
            file.state = if error.kind() == io::ErrorKind::NotFound {
                FileState::Missing
            } else {
                FileState::Unreadable
            };
            diagnostics.push(Diagnostic {
                severity: if file.state == FileState::Missing {
                    DiagnosticSeverity::Info
                } else {
                    DiagnosticSeverity::Error
                },
                code: if file.state == FileState::Missing {
                    "file_missing"
                } else {
                    "file_unreadable"
                }
                .into(),
                path: file.path.clone(),
                message: if file.state == FileState::Missing {
                    format!("{relative} is absent")
                } else {
                    format!("Cannot read {relative}: {error}")
                },
            });
            (file, None)
        }
    }
}

fn read_configuration(path: &Path) -> io::Result<String> {
    const MAX_BYTES: u64 = 16 * 1024 * 1024;
    let file = fs::File::open(path)?;
    if file.metadata()?.len() > MAX_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "configuration exceeds 16 MiB limit",
        ));
    }
    let mut source = String::new();
    file.take(MAX_BYTES + 1).read_to_string(&mut source)?;
    if source.len() as u64 > MAX_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "configuration exceeds 16 MiB limit",
        ));
    }
    Ok(source)
}

fn read_toml(
    root: &Path,
    relative: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> (FileSnapshot, Option<toml::Value>) {
    let (mut file, source) = read_text(root, relative, diagnostics);
    let value = source.and_then(|source| match toml::from_str(&source) {
        Ok(value) => Some(value),
        Err(error) => {
            let location = error
                .span()
                .map(|span| {
                    let prefix = &source.as_bytes()[..span.start.min(source.len())];
                    let line = prefix.iter().filter(|byte| **byte == b'\n').count() + 1;
                    format!(" near line {line}")
                })
                .unwrap_or_default();
            invalid(
                &mut file,
                diagnostics,
                "invalid_toml",
                format!("Invalid TOML{location}; inspect the file to correct its syntax"),
            );
            None
        }
    });
    (file, value)
}

fn read_json(
    root: &Path,
    relative: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> (FileSnapshot, Option<serde_json::Value>) {
    let (mut file, source) = read_text(root, relative, diagnostics);
    let value = source.and_then(|source| match serde_json::from_str(&source) {
        Ok(value) => Some(value),
        Err(error) => {
            invalid(
                &mut file,
                diagnostics,
                "invalid_json",
                format!(
                    "Invalid JSON at line {}, column {}; inspect the file to correct its syntax",
                    error.line(),
                    error.column()
                ),
            );
            None
        }
    });
    (file, value)
}

fn invalid(
    file: &mut FileSnapshot,
    diagnostics: &mut Vec<Diagnostic>,
    code: &str,
    message: String,
) {
    file.state = FileState::Invalid;
    diagnostics.push(Diagnostic {
        severity: DiagnosticSeverity::Error,
        code: code.into(),
        path: file.path.clone(),
        message,
    });
}

fn warning(file: &FileSnapshot, diagnostics: &mut Vec<Diagnostic>, code: &str, message: String) {
    diagnostics.push(Diagnostic {
        severity: DiagnosticSeverity::Warning,
        code: code.into(),
        path: file.path.clone(),
        message,
    });
}

fn parse_lock(
    value: &toml::Value,
    manifest: Option<&toml::Value>,
    file: &mut FileSnapshot,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<BTreeMap<String, Vec<ResolvedPackage>>> {
    let Some(entries) = value.get("package").and_then(toml::Value::as_array) else {
        invalid(
            file,
            diagnostics,
            "invalid_lock",
            "Expected [[package]] entries in wally.lock".into(),
        );
        return None;
    };
    for entry in entries {
        let valid_identity = entry
            .get("name")
            .and_then(toml::Value::as_str)
            .zip(entry.get("version").and_then(toml::Value::as_str))
            .is_some_and(|(name, version)| {
                split_source(&format!("{name}@{version}")).is_some()
                    && Version::parse(version).is_ok()
            });
        let valid_dependencies = entry.get("dependencies").is_none_or(|value| {
            value.as_array().is_some_and(|dependencies| {
                dependencies.iter().all(|dependency| {
                    dependency
                        .as_array()
                        .filter(|pair| pair.len() == 2)
                        .is_some_and(|pair| {
                            pair[0].as_str().is_some()
                                && pair[1]
                                    .as_str()
                                    .and_then(split_source)
                                    .is_some_and(|(_, version)| Version::parse(version).is_ok())
                        })
                })
            })
        });
        if !valid_identity || !valid_dependencies {
            invalid(file, diagnostics, "invalid_lock", "Each locked package needs a valid name, SemVer version and [alias, package@version] dependency pairs".into());
            return None;
        }
    }
    let available: BTreeSet<_> = entries
        .iter()
        .filter_map(|entry| {
            Some((
                entry.get("name")?.as_str()?,
                entry.get("version")?.as_str()?,
            ))
        })
        .collect();
    for entry in entries {
        let dependencies = entry
            .get("dependencies")
            .and_then(toml::Value::as_array)
            .into_iter()
            .flatten();
        for dependency in dependencies {
            let target = dependency
                .as_array()
                .and_then(|pair| pair.get(1))
                .and_then(toml::Value::as_str)
                .and_then(split_source);
            if !target.is_some_and(|target| available.contains(&target)) {
                invalid(
                    file,
                    diagnostics,
                    "missing_locked_package",
                    "wally.lock references a package version without a matching [[package]] entry"
                        .into(),
                );
                return None;
            }
        }
    }
    let package = manifest?.get("package")?;
    let name = package.get("name")?.as_str()?;
    let version = package.get("version")?.as_str()?;
    if entries
        .iter()
        .filter(|entry| {
            entry.get("name").and_then(toml::Value::as_str) == Some(name)
                && entry.get("version").and_then(toml::Value::as_str) == Some(version)
        })
        .count()
        > 1
    {
        invalid(
            file,
            diagnostics,
            "invalid_lock",
            "More than one lockfile entry matches this project's name and version".into(),
        );
        return None;
    }
    let Some(root) = entries.iter().find(|entry| {
        entry.get("name").and_then(toml::Value::as_str) == Some(name)
            && entry.get("version").and_then(toml::Value::as_str) == Some(version)
    }) else {
        warning(
            file,
            diagnostics,
            "stale_lock",
            "No lockfile entry matches this project's package name and version".into(),
        );
        return Some(BTreeMap::new());
    };
    let dependencies = root
        .get("dependencies")
        .and_then(toml::Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default();
    let mut result: BTreeMap<String, Vec<ResolvedPackage>> = BTreeMap::new();
    for dependency in dependencies {
        let pair = dependency.as_array().filter(|pair| pair.len() == 2);
        let parsed = pair.and_then(|pair| Some((pair[0].as_str()?, pair[1].as_str()?)));
        let Some((alias, source)) = parsed else {
            invalid(
                file,
                diagnostics,
                "invalid_lock",
                "Lockfile dependencies must be [alias, package@version] pairs".into(),
            );
            return None;
        };
        let Some((package, version)) =
            split_source(source).filter(|(_, version)| Version::parse(version).is_ok())
        else {
            invalid(
                file,
                diagnostics,
                "invalid_lock",
                format!("Invalid resolved package for {alias}: {source}"),
            );
            return None;
        };
        let candidates = result.entry(alias.into()).or_default();
        if !candidates
            .iter()
            .any(|candidate| candidate.source == source)
        {
            candidates.push(ResolvedPackage {
                package: package.into(),
                version: version.into(),
                source: source.into(),
            });
        }
    }
    Some(result)
}

fn split_source(source: &str) -> Option<(&str, &str)> {
    let (package, version) = source.rsplit_once('@')?;
    let mut components = package.split('/');
    let scope = components.next()?;
    let name = components.next()?;
    let valid_part = |part: &str| {
        !part.is_empty()
            && part
                .bytes()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
    };
    (components.next().is_none()
        && valid_part(scope)
        && valid_part(name)
        && !version.trim().is_empty())
    .then_some((package, version))
}

fn safe_alias(alias: &str) -> bool {
    !alias.is_empty()
        && alias != "."
        && alias != ".."
        && !alias
            .chars()
            .any(|c| c.is_control() || "/\\:*?\"<>|".contains(c))
}

fn parse_dependencies(
    manifest: &toml::Value,
    root: &Path,
    file: &mut FileSnapshot,
    lock_file: &FileSnapshot,
    locked: Option<&BTreeMap<String, Vec<ResolvedPackage>>>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<PackageDependency> {
    let package = manifest.get("package");
    let valid_package = package.is_some_and(|package| {
        package
            .get("name")
            .and_then(toml::Value::as_str)
            .is_some_and(|name| split_source(&format!("{name}@0.0.0")).is_some())
            && package
                .get("version")
                .and_then(toml::Value::as_str)
                .is_some_and(|version| Version::parse(version).is_ok())
            && package
                .get("realm")
                .and_then(toml::Value::as_str)
                .is_some_and(|realm| matches!(realm, "shared" | "server" | "dev"))
    });
    if !valid_package {
        invalid(
            file,
            diagnostics,
            "invalid_wally_package",
            "The [package] table needs a scope/name, SemVer version, and shared/server/dev realm"
                .into(),
        );
    }
    let mut result = Vec::new();
    for realm in [Realm::Shared, Realm::Server, Realm::Dev] {
        let Some(section) = manifest.get(realm.section()) else {
            continue;
        };
        let Some(dependencies) = section.as_table() else {
            invalid(
                file,
                diagnostics,
                "invalid_dependencies",
                format!("{} must be a table", realm.section()),
            );
            continue;
        };
        for (alias, source) in dependencies {
            let declaration = source
                .as_str()
                .map(str::to_owned)
                .unwrap_or_else(|| source.to_string());
            let parts = source.as_str().and_then(split_source);
            let requirement = parts.and_then(|(_, version)| VersionReq::parse(version).ok());
            let valid = safe_alias(alias) && requirement.is_some();
            if !valid {
                invalid(
                    file,
                    diagnostics,
                    "invalid_dependency",
                    format!(
                        "Invalid Wally dependency {}.{alias}: {declaration}",
                        realm.section()
                    ),
                );
            }
            let candidates = locked.and_then(|locked| locked.get(alias));
            let matches: Vec<&ResolvedPackage> = candidates
                .into_iter()
                .flatten()
                .filter(|candidate| {
                    parts.is_some_and(|(package, _)| candidate.package == package)
                        && requirement.as_ref().is_some_and(|requirement| {
                            Version::parse(&candidate.version)
                                .is_ok_and(|version| requirement.matches(&version))
                        })
                })
                .collect();
            // Wally merges all three realms into the root lock entry. Repeated
            // aliases are legal, but the lock does not identify their realms.
            let resolved = match matches.as_slice() {
                [resolved] => Some((*resolved).clone()),
                [] => candidates
                    .filter(|candidates| candidates.len() == 1)
                    .map(|candidates| candidates[0].clone()),
                _ => None,
            };
            let ambiguous = matches.len() > 1
                || (matches.is_empty()
                    && candidates.is_some_and(|candidates| candidates.len() > 1));
            let resolution = if !valid
                || matches!(lock_file.state, FileState::Invalid | FileState::Unreadable)
                || ambiguous
            {
                ResolutionState::Unknown
            } else if lock_file.state == FileState::Missing {
                ResolutionState::Unlocked
            } else if let (Some(resolved), Some((package, _)), Some(requirement)) =
                (&resolved, parts, requirement)
            {
                if resolved.package == package
                    && Version::parse(&resolved.version)
                        .is_ok_and(|version| requirement.matches(&version))
                {
                    ResolutionState::Resolved
                } else {
                    ResolutionState::Stale
                }
            } else if locked.is_some() {
                ResolutionState::Stale
            } else {
                ResolutionState::Unknown
            };
            if ambiguous {
                warning(
                    lock_file,
                    diagnostics,
                    "ambiguous_resolution",
                    format!(
                        "The lockfile does not unambiguously identify the realm's resolution for {}.{alias}",
                        realm.section()
                    ),
                );
            }
            if resolution == ResolutionState::Stale {
                warning(
                    lock_file,
                    diagnostics,
                    "stale_dependency",
                    format!(
                        "{}.{alias} is not resolved to a version allowed by {declaration}",
                        realm.section()
                    ),
                );
            }
            result.push(PackageDependency {
                alias: alias.clone(),
                realm,
                package: parts.map(|(package, _)| package.into()),
                version_requirement: parts.map(|(_, version)| version.into()),
                declaration,
                resolution,
                resolved,
                presence: presence(root, realm, alias),
            });
        }
    }
    if let Some(locked) = locked {
        for alias in locked
            .keys()
            .filter(|alias| !result.iter().any(|dependency| &dependency.alias == *alias))
        {
            warning(
                lock_file,
                diagnostics,
                "removed_dependency",
                format!("Locked alias {alias} is no longer declared in wally.toml"),
            );
        }
    }
    result
}

fn presence(root: &Path, realm: Realm, alias: &str) -> PackagePresence {
    if !safe_alias(alias) {
        return PackagePresence {
            state: PresenceState::Unknown,
            path: None,
        };
    }
    let directory = root.join(realm.directory());
    let mut unreadable = false;
    for extension in ["luau", "lua"] {
        let path = directory.join(format!("{alias}.{extension}"));
        match fs::metadata(&path) {
            Ok(meta) if meta.is_file() => {
                return PackagePresence {
                    state: match fs::metadata(directory.join("_Index")) {
                        Ok(index) if index.is_dir() => PresenceState::Present,
                        Ok(_) => PresenceState::Missing,
                        Err(error) if error.kind() == io::ErrorKind::NotFound => {
                            PresenceState::Missing
                        }
                        Err(_) => PresenceState::Unknown,
                    },
                    path: Some(path),
                };
            }
            Err(error) if error.kind() != io::ErrorKind::NotFound => unreadable = true,
            _ => {}
        }
    }
    PackagePresence {
        state: if unreadable {
            PresenceState::Unknown
        } else {
            PresenceState::Missing
        },
        path: None,
    }
}

fn parse_tools(
    value: &toml::Value,
    file: &mut FileSnapshot,
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<ToolPin> {
    let Some(tools) = value.get("tools") else {
        return Vec::new();
    };
    let Some(tools) = tools.as_table() else {
        invalid(
            file,
            diagnostics,
            "invalid_tools",
            "[tools] must be a table".into(),
        );
        return Vec::new();
    };
    tools
        .iter()
        .map(|(alias, value)| {
            let source = value
                .as_str()
                .map(str::to_owned)
                .unwrap_or_else(|| value.to_string());
            let pin = value.as_str().and_then(|source| {
                let (identity, version) = source.split_once('@')?;
                (!identity.trim().is_empty()
                    && !version.trim().is_empty()
                    && !version.contains('@'))
                .then_some(version.trim())
            });
            if pin.is_none() || !safe_alias(alias) {
                invalid(
                    file,
                    diagnostics,
                    "invalid_tool",
                    format!("Tool {alias} needs a nonempty source@version string and an executable alias"),
                );
            }
            ToolPin {
                alias: alias.clone(),
                version_requirement: pin.map(str::to_owned),
                source,
                executable: find_executable(alias),
            }
        })
        .collect()
}

fn find_executable(alias: &str) -> Option<PathBuf> {
    if !safe_alias(alias) {
        return None;
    }
    let path = std::env::var_os("PATH")?;
    let suffixes = if cfg!(windows) {
        vec![".exe", ".cmd", ".bat", ""]
    } else {
        vec![""]
    };
    for directory in std::env::split_paths(&path).filter(|directory| directory.is_absolute()) {
        for suffix in &suffixes {
            let candidate = directory.join(format!("{alias}{suffix}"));
            if executable_file(&candidate) {
                return Some(candidate);
            }
        }
    }
    None
}

fn executable_file(path: &Path) -> bool {
    let Ok(metadata) = fs::metadata(path) else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

fn actions(files: &ProjectFiles, saved: Option<&SavedProject>) -> ProjectActions {
    let invalid_runtime_file = [&files.rproj, &files.wally, &files.rokit]
        .iter()
        .find(|file| matches!(file.state, FileState::Invalid | FileState::Unreadable))
        .map(|file| {
            format!(
                "Resolve diagnostics for {} first",
                file.path.file_name().unwrap_or_default().to_string_lossy()
            )
        });
    let test_runner = saved.and_then(|saved| {
        saved
            .capabilities
            .get("test")
            .map(String::as_str)
            .or_else(|| {
                saved
                    .packages
                    .iter()
                    .any(|package| package == "testez")
                    .then_some("testez")
            })
    });
    // Existing projects use TestEZ for legacy/unknown test capability keys.
    let jest = matches!(test_runner, Some("jest-roblox" | "jest-roblox-open-cloud"));
    let jest_problem =
        if jest && saved.is_some_and(|saved| saved.package_workflow != PackageWorkflow::Wally) {
            Some("Jest Roblox requires the Wally dependency workflow")
        } else if jest && files.wally.state != FileState::Ready {
            Some("Jest Roblox requires a valid wally.toml")
        } else {
            None
        };
    let watch = if files.rojo.state != FileState::Ready {
        ActionAvailability::no("A valid default.project.json is required")
    } else if let Some(reason) = &invalid_runtime_file {
        ActionAvailability::no(reason)
    } else if let Some(reason) = jest_problem {
        ActionAvailability::no(reason)
    } else {
        ActionAvailability::yes()
    };
    let check = if files.check_script.state == FileState::Ready {
        ActionAvailability::yes()
    } else {
        ActionAvailability::no("No readable .lute/check.luau quality script")
    };
    let test = if let Some(reason) = invalid_runtime_file {
        ActionAvailability::no(reason)
    } else if let Some(reason) = jest_problem {
        ActionAvailability::no(reason)
    } else if test_runner.is_some() && files.rojo.state == FileState::Ready {
        ActionAvailability::yes()
    } else {
        ActionAvailability::no(
            "Testing needs a configured TestEZ or Jest Roblox project and valid default.project.json",
        )
    };
    ProjectActions { watch, check, test }
}
