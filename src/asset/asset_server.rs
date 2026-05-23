use crate::asset::font_loader::find_system_font;
use crate::render::{RawCubeTextureData, RawTextureData, Texture};
use crate::scene::d3::{Model, RawModelData};
use assets_manager::AssetCache;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};

pub enum AssetMessage {
    Model(PathBuf, RawModelData),
    Texture(PathBuf, RawTextureData),
    CubeTexture(PathBuf, RawCubeTextureData),
    Font(PathBuf, Vec<u8>),
}

pub struct AssetServer {
    pub asset_dir: PathBuf,
    pub asset_cache: AssetCache,

    // Background loading
    tx: Sender<AssetMessage>,
    rx: Receiver<AssetMessage>,

    pub loaded_raw_models: HashMap<PathBuf, RawModelData>,
    pub loaded_raw_textures: HashMap<PathBuf, RawTextureData>,
    pub loaded_raw_cubemaps: HashMap<PathBuf, RawCubeTextureData>,
    pub loaded_raw_fonts: HashMap<PathBuf, Vec<u8>>,

    loading_paths: HashMap<PathBuf, bool>,
}

impl AssetServer {
    pub fn new() -> Self {
        let asset_dir = std::path::Path::new(env!("OUT_DIR")).join("assets");
        let cache = AssetCache::new("assets").unwrap();
        let (tx, rx) = channel();

        Self {
            asset_dir,
            asset_cache: cache,
            tx,
            rx,
            loaded_raw_models: HashMap::new(),
            loaded_raw_textures: HashMap::new(),
            loaded_raw_cubemaps: HashMap::new(),
            loaded_raw_fonts: HashMap::new(),
            loading_paths: HashMap::new(),
        }
    }

    pub fn request_load<P: AsRef<Path>>(&mut self, path: P) {
        let path_buf = path.as_ref().to_path_buf();
        if self.loading_paths.contains_key(&path_buf)
            || self.loaded_raw_models.contains_key(&path_buf)
        {
            return;
        }

        self.loading_paths.insert(path_buf.clone(), true);
        let tx = self.tx.clone();

        std::thread::spawn(move || match Model::parse(&path_buf) {
            Ok(raw) => {
                let _ = tx.send(AssetMessage::Model(path_buf, raw));
            }
            Err(e) => {
                log::error!("Failed to parse model: {}", e);
            }
        });
    }

    pub fn request_texture<P: AsRef<Path>>(&mut self, path: P) {
        let path_buf = path.as_ref().to_path_buf();
        if self.loading_paths.contains_key(&path_buf)
            || self.loaded_raw_textures.contains_key(&path_buf)
        {
            return;
        }

        self.loading_paths.insert(path_buf.clone(), true);
        let tx = self.tx.clone();

        std::thread::spawn(move || match Texture::decode_from_disk(&path_buf) {
            Ok(raw) => {
                let _ = tx.send(AssetMessage::Texture(path_buf, raw));
            }
            Err(e) => {
                log::error!("Failed to decode texture: {}", e);
            }
        });
    }

    pub fn request_cubemap<P: AsRef<Path>>(&mut self, path: P) {
        let path_buf = path.as_ref().to_path_buf();
        if self.loading_paths.contains_key(&path_buf)
            || self.loaded_raw_cubemaps.contains_key(&path_buf)
        {
            return;
        }

        self.loading_paths.insert(path_buf.clone(), true);
        let tx = self.tx.clone();

        std::thread::spawn(move || match Texture::decode_cube_from_disk(&path_buf) {
            Ok(raw) => {
                let _ = tx.send(AssetMessage::CubeTexture(path_buf, raw));
            }
            Err(e) => {
                log::error!("Failed to decode cubemap: {}", e);
            }
        });
    }

    pub fn request_font<P: AsRef<Path>>(&mut self, path: P) {
        let path_buf = path.as_ref().to_path_buf();
        if self.loading_paths.contains_key(&path_buf) || self.loaded_raw_fonts.contains_key(&path_buf) {
            return;
        }

        self.loading_paths.insert(path_buf.clone(), true);
        let tx = self.tx.clone();

        let path_str = path_buf.to_string_lossy().to_string();
        if path_str.starts_with("system://") {
            let font_name = path_str.strip_prefix("system://").unwrap().to_string();
            std::thread::spawn(move || {
                if let Some(buffer) = find_system_font(&font_name) {
                    let _ = tx.send(AssetMessage::Font(path_buf, buffer));
                } else {
                    log::error!("Failed to find system font: {}", font_name);
                }
            });
        } else {
            std::thread::spawn(move || match std::fs::read(&path_buf) {
                Ok(buffer) => {
                    let _ = tx.send(AssetMessage::Font(path_buf, buffer));
                }
                Err(e) => {
                    log::error!("Failed to read font file: {}", e);
                }
            });
        }
    }

    pub fn update(&mut self) {
        self.asset_cache.hot_reload();
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                AssetMessage::Model(path, raw) => {
                    self.loading_paths.remove(&path);
                    self.loaded_raw_models.insert(path, raw);
                }
                AssetMessage::Texture(path, raw) => {
                    self.loading_paths.remove(&path);
                    self.loaded_raw_textures.insert(path, raw);
                }
                AssetMessage::CubeTexture(path, raw) => {
                    self.loading_paths.remove(&path);
                    self.loaded_raw_cubemaps.insert(path, raw);
                }
                AssetMessage::Font(path, buffer) => {
                    self.loading_paths.remove(&path);
                    self.loaded_raw_fonts.insert(path, buffer);
                }
            }
        }
    }
}
