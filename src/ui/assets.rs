//! The few SVG icons the UI draws, compiled into the binary.

use gpui::{AssetSource, Result, SharedString};
use std::borrow::Cow;

pub const REFRESH: &str = "ui/refresh.svg";
pub const EDIT: &str = "ui/edit.svg";
pub const SPINNER: &str = "ui/spinner.svg";
pub const CHEVRON: &str = "ui/chevron-down.svg";
pub const MINIMIZE: &str = "ui/minimize.svg";
pub const MAXIMIZE: &str = "ui/maximize.svg";
pub const RESTORE: &str = "ui/restore.svg";
pub const CLOSE: &str = "ui/close.svg";

const FILES: &[(&str, &[u8])] = &[
    (REFRESH, include_bytes!("../../assets/ui/refresh.svg")),
    (EDIT, include_bytes!("../../assets/ui/edit.svg")),
    (SPINNER, include_bytes!("../../assets/ui/spinner.svg")),
    (CHEVRON, include_bytes!("../../assets/ui/chevron-down.svg")),
    (MINIMIZE, include_bytes!("../../assets/ui/minimize.svg")),
    (MAXIMIZE, include_bytes!("../../assets/ui/maximize.svg")),
    (RESTORE, include_bytes!("../../assets/ui/restore.svg")),
    (CLOSE, include_bytes!("../../assets/ui/close.svg")),
];

pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        Ok(FILES
            .iter()
            .find(|(p, _)| *p == path)
            .map(|(_, bytes)| Cow::Borrowed(*bytes)))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        Ok(FILES
            .iter()
            .filter(|(p, _)| p.starts_with(path))
            .map(|(p, _)| SharedString::from(*p))
            .collect())
    }
}
