use crate::opds::{OpdsAuth, OpdsCoverManager};
use eframe::egui::{self, ColorImage, TextureHandle, TextureOptions};
use std::collections::HashMap;

pub struct OpdsCoverCache {
    manager: OpdsCoverManager,
    textures: HashMap<String, Option<TextureHandle>>,
}

impl Default for OpdsCoverCache {
    fn default() -> Self {
        Self::new()
    }
}

impl OpdsCoverCache {
    pub fn new() -> Self {
        Self {
            manager: OpdsCoverManager::new(),
            textures: HashMap::new(),
        }
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
        if let Some(cached) = self.textures.get(url) {
            return cached.clone();
        }

        let cover = self.manager.get_cover(url, auth)?;
        let handle = ctx.load_texture(
            format!("opds_cover_{}", url.chars().take(60).collect::<String>()),
            ColorImage::from_rgba_unmultiplied([cover.width, cover.height], &cover.rgba),
            TextureOptions::LINEAR,
        );
        self.textures.insert(url.to_string(), Some(handle.clone()));
        Some(handle)
    }
}
