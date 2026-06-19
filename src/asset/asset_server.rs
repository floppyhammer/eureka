use crate::asset::font_loader::find_system_font;
use crate::render::{RawCubeTextureData, RawTextureData, Texture};
use crate::scene::d3::{Model, RawModelData};
use rayon::{ThreadPool, ThreadPoolBuilder};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};

pub enum AssetMessage {
    Model(PathBuf, RawModelData),
    Texture(PathBuf, RawTextureData),
    CubeTexture(PathBuf, RawCubeTextureData),
    Font(PathBuf, Vec<u8>),
    Error(PathBuf, String),
}

pub struct AssetServer {
    pub asset_dir: PathBuf,
    pool: ThreadPool,

    // Background loading
    tx: Sender<AssetMessage>,
    rx: Receiver<AssetMessage>,

    loaded_raw_models: HashMap<PathBuf, RawModelData>,
    loaded_raw_textures: HashMap<PathBuf, RawTextureData>,
    loaded_raw_cubemaps: HashMap<PathBuf, RawCubeTextureData>,
    loaded_raw_fonts: HashMap<PathBuf, Vec<u8>>,

    loading_paths: HashMap<PathBuf, bool>,
    failed_paths: HashMap<PathBuf, String>,
}

impl AssetServer {
    pub fn new() -> Self {
        let asset_dir = Path::new(env!("OUT_DIR")).join("assets");
        let (tx, rx) = channel();

        let pool = ThreadPoolBuilder::new()
            .thread_name(|i| format!("AssetLoader-{}", i))
            .build()
            .expect("Failed to create asset thread pool");

        Self {
            asset_dir,
            pool,
            tx,
            rx,
            loaded_raw_models: HashMap::new(),
            loaded_raw_textures: HashMap::new(),
            loaded_raw_cubemaps: HashMap::new(),
            loaded_raw_fonts: HashMap::new(),
            loading_paths: HashMap::new(),
            failed_paths: HashMap::new(),
        }
    }

    pub fn is_loading<P: AsRef<Path>>(&self, path: P) -> bool {
        self.loading_paths.contains_key(path.as_ref())
    }

    pub fn has_failed<P: AsRef<Path>>(&self, path: P) -> Option<&String> {
        self.failed_paths.get(path.as_ref())
    }

    pub fn take_model<P: AsRef<Path>>(&mut self, path: P) -> Option<RawModelData> {
        self.loaded_raw_models.remove(path.as_ref())
    }

    pub fn take_texture<P: AsRef<Path>>(&mut self, path: P) -> Option<RawTextureData> {
        self.loaded_raw_textures.remove(path.as_ref())
    }

    pub fn take_cubemap<P: AsRef<Path>>(&mut self, path: P) -> Option<RawCubeTextureData> {
        self.loaded_raw_cubemaps.remove(path.as_ref())
    }

    pub fn take_font<P: AsRef<Path>>(&mut self, path: P) -> Option<Vec<u8>> {
        self.loaded_raw_fonts.remove(path.as_ref())
    }

    pub fn get_fonts(&self) -> &HashMap<PathBuf, Vec<u8>> {
        &self.loaded_raw_fonts
    }

    pub fn request_load<P: AsRef<Path>>(&mut self, path: P) {
        let path_buf = path.as_ref().to_path_buf();
        if self.loading_paths.contains_key(&path_buf)
            || self.loaded_raw_models.contains_key(&path_buf)
            || self.failed_paths.contains_key(&path_buf)
        {
            return;
        }

        self.loading_paths.insert(path_buf.clone(), true);
        let tx = self.tx.clone();

        self.pool.spawn(move || match Model::parse(&path_buf) {
            Ok(raw) => {
                let _ = tx.send(AssetMessage::Model(path_buf, raw));
            }
            Err(e) => {
                let err_msg = format!("Failed to parse model: {}", e);
                log::error!("{}", err_msg);
                let _ = tx.send(AssetMessage::Error(path_buf, err_msg));
            }
        });
    }

    pub fn request_texture<P: AsRef<Path>>(&mut self, path: P) {
        let path_buf = path.as_ref().to_path_buf();
        if self.loading_paths.contains_key(&path_buf)
            || self.loaded_raw_textures.contains_key(&path_buf)
            || self.failed_paths.contains_key(&path_buf)
        {
            return;
        }

        self.loading_paths.insert(path_buf.clone(), true);
        let tx = self.tx.clone();
        self.pool.spawn(move || match Texture::decode_from_disk(&path_buf) {
            Ok(raw) => {
                let _ = tx.send(AssetMessage::Texture(path_buf, raw));
            }
            Err(e) => {
                let err_msg = format!("Failed to decode texture: {}", e);
                log::error!("{}", err_msg);
                let _ = tx.send(AssetMessage::Error(path_buf, err_msg));
            }
        });
    }

    pub fn request_cubemap<P: AsRef<Path>>(&mut self, path: P) {
        let path_buf = path.as_ref().to_path_buf();
        if self.loading_paths.contains_key(&path_buf)
            || self.loaded_raw_cubemaps.contains_key(&path_buf)
            || self.failed_paths.contains_key(&path_buf)
        {
            return;
        }

        self.loading_paths.insert(path_buf.clone(), true);
        let tx = self.tx.clone();

        self.pool.spawn(move || match Texture::decode_cube_from_disk(&path_buf) {
            Ok(raw) => {
                let _ = tx.send(AssetMessage::CubeTexture(path_buf, raw));
            }
            Err(e) => {
                let err_msg = format!("Failed to decode cubemap: {}", e);
                log::error!("{}", err_msg);
                let _ = tx.send(AssetMessage::Error(path_buf, err_msg));
            }
        });
    }

    pub fn request_font<P: AsRef<Path>>(&mut self, path: P) {
        let path_buf = path.as_ref().to_path_buf();
        if self.loading_paths.contains_key(&path_buf)
            || self.loaded_raw_fonts.contains_key(&path_buf)
            || self.failed_paths.contains_key(&path_buf)
        {
            return;
        }

        self.loading_paths.insert(path_buf.clone(), true);
        let tx = self.tx.clone();

        let path_str = path_buf.to_string_lossy().to_string();
        if path_str.starts_with("system://") {
            let font_name = path_str.strip_prefix("system://").unwrap().to_string();
            self.pool.spawn(move || {
                if let Some(buffer) = find_system_font(&font_name) {
                    let _ = tx.send(AssetMessage::Font(path_buf, buffer));
                } else {
                    let err_msg = format!("Failed to find system font: {}", font_name);
                    log::error!("{}", err_msg);
                    let _ = tx.send(AssetMessage::Error(path_buf, err_msg));
                }
            });
        } else {
            self.pool.spawn(move || match std::fs::read(&path_buf) {
                Ok(buffer) => {
                    let _ = tx.send(AssetMessage::Font(path_buf, buffer));
                }
                Err(e) => {
                    let err_msg = format!("Failed to read font file: {}", e);
                    log::error!("{}", err_msg);
                    let _ = tx.send(AssetMessage::Error(path_buf, err_msg));
                }
            });
        }
    }

    pub fn update(&mut self) {
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
                AssetMessage::Error(path, err) => {
                    self.loading_paths.remove(&path);
                    self.failed_paths.insert(path, err);
                }
            }
        }
    }
}
