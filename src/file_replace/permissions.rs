use std::{fs, path::Path};

use anyhow::{Context, Result, ensure};

pub(super) fn preserve(source: &Path, pending: &tempfile::NamedTempFile) -> Result<()> {
    let permissions = fs::metadata(source)
        .with_context(|| format!("could not read permissions for {}", source.display()))?
        .permissions();
    ensure!(!permissions.readonly(), "{} is read-only", source.display());
    pending.as_file().set_permissions(permissions)?;
    #[cfg(windows)]
    preserve_dacl(source, pending.path()).with_context(|| {
        format!(
            "could not preserve access permissions for {}",
            source.display()
        )
    })?;
    Ok(())
}

#[cfg(windows)]
fn preserve_dacl(source: &Path, destination: &Path) -> Result<()> {
    use windows_permissions::{
        constants::{SeObjectType::SE_FILE_OBJECT, SecurityInformation},
        wrappers,
    };

    let descriptor =
        wrappers::GetNamedSecurityInfo(source, SE_FILE_OBJECT, SecurityInformation::Dacl)?;
    let sddl = wrappers::ConvertSecurityDescriptorToStringSecurityDescriptor(
        &descriptor,
        SecurityInformation::Dacl,
    )?;
    // SDDL's D: header carries P (protected), AI and AR before the first ACE.
    // Copy protection too, or inherited parent entries can broaden a custom DACL.
    let sddl = sddl.to_string_lossy();
    let flags = sddl
        .strip_prefix("D:")
        .context("Missing DACL header")?
        .split('(')
        .next()
        .unwrap_or_default();
    let protection = if flags.contains('P') {
        SecurityInformation::ProtectedDacl
    } else {
        SecurityInformation::UnprotectedDacl
    };
    wrappers::SetNamedSecurityInfo(
        destination,
        SE_FILE_OBJECT,
        SecurityInformation::Dacl | protection,
        None,
        None,
        descriptor.dacl(),
        None,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn change_width(root: &Path) -> Result<()> {
        let path = root.join("stylua.toml");
        crate::file_replace::stage(&path, b"column_width=100\n", true)?
            .persist(path)
            .map_err(|error| error.error)?;
        Ok(())
    }

    #[test]
    fn read_only_configuration_is_not_replaced() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("stylua.toml");
        fs::write(&path, "column_width=91\n").unwrap();
        let original = fs::metadata(&path).unwrap().permissions();
        let mut readonly = original.clone();
        readonly.set_readonly(true);
        fs::set_permissions(&path, readonly).unwrap();
        let result = change_width(root.path());
        fs::set_permissions(&path, original).unwrap();
        assert!(result.unwrap_err().to_string().contains("read-only"));
        assert_eq!(fs::read_to_string(&path).unwrap(), "column_width=91\n");
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
    }

    #[cfg(unix)]
    #[test]
    fn replacement_preserves_unix_mode() {
        use std::os::unix::fs::PermissionsExt;
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("stylua.toml");
        fs::write(&path, "column_width=91\n").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
        change_width(root.path()).unwrap();
        assert_eq!(
            fs::metadata(path).unwrap().permissions().mode() & 0o777,
            0o640
        );
    }

    #[cfg(windows)]
    #[test]
    fn replacement_preserves_windows_dacl_and_inheritance_protection() {
        use windows_permissions::{
            LocalBox, SecurityDescriptor,
            constants::{SeObjectType::SE_FILE_OBJECT, SecurityInformation},
            wrappers,
        };
        let dacl = |path: &Path| {
            let sd =
                wrappers::GetNamedSecurityInfo(path, SE_FILE_OBJECT, SecurityInformation::Dacl)
                    .unwrap();
            wrappers::ConvertSecurityDescriptorToStringSecurityDescriptor(
                &sd,
                SecurityInformation::Dacl,
            )
            .unwrap()
        };
        for protected in [false, true] {
            let root = tempfile::tempdir().unwrap();
            let path = root.path().join("stylua.toml");
            fs::write(&path, "column_width=91\n").unwrap();
            if protected {
                let initial = dacl(&path);
                let initial = initial.to_string_lossy();
                let entries = &initial[initial.find('(').unwrap()..];
                let sd: LocalBox<SecurityDescriptor> = format!("D:P{entries}").parse().unwrap();
                wrappers::SetNamedSecurityInfo(
                    &path,
                    SE_FILE_OBJECT,
                    SecurityInformation::Dacl | SecurityInformation::ProtectedDacl,
                    None,
                    None,
                    sd.dacl(),
                    None,
                )
                .unwrap();
            }
            let before = dacl(&path);
            change_width(root.path()).unwrap();
            let after = dacl(&path);
            let before = before.to_string_lossy();
            let after = after.to_string_lossy();
            let (before_flags, before_entries) = before.split_at(before.find('(').unwrap());
            let (after_flags, after_entries) = after.split_at(after.find('(').unwrap());
            assert_eq!(after_entries, before_entries);
            // Windows may record that automatic inheritance has run when setting
            // the DACL. Only that bookkeeping flag may differ, not protection.
            assert_eq!(
                after_flags.replace("AI", ""),
                before_flags.replace("AI", "")
            );
            assert!(fs::read_to_string(&path).unwrap().contains("100"));
        }
    }
}
