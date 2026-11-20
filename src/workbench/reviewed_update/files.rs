use crate::path_identity::identity;
use anyhow::{Context, Result, ensure};
use rproj_core::creation::protocol::IPC_LIMIT;
use std::{
    collections::BTreeMap,
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub(super) struct FileState {
    pub id: Option<String>,
    pub content: Option<String>,
}

pub(super) fn file_state(path: &Path, directory: bool) -> Result<FileState> {
    let id = identity(path, directory)?;
    let content = if id.is_some() && !directory {
        let mut h = blake3::Hasher::new();
        h.update_reader(fs::File::open(path)?)?;
        Some(h.finalize().to_string())
    } else {
        None
    };
    ensure!(
        identity(path, directory)? == id,
        "{} changed during inspection",
        path.display()
    );
    Ok(FileState { id, content })
}

pub(super) fn text(path: &Path) -> Result<String> {
    ensure!(
        identity(path, false)?.is_some(),
        "Missing {}",
        path.display()
    );
    let mut data = String::new();
    fs::File::open(path)?
        .take(IPC_LIMIT + 1)
        .read_to_string(&mut data)?;
    ensure!(
        data.len() as u64 <= IPC_LIMIT,
        "{} exceeds the review limit",
        path.display()
    );
    Ok(data)
}

pub(super) type Tree = BTreeMap<PathBuf, FileState>;
pub(super) fn tree(base: &Path, name: &Path) -> Result<Tree> {
    let mut result = Tree::new();
    let path = base.join(name);
    if matches!(fs::symlink_metadata(&path),Err(e) if e.kind()==std::io::ErrorKind::NotFound) {
        return Ok(result);
    }
    for item in walkdir::WalkDir::new(&path).follow_links(false) {
        let item = item?;
        ensure!(
            result.len() < 100_000,
            "Too many files to verify in {}",
            path.display()
        );
        let relative = item.path().strip_prefix(base)?.to_owned();
        let state = file_state(item.path(), item.file_type().is_dir())?;
        ensure!(state.id.is_some(), "{} disappeared", item.path().display());
        result.insert(relative, state);
    }
    Ok(result)
}

pub(super) fn contents(tree: &Tree) -> BTreeMap<&PathBuf, &Option<String>> {
    tree.iter().map(|(p, e)| (p, &e.content)).collect()
}

pub(super) fn digest(value: &impl serde::Serialize) -> Result<String> {
    Ok(blake3::hash(&serde_json::to_vec(value)?).to_string())
}

pub(super) fn replace(path: &Path, bytes: &[u8], expected: &FileState) -> Result<()> {
    let mut tmp = tempfile::NamedTempFile::new_in(path.parent().context("Missing parent")?)?;
    tmp.write_all(bytes)?;
    tmp.as_file().sync_all()?;
    if expected.id.is_some() {
        tmp.as_file()
            .set_permissions(fs::metadata(path)?.permissions())?;
    }
    ensure!(
        file_state(path, false)? == *expected,
        "{} changed; check for updates again",
        path.display()
    );
    if expected.id.is_some() {
        tmp.persist(path).map_err(|e| e.error)?;
    } else {
        tmp.persist_noclobber(path).map_err(|e| e.error)?;
    }
    Ok(())
}

pub(super) fn copy_tree(from: &Path, to: &Path, entries: &Tree) -> Result<()> {
    for (relative, entry) in entries {
        if entry.content.is_none() {
            fs::create_dir_all(to.join(relative))?;
        } else {
            let dest = to.join(relative);
            fs::create_dir_all(dest.parent().context("Missing file parent")?)?;
            fs::copy(from.join(relative), &dest)?;
            ensure!(
                file_state(&dest, false)?.content == entry.content,
                "{} changed while copying",
                relative.display()
            );
        }
    }
    Ok(())
}
