use gpui_kit::{AssetSource, SharedString};
use std::borrow::Cow;

gpui_kit::assets::icon_assets!(WorkbenchIcons, [Package, Wrench, FlaskConical, Plus]);

pub(crate) struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> anyhow::Result<Option<Cow<'static, [u8]>>> {
        match WorkbenchIcons.load(path)? {
            Some(icon) => Ok(Some(icon)),
            None => gpui_kit::assets::Assets.load(path),
        }
    }

    fn list(&self, path: &str) -> anyhow::Result<Vec<SharedString>> {
        let mut paths = gpui_kit::assets::Assets.list(path)?;
        paths.extend(WorkbenchIcons.list(path)?);
        paths.sort();
        paths.dedup();
        Ok(paths)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::assets::IconName;

    #[test]
    fn navigation_and_existing_control_icons_are_bundled() {
        for icon in [
            IconName::LayoutDashboard,
            IconName::Package,
            IconName::Wrench,
            IconName::FlaskConical,
            IconName::FolderOpen,
            IconName::ChevronDown,
            IconName::ChevronUp,
        ] {
            assert!(!Assets.load(&icon.path()).unwrap().unwrap().is_empty());
        }
    }
}
