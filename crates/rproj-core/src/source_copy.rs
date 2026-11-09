//! Bounded, read-only source collection. Clipboard access belongs to frontends.
use anyhow::{Context, Result, bail, ensure};
use same_file::Handle;
use std::{
    fs,
    io::{self, Read},
    ops::Range,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

pub const MAX_PAYLOAD_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_VISITED_ENTRIES: usize = 10_000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IncludedFile {
    pub path: String,
    /// Byte range including the path header and two LF separators.
    pub range: Range<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SkippedPath {
    pub path: String,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub project: PathBuf,
    pub source_missing: bool,
    pub text: String,
    pub files: Vec<IncludedFile>,
    pub skipped: Vec<SkippedPath>,
}

pub fn collect(project: &Path) -> Result<Snapshot> {
    collect_with_abandonment(project, || false)
}

/// Checks between entries and read chunks cannot interrupt a blocked OS read.
pub fn collect_with_abandonment(
    project: &Path,
    mut abandoned: impl FnMut() -> bool,
) -> Result<Snapshot> {
    collect_limited(
        project,
        MAX_PAYLOAD_BYTES,
        MAX_VISITED_ENTRIES,
        &mut abandoned,
    )
}

fn checkpoint(abandoned: &mut impl FnMut() -> bool) -> Result<()> {
    ensure!(!abandoned(), "Source collection abandoned");
    Ok(())
}

fn unlinked(path: &Path) -> Result<()> {
    let meta =
        fs::symlink_metadata(path).with_context(|| format!("Cannot inspect {}", path.display()))?;
    // is_symlink includes Windows name-surrogate links/junctions, not every
    // reparse point: ordinary cloud placeholders are not link refusals.
    ensure!(
        !meta.file_type().is_symlink(),
        "{} is linked",
        path.display()
    );
    Ok(())
}

fn within_source(path: &Path, source: &Path) -> Result<()> {
    ensure!(path.starts_with(source), "Source path escaped src");
    for cursor in path.ancestors().take_while(|p| *p != source) {
        unlinked(cursor)?;
    }
    ensure!(
        path.canonicalize()?.starts_with(source),
        "Source path redirected outside src"
    );
    Ok(())
}

struct Directory {
    path: PathBuf,
    handle: Handle,
}

fn identity_handle(path: &Path) -> io::Result<Handle> {
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        // Metadata-only directory/file handle, permitting concurrent rename and
        // deletion. OPEN_REPARSE_POINT avoids following a newly linked leaf.
        fs::OpenOptions::new()
            .access_mode(0x80)
            .share_mode(0x7)
            .custom_flags(0x02000000 | 0x00200000)
            .open(path)
            .and_then(Handle::from_file)
    }
    #[cfg(not(windows))]
    {
        Handle::from_path(path)
    }
}

impl Directory {
    fn capture(path: &Path) -> Result<Self> {
        unlinked(path)?;
        ensure!(
            fs::metadata(path)?.is_dir(),
            "{} is not a directory",
            path.display()
        );
        let path = path
            .canonicalize()
            .with_context(|| format!("Cannot open {}", path.display()))?;
        let handle = identity_handle(&path)?;
        ensure!(
            handle.as_file().metadata()?.is_dir(),
            "{} is not a directory",
            path.display()
        );
        let result = Self { path, handle };
        result.verify()?;
        Ok(result)
    }
    fn verify(&self) -> Result<()> {
        unlinked(&self.path)?;
        ensure!(
            self.path.canonicalize()? == self.path && identity_handle(&self.path)? == self.handle,
            "Captured directory changed: {}",
            self.path.display()
        );
        Ok(())
    }
}

fn collect_limited(
    project: &Path,
    max_bytes: usize,
    max_entries: usize,
    abandoned: &mut impl FnMut() -> bool,
) -> Result<Snapshot> {
    checkpoint(abandoned)?;
    let project = Directory::capture(project)?;
    let source_path = project.path.join("src");
    let mut snapshot = Snapshot {
        project: project.path.clone(),
        source_missing: false,
        text: String::new(),
        files: Vec::new(),
        skipped: Vec::new(),
    };
    match fs::symlink_metadata(&source_path) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            project.verify()?;
            snapshot.source_missing = true;
            return Ok(snapshot);
        }
        Err(e) => return Err(e).context("Cannot inspect src"),
        Ok(_) => {}
    }
    let source = Directory::capture(&source_path)?;
    ensure!(
        source.path.starts_with(&project.path),
        "src escaped the captured project"
    );
    let mut entries = WalkDir::new(&source.path)
        .follow_links(false)
        .follow_root_links(false)
        // WalkDir spills an entire directory into memory when its open-handle
        // limit is reached. The visited-entry guard must fire before that spill.
        .max_open(max_entries.saturating_add(2))
        .into_iter();
    let mut visited = 0usize;
    while let Some(entry) = entries.next() {
        checkpoint(abandoned)?;
        project.verify()?;
        source.verify()?;
        let entry = entry.context("Cannot enumerate src completely; nothing copied")?;
        if entry.depth() == 0 {
            continue;
        }
        visited += 1;
        ensure!(
            visited <= max_entries,
            "Source exceeds {max_entries} visited entries; nothing copied"
        );
        let path = entry.path();
        let relative = path
            .strip_prefix(&project.path)
            .context("Source path escaped project")?
            .to_string_lossy()
            .replace('\\', "/");
        let meta = match fs::symlink_metadata(path) {
            Ok(meta) => meta,
            Err(error) if entry.file_type().is_dir() => {
                return Err(error).context("Cannot enumerate source directory");
            }
            Err(error) => {
                snapshot.skipped.push(SkippedPath {
                    path: relative,
                    reason: format!("Cannot inspect: {error}"),
                });
                continue;
            }
        };
        if meta.file_type().is_symlink() {
            if entry.file_type().is_dir() {
                entries.skip_current_dir();
            }
            snapshot.skipped.push(SkippedPath {
                path: relative,
                reason: "Linked path skipped".into(),
            });
            continue;
        }
        if meta.is_dir() {
            within_source(path, &source.path)?;
            continue;
        }
        if !meta.is_file() {
            snapshot.skipped.push(SkippedPath {
                path: relative,
                reason: "Not a regular file".into(),
            });
            continue;
        }
        let header = format!("// --- {relative} --- \n");
        let remaining = max_bytes
            .checked_sub(snapshot.text.len())
            .and_then(|n| n.checked_sub(header.len() + 2))
            .context("Source exceeds the combined payload byte limit; nothing copied")?;
        // A linked ancestor or out-of-root redirect is not an ordinary unreadable file.
        within_source(path, &source.path)?;
        let result = read_file(path, &source.path, &meta, remaining, abandoned)?;
        match result {
            Ok(content) => {
                let start = snapshot.text.len();
                snapshot.text.push_str(&header);
                snapshot.text.push_str(&content);
                snapshot.text.push_str("\n\n");
                snapshot.files.push(IncludedFile {
                    path: relative,
                    range: start..snapshot.text.len(),
                });
            }
            Err(reason) => snapshot.skipped.push(SkippedPath {
                path: relative,
                reason,
            }),
        }
    }
    checkpoint(abandoned)?;
    project.verify()?;
    source.verify()?;
    Ok(snapshot)
}

fn same_contents_metadata(a: &fs::Metadata, b: &fs::Metadata) -> bool {
    a.len() == b.len()
        && a.modified().ok() == b.modified().ok()
        && a.created().ok() == b.created().ok()
}

fn read_file(
    path: &Path,
    source: &Path,
    before: &fs::Metadata,
    remaining: usize,
    abandoned: &mut impl FnMut() -> bool,
) -> Result<std::result::Result<String, String>> {
    let file = match fs::File::open(path).and_then(Handle::from_file) {
        Ok(file) => file,
        Err(error) => return Ok(Err(format!("Unreadable: {error}"))),
    };
    let opened = file.as_file().metadata()?;
    ensure!(opened.is_file(), "Source changed into a non-regular file");
    if !same_contents_metadata(before, &opened) {
        return Ok(Err(
            "File changed before reading; Refresh to collect again".into()
        ));
    }
    ensure!(
        opened.len() <= remaining as u64,
        "Source exceeds the combined payload byte limit; nothing copied"
    );
    let bytes = match read_bounded(file.as_file(), remaining, abandoned)? {
        Ok(bytes) => bytes,
        Err(error) => return Ok(Err(format!("Unreadable: {error}"))),
    };
    let after = match fs::symlink_metadata(path) {
        Ok(after) => after,
        Err(error) => return Ok(Err(format!("File unavailable after reading: {error}"))),
    };
    if after.file_type().is_symlink() {
        return Ok(Err(
            "File became linked while reading; Refresh to collect again".into(),
        ));
    }
    within_source(path, source)?;
    let current = match identity_handle(path) {
        Ok(current) => current,
        Err(error) => return Ok(Err(format!("Cannot verify collected file: {error}"))),
    };
    if file != current
        || !same_contents_metadata(&opened, &after)
        || !same_contents_metadata(&opened, &file.as_file().metadata()?)
    {
        return Ok(Err(
            "File changed while reading; Refresh to collect again".into()
        ));
    }
    Ok(String::from_utf8(bytes).map_err(|_| "Not UTF-8 text".into()))
}

fn read_bounded(
    mut reader: impl Read,
    limit: usize,
    abandoned: &mut impl FnMut() -> bool,
) -> Result<io::Result<Vec<u8>>> {
    let mut bytes = Vec::new();
    let mut chunk = [0u8; 8192];
    loop {
        checkpoint(abandoned)?;
        let wanted = chunk.len().min(limit.saturating_sub(bytes.len()) + 1);
        let count = match reader.read(&mut chunk[..wanted]) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Ok(Err(e)),
        };
        if bytes.len() + count > limit {
            bail!("Source exceeds the combined payload byte limit; nothing copied");
        }
        bytes.extend_from_slice(&chunk[..count]);
    }
    Ok(Ok(bytes))
}

#[cfg(test)]
mod tests;
