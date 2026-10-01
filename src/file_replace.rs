use std::{fs, io::Write, path::Path};

use anyhow::{Context, Result};

mod permissions;

/// Prepare a complete sibling file without modifying the destination.
pub(crate) fn stage(
    path: &Path,
    contents: &[u8],
    existing: bool,
) -> Result<tempfile::NamedTempFile> {
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
