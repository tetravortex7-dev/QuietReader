use crate::opds::covers::{
    BoundedLruMap, DEFAULT_MAX_COVER_CACHE_BYTES, DEFAULT_MAX_COVER_CACHE_ENTRIES,
};
use crate::opds::{OpdsAuth, OpdsCoverManager};
use eframe::egui::{self, ColorImage, TextureHandle, TextureOptions};

pub struct OpdsCoverCache {
    manager: OpdsCoverManager,
    textures: BoundedLruMap<TextureHandle>,
}

impl Default for OpdsCoverCache {
    fn default() -> Self {
        Self::new()
    }
}

impl OpdsCoverCache {
    pub fn new() -> Self {
        Self::with_limits(
            DEFAULT_MAX_COVER_CACHE_ENTRIES,
            DEFAULT_MAX_COVER_CACHE_BYTES,
        )
    }

    pub fn with_limits(max_entries: usize, max_bytes: usize) -> Self {
        Self {
            manager: OpdsCoverManager::with_limits(max_entries, max_bytes),
            textures: BoundedLruMap::new(max_entries, max_bytes),
        }
    }

    pub fn clear(&mut self) {
        self.textures.clear();
        self.manager.clear();
    }

    pub fn poll(&mut self, ctx: &egui::Context) {
        if self.manager.poll() {
            ctx.request_repaint();
        }
    }

    pub fn get_texture(
        &mut self,
        ctx: &egui::Context,
        url: &str,
        auth: Option<OpdsAuth>,
    ) -> Option<TextureHandle> {
        self.poll(ctx);
        if let Some(cached) = self.textures.get_cloned(url) {
            return Some(cached);
        }

        let cover = self.manager.get_cover(url, auth)?;
        let byte_size = cover.width.saturating_mul(cover.height).saturating_mul(4);
        let handle = ctx.load_texture(
            format!("opds_cover_{url}"),
            ColorImage::from_rgba_unmultiplied([cover.width, cover.height], &cover.rgba),
            TextureOptions::LINEAR,
        );
        self.textures
            .insert(url.to_string(), handle.clone(), byte_size);
        Some(handle)
    }
}
