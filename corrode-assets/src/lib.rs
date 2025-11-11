use gpui::AssetSource;
use std::borrow::Cow;

mod assets;

pub use assets::CorrodeAssetSource;

include!(concat!(env!("OUT_DIR"), "/out.rs"));

impl AssetSource for CorrodeAssetSource {
    fn load(&self, path: &str) -> gpui::Result<Option<Cow<'static, [u8]>>> {
        if path.is_empty() {
            return Ok(None);
        }

        Ok(Self::get(path).map(|f| f.data))
    }

    fn list(&self, path: &str) -> gpui::Result<Vec<gpui::SharedString>> {
        Ok(Self::iter()
            .filter_map(|p| p.starts_with(path).then(|| p.into()))
            .collect())
    }
}
