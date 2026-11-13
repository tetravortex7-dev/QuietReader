use super::auth::OpdsAuth;
use std::collections::{HashMap, HashSet, VecDeque};
use std::io::Read;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread;
use std::time::Duration;

const MAX_COVER_BYTES: u64 = 5 * 1024 * 1024;
const MAX_COVER_DIMENSION: u32 = 2048;
const MAX_COVER_ALLOC: u64 = 32 * 1024 * 1024;
const MAX_CONCURRENT_COVER_FETCHES: usize = 4;
pub(crate) const DEFAULT_MAX_COVER_CACHE_ENTRIES: usize = 128;
pub(crate) const DEFAULT_MAX_COVER_CACHE_BYTES: usize = 64 * 1024 * 1024;

#[derive(Clone, Debug)]
pub struct CoverImage {
    pub width: usize,
    pub height: usize,
    pub rgba: Vec<u8>,
}

pub(crate) struct BoundedLruMap<V> {
    entries: HashMap<String, (V, usize)>,
    order: VecDeque<String>,
    total_bytes: usize,
    max_entries: usize,
    max_bytes: usize,
}

impl<V> BoundedLruMap<V> {
    pub(crate) fn new(max_entries: usize, max_bytes: usize) -> Self {
        Self {
            entries: HashMap::new(),
            order: VecDeque::new(),
            total_bytes: 0,
            max_entries,
            max_bytes,
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }

    pub(crate) fn total_bytes(&self) -> usize {
        self.total_bytes
    }

    pub(crate) fn clear(&mut self) {
        self.entries.clear();
        self.order.clear();
        self.total_bytes = 0;
    }

    fn touch(&mut self, key: &str) {
        self.order.retain(|k| k != key);
        self.order.push_back(key.to_string());
    }

    pub(crate) fn get_cloned(&mut self, key: &str) -> Option<V>
    where
        V: Clone,
    {
        let value = self.entries.get(key).map(|(v, _)| v.clone())?;
        self.touch(key);
        Some(value)
    }

    pub(crate) fn insert(&mut self, key: String, value: V, byte_size: usize) -> bool {
        if let Some((_, old_bytes)) = self.entries.remove(&key) {
            self.total_bytes = self.total_bytes.saturating_sub(old_bytes);
            self.order.retain(|k| k != &key);
        }

        if self.max_entries == 0 || (self.max_bytes > 0 && byte_size > self.max_bytes) {
            return false;
        }

        while self.entries.len() >= self.max_entries
            || (self.max_bytes > 0
                && self.total_bytes.saturating_add(byte_size) > self.max_bytes
                && !self.entries.is_empty())
        {
            let Some(oldest) = self.order.pop_front() else {
                break;
            };
            if let Some((_, evicted_bytes)) = self.entries.remove(&oldest) {
                self.total_bytes = self.total_bytes.saturating_sub(evicted_bytes);
            }
        }

        if self.entries.len() >= self.max_entries
            || (self.max_bytes > 0 && self.total_bytes.saturating_add(byte_size) > self.max_bytes)
        {
            return false;
        }

        self.total_bytes = self.total_bytes.saturating_add(byte_size);
        self.order.push_back(key.clone());
        self.entries.insert(key, (value, byte_size));
        true
    }
}

pub struct OpdsCoverManager {
    cache: BoundedLruMap<Option<CoverImage>>,
    in_flight: HashSet<String>,
    generation: u64,
    tx_image: Sender<(u64, String, Option<CoverImage>)>,
    rx_image: Receiver<(u64, String, Option<CoverImage>)>,
}

impl Default for OpdsCoverManager {
    fn default() -> Self {
        Self::new()
    }
}

impl OpdsCoverManager {
    pub fn new() -> Self {
        Self::with_limits(
            DEFAULT_MAX_COVER_CACHE_ENTRIES,
            DEFAULT_MAX_COVER_CACHE_BYTES,
        )
    }

    pub fn with_limits(max_entries: usize, max_bytes: usize) -> Self {
        let (tx_image, rx_image) = channel();
        Self {
            cache: BoundedLruMap::new(max_entries, max_bytes),
            in_flight: HashSet::new(),
            generation: 0,
            tx_image,
            rx_image,
        }
    }

    pub fn cache_len(&self) -> usize {
        self.cache.len()
    }

    pub fn cache_bytes(&self) -> usize {
        self.cache.total_bytes()
    }

    pub fn clear(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.cache.clear();
        self.in_flight.clear();
    }

    pub(crate) fn insert_cached(&mut self, url: String, img: Option<CoverImage>) {
        let bytes = img.as_ref().map(|c| c.rgba.len()).unwrap_or(0);
        self.cache.insert(url, img, bytes);
    }

    pub fn poll(&mut self) -> bool {
        let mut got_new = false;
        while let Ok((req_gen, url, opt_img)) = self.rx_image.try_recv() {
            if req_gen != self.generation {
                continue;
            }
            self.in_flight.remove(&url);
            if opt_img.is_some() {
                got_new = true;
            }
            self.insert_cached(url, opt_img);
        }
        got_new
    }

    pub fn get_cover(&mut self, url: &str, auth: Option<OpdsAuth>) -> Option<CoverImage> {
        self.poll();
        if let Some(cached) = self.cache.get_cloned(url) {
            return cached;
        }

        if self.in_flight.len() >= MAX_CONCURRENT_COVER_FETCHES {
            return None;
        }

        if !self.in_flight.contains(url) {
            self.in_flight.insert(url.to_string());
            let tx = self.tx_image.clone();
            let cover_url = url.to_string();
            let req_gen = self.generation;
            let spawn_result = thread::Builder::new()
                .name("opds-cover-fetch".to_string())
                .spawn(move || {
                    let agent = crate::net::agent(Duration::from_secs(6));

                    let ua = crate::net::user_agent(Some("OPDS Cover"));
                    let mut req = agent
                        .get(&cover_url)
                        .header("User-Agent", &ua)
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
                        Some(CoverImage {
                            width: w as usize,
                            height: h as usize,
                            rgba: rgba.into_raw(),
                        })
                    });

                    let _ = tx.send((req_gen, cover_url, res));
                });

            if spawn_result.is_err() {
                self.in_flight.remove(url);
            }
        }

        None
    }
}
