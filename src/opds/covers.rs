use super::auth::OpdsAuth;
use eframe::egui::{self, ColorImage, TextureHandle, TextureOptions};
use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread;
use std::time::Duration;

const MAX_COVER_BYTES: u64 = 5 * 1024 * 1024;
const MAX_COVER_DIMENSION: u32 = 2048;
const MAX_COVER_ALLOC: u64 = 32 * 1024 * 1024;
const MAX_CONCURRENT_COVER_FETCHES: usize = 4;

pub struct OpdsCoverManager {
    cache: HashMap<String, Option<TextureHandle>>,
    in_flight: HashSet<String>,
    tx_image: Sender<(String, Option<ColorImage>)>,
    rx_image: Receiver<(String, Option<ColorImage>)>,
}

impl Default for OpdsCoverManager {
    fn default() -> Self {
        Self::new()
    }
}

impl OpdsCoverManager {
    pub fn new() -> Self {
        let (tx_image, rx_image) = channel();
        Self {
            cache: HashMap::new(),
            in_flight: HashSet::new(),
            tx_image,
            rx_image,
        }
    }

    pub fn poll(&mut self, ctx: &egui::Context) {
        let mut got_new = false;
        while let Ok((url, opt_img)) = self.rx_image.try_recv() {
            self.in_flight.remove(&url);
            match opt_img {
                Some(img) => {
                    let handle = ctx.load_texture(
                        format!("opds_cover_{}", url.chars().take(60).collect::<String>()),
                        img,
                        TextureOptions::LINEAR,
                    );
                    self.cache.insert(url, Some(handle));
                    got_new = true;
                }
                None => {
                    self.cache.insert(url, None);
                }
            }
        }
        if got_new {
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
        if let Some(cached) = self.cache.get(url) {
            return cached.clone();
        }

        if self.in_flight.len() >= MAX_CONCURRENT_COVER_FETCHES {
            return None;
        }

        if !self.in_flight.contains(url) {
            self.in_flight.insert(url.to_string());
            let tx = self.tx_image.clone();
            let cover_url = url.to_string();
            let spawn_result = thread::Builder::new()
                .name("opds-cover-fetch".to_string())
                .spawn(move || {
                    let agent = ureq::Agent::config_builder()
                        .timeout_global(Some(Duration::from_secs(6)))
                        .build()
                        .new_agent();

                    let mut req = agent
                        .get(&cover_url)
                        .header("User-Agent", "QuietReader/0.7.1 (OPDS Cover)")
                        .header("Accept", "image/jpeg,image/png,image/webp,image/*,*/*");

                    if let Some(header) = auth.as_ref().and_then(|auth| auth.header_for(&cover_url))
                    {
                        req = req.header("Authorization", header);
                    }

                    let res = req.call().ok().and_then(|mut resp| {
                        if !resp.status().is_success() {
                            return None;
                        }
                        let length = resp
                            .headers()
                            .get("content-length")
                            .and_then(|value| value.to_str().ok())
                            .and_then(|value| value.parse::<u64>().ok());
                        if length.is_some_and(|value| value > MAX_COVER_BYTES) {
                            return None;
                        }
                        let mut bytes = Vec::new();
                        resp.body_mut()
                            .as_reader()
                            .take(MAX_COVER_BYTES + 1)
                            .read_to_end(&mut bytes)
                            .ok()?;
                        if bytes.len() as u64 > MAX_COVER_BYTES {
                            return None;
                        }
                        let mut reader =
                            image::ImageReader::new(std::io::Cursor::new(bytes.as_slice()))
                                .with_guessed_format()
                                .ok()?;
                        let mut limits = image::Limits::default();
                        limits.max_image_width = Some(MAX_COVER_DIMENSION);
                        limits.max_image_height = Some(MAX_COVER_DIMENSION);
                        limits.max_alloc = Some(MAX_COVER_ALLOC);
                        reader.limits(limits);
                        let dynamic_img = reader.decode().ok()?;
                        let rgba = dynamic_img.to_rgba8();
                        let (w, h) = rgba.dimensions();
                        Some(ColorImage::from_rgba_unmultiplied(
                            [w as usize, h as usize],
                            rgba.as_raw(),
                        ))
                    });

                    let _ = tx.send((cover_url, res));
                });

            if spawn_result.is_err() {
                self.in_flight.remove(url);
            }
        }

        None
    }
}
