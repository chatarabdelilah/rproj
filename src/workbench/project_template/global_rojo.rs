//! Resolve an installed machine tool without executing a project-aware launcher.
use crate::{config::template_checked, path_identity::identity};
use anyhow::{Context, Result, ensure};
use rproj_core::project_template::protocol::{GlobalRojo, IPC_LIMIT};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};

pub(super) struct MachineTools {
    pub rokit_root: PathBuf,
    pub search: Vec<PathBuf>,
}
impl MachineTools {
    pub fn system() -> Result<Self> {
        let rokit_root = match std::env::var_os("ROKIT_ROOT") {
            Some(root) => PathBuf::from(root),
            None => directories::BaseDirs::new()
                .context("Cannot locate the machine tool directory")?
                .home_dir()
                .join(".rokit"),
        };
        ensure!(
            rokit_root.is_absolute(),
            "ROKIT_ROOT must be absolute for machine-wide template validation"
        );
        let search = std::env::var_os("PATH")
            .map(|paths| {
                std::env::split_paths(&paths)
                    .filter(|path| path.is_absolute())
                    .collect()
            })
            .unwrap_or_default();
        Ok(Self { rokit_root, search })
    }

    pub fn resolve(&self) -> Result<GlobalRojo> {
        let manifest =
            template_checked::capture(&self.rokit_root.join("rokit.toml"), Some(IPC_LIMIT))?;
        let spec = if let Some(text) = &manifest.original {
            let value: toml::Table = toml::from_str(text)
                .context("Global rokit.toml is malformed; repair it before template validation")?;
            let tools = value
                .get("tools")
                .map(|value| {
                    value
                        .as_table()
                        .context("Global Rokit tools must be a table")
                })
                .transpose()?;
            let choices: Vec<_> = tools
                .into_iter()
                .flat_map(|tools| tools.iter())
                .filter(|(key, _)| key.eq_ignore_ascii_case("rojo"))
                .collect();
            ensure!(
                choices.len() <= 1,
                "Global Rokit manifest has ambiguous Rojo aliases"
            );
            choices
                .first()
                .map(|(_, value)| {
                    value
                        .as_str()
                        .context("Global Rojo pin must be a string")
                        .map(str::to_owned)
                })
                .transpose()?
        } else {
            None
        };
        let executable = if let Some(spec) = spec {
            let spec = spec.strip_prefix("github:").unwrap_or(&spec);
            let (repository, version) = spec.split_once('@').context(
                "Global Rojo needs an exact installed version; use `rokit add --global rojo`",
            )?;
            let version = semver::Version::parse(version)
                .context("Global Rojo version is not an exact version")?;
            let (author, name) = repository
                .split_once('/')
                .context("Global Rojo pin must name its GitHub repository")?;
            for component in [author, name] {
                ensure!(
                    !component.is_empty()
                        && component != "."
                        && component != ".."
                        && component
                            .bytes()
                            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.')),
                    "Unsafe global Rojo repository component"
                );
            }
            // Rokit 1.2 stores installed executables by lower-case owner/repository/version.
            self.rokit_root
                .join("tool-storage")
                .join(author.to_ascii_lowercase())
                .join(name.to_ascii_lowercase())
                .join(version.to_string())
                .join(format!(
                    "{}{suffix}",
                    name.to_ascii_lowercase(),
                    suffix = std::env::consts::EXE_SUFFIX
                ))
        } else {
            self.standalone()?
        };
        let fingerprint = fingerprint(&executable).context("Cannot use the installed global Rojo executable; use Machine Setup or `rokit add --global rojo`")?;
        template_checked::verify_with_limit(&manifest, Some(IPC_LIMIT))?;
        Ok(GlobalRojo {
            executable,
            identity: fingerprint,
            manifest,
        })
    }
    fn standalone(&self) -> Result<PathBuf> {
        for directory in &self.search {
            let launcher_directory = directory.components().any(|part| {
                part.as_os_str().to_str().is_some_and(|name| {
                    [".aftman", ".foreman", ".rokit"]
                        .iter()
                        .any(|launcher| name.eq_ignore_ascii_case(launcher))
                })
            });
            if !directory.is_absolute()
                || directory.starts_with(&self.rokit_root)
                || launcher_directory
            {
                continue;
            }
            let path = directory.join(format!("rojo{}", std::env::consts::EXE_SUFFIX));
            match fs::symlink_metadata(&path) {
                Ok(_) => return Ok(path),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(error) => {
                    return Err(error).with_context(|| {
                        format!("Cannot inspect machine tool {}", path.display())
                    });
                }
            }
        }
        anyhow::bail!(
            "Global Rojo is unavailable. Use Machine Setup or `rokit add --global rojo`, then prepare the review again. Editing and Reset remain available."
        )
    }
}

fn fingerprint(path: &Path) -> Result<String> {
    let mut parents = Vec::new();
    for parent in path
        .ancestors()
        .skip(1)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
    {
        parents.push((
            parent,
            identity(parent, true)?.context("Global Rojo directory is missing")?,
        ));
    }
    let captured = identity(path, false)?.context("Global Rojo is not installed")?;
    let before = fs::metadata(path)?;
    let mut file = fs::File::open(path)?;
    let mut digest = blake3::Hasher::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    for (parent, expected) in parents {
        ensure!(
            identity(parent, true)?.as_ref() == Some(&expected),
            "Global Rojo directory changed during capture"
        );
    }
    let after = fs::metadata(path)?;
    ensure!(
        identity(path, false)?.as_ref() == Some(&captured)
            && before.len() == after.len()
            && before.modified()? == after.modified()?,
        "Global Rojo changed during capture; prepare again"
    );
    Ok(format!("{captured}:{}", digest.finalize().to_hex()))
}
