use std::{
    fs,
    io::{ErrorKind, Write},
    path::Path,
};

use anyhow::{Context, Result, ensure};

mod permissions;

/// Prepare a complete sibling file without modifying the destination.
pub(crate) fn stage(
    path: &Path,
    contents: &[u8],
    existing: bool,
) -> Result<tempfile::NamedTempFile> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => ensure!(
            !metadata.file_type().is_symlink(),
            "{} is a symbolic link. Use a regular project file or edit its linked configuration directly.",
            path.display()
        ),
        Err(error) if error.kind() == ErrorKind::NotFound => {}
        Err(error) => {
            return Err(error).with_context(|| format!("failed to inspect {}", path.display()));
        }
    }
    stage_with(path, existing, |pending| {
        pending.write_all(contents)?;
        pending.as_file().sync_all()?;
        Ok(())
    })
}

fn stage_with(
    path: &Path,
    existing: bool,
    write: impl FnOnce(&mut tempfile::NamedTempFile) -> Result<()>,
) -> Result<tempfile::NamedTempFile> {
    let parent = path.parent().context("output path has no parent")?;
    fs::create_dir_all(parent)?;
    let mut pending = tempfile::NamedTempFile::new_in(parent)?;
    if existing {
        permissions::preserve(path, &pending)?;
    }
    write(&mut pending)?;
    Ok(pending)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(any(unix, windows))]
    #[cfg_attr(
        windows,
        ignore = "requires Windows symlink privilege or Developer Mode"
    )]
    #[test]
    fn symbolic_link_targets_keep_the_link_and_shared_configuration() {
        let root = tempfile::tempdir().unwrap();
        let shared = root.path().join("shared.toml");
        let target = root.path().join("selene.toml");
        let original = b"std='roblox'\n";
        fs::write(&shared, original).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&shared, &target).unwrap();
        #[cfg(windows)]
        std::os::windows::fs::symlink_file(&shared, &target).unwrap();
        let error = stage(&target, b"std='changed'\n", true).unwrap_err();
        assert!(error.to_string().contains("symbolic link"));
        assert!(
            fs::symlink_metadata(&target)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(fs::read(&shared).unwrap(), original);
        assert_eq!(fs::read(&target).unwrap(), original);
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 2);
    }

    #[test]
    fn a_partial_staging_write_keeps_the_destination_and_cleans_up() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("settings.json");
        let original = b"{\"custom\":true}\n";
        fs::write(&path, original).unwrap();
        let result = stage_with(&path, true, |pending| {
            pending.write_all(b"{\"incomplete\":")?;
            anyhow::bail!("injected write failure");
        });
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("injected write failure")
        );
        assert_eq!(fs::read(&path).unwrap(), original);
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
        let pending = stage(&path, b"{\"complete\":true}\n", true).unwrap();
        assert_eq!(fs::read(&path).unwrap(), original);
        pending.persist(&path).unwrap();
        assert_eq!(fs::read(path).unwrap(), b"{\"complete\":true}\n");
    }
}
