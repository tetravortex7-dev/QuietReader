use crate::app::rgba_byte_size;
use crate::document::{DecodedImage, Document, DocumentError};
use eframe::egui;
use std::collections::{HashMap, HashSet};
use std::io::Cursor;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender};

const MAX_CACHED_IMAGES: usize = 24;
const MAX_CACHE_BYTES: usize = 192 * 1024 * 1024;
const MAX_IMAGE_WIDTH: u32 = 8192;
const MAX_IMAGE_HEIGHT: u32 = 16384;
const MAX_IMAGE_ALLOC: u64 = 128 * 1024 * 1024;
const MAX_RENDER_WIDTH: u32 = 2048;
const MAX_RENDER_HEIGHT: u32 = 3072;

struct ImageRequest {
    generation: u64,
    doc: Arc<dyn Document>,
    id: String,
    ctx: egui::Context,
}

struct ImageResponse {
    generation: u64,
    id: String,
    result: Result<DecodedImage, DocumentError>,
}

struct CachedImage {
    texture: egui::TextureHandle,
    byte_size: usize,
    access: u64,
}

pub struct TextImageState {
    textures: HashMap<String, CachedImage>,
    pending: HashSet<String>,
    failed: HashSet<String>,
    total_bytes: usize,
    access_counter: u64,
    generation: u64,
    document_ptr: usize,
    req_tx: Sender<ImageRequest>,
    resp_rx: Receiver<ImageResponse>,
}

impl Default for TextImageState {
    fn default() -> Self {
        let (req_tx, resp_rx) = spawn_image_worker();
        Self {
            textures: HashMap::new(),
            pending: HashSet::new(),
            failed: HashSet::new(),
            total_bytes: 0,
            access_counter: 0,
            generation: 0,
            document_ptr: 0,
            req_tx,
            resp_rx,
        }
    }
}

fn spawn_image_worker() -> (Sender<ImageRequest>, Receiver<ImageResponse>) {
    let (req_tx, req_rx) = mpsc::channel::<ImageRequest>();
    let (resp_tx, resp_rx) = mpsc::channel::<ImageResponse>();

    std::thread::Builder::new()
        .name("text-image-worker".to_string())
        .spawn(move || {
            while let Ok(req) = req_rx.recv() {
                let result = req
                    .doc
                    .image_bytes(&req.id)
                    .ok_or_else(|| DocumentError::Corrupted("Изображение не найдено".to_string()))
                    .and_then(|bytes| decode_image_bytes(&bytes));
                let _ = resp_tx.send(ImageResponse {
                    generation: req.generation,
                    id: req.id,
                    result,
                });
                req.ctx.request_repaint();
            }
        })
        .ok();

    (req_tx, resp_rx)
}

fn decode_image_bytes(bytes: &[u8]) -> Result<DecodedImage, DocumentError> {
    if bytes.is_empty() {
        return Err(DocumentError::Corrupted(
            "Пустые данные изображения".to_string(),
        ));
    }
    let mut reader = image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| DocumentError::Corrupted(format!("Ошибка формата изображения: {e}")))?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_IMAGE_WIDTH);
    limits.max_image_height = Some(MAX_IMAGE_HEIGHT);
    limits.max_alloc = Some(MAX_IMAGE_ALLOC);
    reader.limits(limits);
    let image = reader.decode().map_err(|e| {
        DocumentError::Corrupted(format!("Не удалось декодировать изображение: {e}"))
    })?;
    let image = if image.width() > MAX_RENDER_WIDTH || image.height() > MAX_RENDER_HEIGHT {
        image.thumbnail(MAX_RENDER_WIDTH, MAX_RENDER_HEIGHT)
    } else {
        image
    };
    let rgba = image.to_rgba8();
    Ok(DecodedImage {
        width: rgba.width(),
        height: rgba.height(),
        rgba: rgba.into_raw(),
    })
}

impl TextImageState {
    pub fn ensure_document(&mut self, document: &Arc<dyn Document>) {
        let pointer = Arc::as_ptr(document) as *const () as usize;
        if pointer != self.document_ptr {
            self.clear();
            self.document_ptr = pointer;
        }
    }

    pub fn clear(&mut self) {
        self.textures.clear();
        self.pending.clear();
        self.failed.clear();
        self.total_bytes = 0;
        self.generation = self.generation.wrapping_add(1);
    }

    pub fn poll(&mut self, ctx: &egui::Context) {
        loop {
            match self.resp_rx.try_recv() {
                Ok(response) => {
                    if response.generation != self.generation {
                        continue;
                    }
                    self.pending.remove(&response.id);
                    match response.result {
                        Ok(image) => {
                            let color_image = egui::ColorImage::from_rgba_unmultiplied(
                                [image.width as usize, image.height as usize],
                                &image.rgba,
                            );
                            let texture = ctx.load_texture(
                                format!("text_image_{}_{}", self.generation, response.id),
                                color_image,
                                egui::TextureOptions::LINEAR,
                            );
                            let byte_size = rgba_byte_size(image.width, image.height);
                            self.access_counter += 1;
                            let access = self.access_counter;
                            if let Some(old) = self.textures.remove(&response.id) {
                                self.total_bytes = self.total_bytes.saturating_sub(old.byte_size);
                            }
                            while self.textures.len() >= MAX_CACHED_IMAGES
                                || self.total_bytes.saturating_add(byte_size) > MAX_CACHE_BYTES
                            {
                                let Some((oldest, _)) =
                                    self.textures.iter().min_by_key(|(_, entry)| entry.access)
                                else {
                                    break;
                                };
                                let oldest = oldest.clone();
                                if let Some(old) = self.textures.remove(&oldest) {
                                    self.total_bytes =
                                        self.total_bytes.saturating_sub(old.byte_size);
                                }
                            }
                            self.textures.insert(
                                response.id,
                                CachedImage {
                                    texture,
                                    byte_size,
                                    access,
                                },
                            );
                            self.total_bytes = self.total_bytes.saturating_add(byte_size);
                        }
                        Err(_) => {
                            self.failed.insert(response.id);
                        }
                    }
                }
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    let (tx, rx) = spawn_image_worker();
                    self.req_tx = tx;
                    self.resp_rx = rx;
                    self.pending.clear();
                    break;
                }
            }
        }
    }

    pub fn texture(
        &mut self,
        ctx: &egui::Context,
        document: &Arc<dyn Document>,
        id: &str,
    ) -> Option<egui::TextureHandle> {
        if let Some(entry) = self.textures.get_mut(id) {
            self.access_counter += 1;
            entry.access = self.access_counter;
            return Some(entry.texture.clone());
        }
        if self.pending.contains(id) || self.failed.contains(id) {
            return None;
        }
        self.pending.insert(id.to_string());
        let _ = self.req_tx.send(ImageRequest {
            generation: self.generation,
            doc: Arc::clone(document),
            id: id.to_string(),
            ctx: ctx.clone(),
        });
        None
    }

    pub fn image_size(&self, id: &str) -> Option<egui::Vec2> {
        self.textures.get(id).map(|entry| entry.texture.size_vec2())
    }
}
