//! Stable identity checks without following a linked leaf.
use anyhow::{Context, Result, ensure};
use std::{fs, io::ErrorKind, path::Path};

pub(crate) fn identity(path: &Path, directory: bool) -> Result<Option<String>> {
    let meta = match fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e).with_context(|| format!("Cannot inspect {}", path.display())),
    };
    ensure!(
        !meta.file_type().is_symlink(),
        "{} is linked; use regular project files",
        path.display()
    );
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        ensure!(
            meta.file_attributes() & 0x400 == 0,
            "{} is a reparse point; use regular project files",
            path.display()
        );
    }
    ensure!(
        if directory {
            meta.is_dir()
        } else {
            meta.is_file()
        },
        "Unexpected file type at {}",
        path.display()
    );
    #[cfg(windows)]
    {
        use winsafe::{HFILE, co};
        let (handle, _) = HFILE::CreateFile(
            path.to_str()
                .context("Settings path is not valid Unicode")?,
            co::GENERIC::default(),
            Some(co::FILE_SHARE::READ | co::FILE_SHARE::WRITE | co::FILE_SHARE::DELETE),
            None,
            co::DISPOSITION::OPEN_EXISTING,
            co::FILE_ATTRIBUTE::NORMAL,
            Some(co::FILE_FLAG::BACKUP_SEMANTICS | co::FILE_FLAG::OPEN_REPARSE_POINT),
            None,
            None,
        )?;
        let info = handle.GetFileInformationByHandle()?;
        ensure!(
            !info.dwFileAttributes.has(co::FILE_ATTRIBUTE::REPARSE_POINT),
            "Settings path became a reparse point"
        );
        Ok(Some(format!(
            "{}:{}",
            info.dwVolumeSerialNumber,
            info.nFileIndex()
        )))
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        Ok(Some(format!("{}:{}", meta.dev(), meta.ino())))
    }
    #[cfg(not(any(windows, unix)))]
    {
        Ok(Some(format!("{:?}", meta.created()?)))
    }
}
