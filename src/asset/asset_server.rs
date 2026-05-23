use assets_manager::AssetCache;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender, channel};
use crate::scene::d3::{Model, RawModelData};
use crate::render::{RawTextureData, RawCubeTextureData, Texture};
use std::collections::HashMap;

pub enum AssetMessage {
    Model(PathBuf, RawModelData),
    Texture(PathBuf, RawTextureData),
    CubeTexture(PathBuf, RawCubeTextureData),
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
            loading_paths: HashMap::new(),
        }
    }

    pub fn request_load<P: AsRef<Path>>(&mut self, path: P) {
        let path_buf = path.as_ref().to_path_buf();
        if self.loading_paths.contains_key(&path_buf) || self.loaded_raw_models.contains_key(&path_buf) {
            return;
        }

        self.loading_paths.insert(path_buf.clone(), true);
        let tx = self.tx.clone();

        std::thread::spawn(move || {
            match Model::parse(&path_buf) {
                Ok(raw) => { let _ = tx.send(AssetMessage::Model(path_buf, raw)); }
                Err(e) => { log::error!("Failed to parse model: {}", e); }
            }
        });
    }

    pub fn request_texture<P: AsRef<Path>>(&mut self, path: P) {
        let path_buf = path.as_ref().to_path_buf();
        if self.loading_paths.contains_key(&path_buf) || self.loaded_raw_textures.contains_key(&path_buf) {
            return;
        }

        self.loading_paths.insert(path_buf.clone(), true);
        let tx = self.tx.clone();

        std::thread::spawn(move || {
            match Texture::decode_from_disk(&path_buf) {
                Ok(raw) => { let _ = tx.send(AssetMessage::Texture(path_buf, raw)); }
                Err(e) => { log::error!("Failed to decode texture: {}", e); }
            }
        });
    }

    pub fn request_cubemap<P: AsRef<Path>>(&mut self, path: P) {
        let path_buf = path.as_ref().to_path_buf();
        if self.loading_paths.contains_key(&path_buf) || self.loaded_raw_cubemaps.contains_key(&path_buf) {
            return;
        }

        self.loading_paths.insert(path_buf.clone(), true);
        let tx = self.tx.clone();

        std::thread::spawn(move || {
            match Texture::decode_cube_from_disk(&path_buf) {
                Ok(raw) => { let _ = tx.send(AssetMessage::CubeTexture(path_buf, raw)); }
                Err(e) => { log::error!("Failed to decode cubemap: {}", e); }
            }
        });
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
            }
        }
    }
}
