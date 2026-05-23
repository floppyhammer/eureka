use assets_manager::AssetCache;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender, channel};
use crate::scene::d3::{Model, RawModelData};
use std::collections::HashMap;

pub struct AssetServer {
    pub asset_dir: PathBuf,
    pub asset_cache: AssetCache,

    // Background loading
    tx: Sender<(PathBuf, RawModelData)>,
    rx: Receiver<(PathBuf, RawModelData)>,
    pub loaded_raw_models: HashMap<PathBuf, RawModelData>,
}

impl AssetServer {
    pub fn new() -> Self {
        // Get the asset directory.
        let asset_dir = std::path::Path::new(env!("OUT_DIR")).join("assets");
        log::info!("Asset dir: {}", asset_dir.display());

        // Create a new cache to load assets under the "./assets" folder.
        let cache = AssetCache::new("assets").unwrap();

        let (tx, rx) = channel();

        Self {
            asset_dir,
            asset_cache: cache,
            tx,
            rx,
            loaded_raw_models: HashMap::new(),
        }
    }

    /// Request a model to be loaded in the background.
    pub fn request_model<P: AsRef<Path>>(&self, path: P) {
        let path_buf = path.as_ref().to_path_buf();
        let tx = self.tx.clone();

        std::thread::spawn(move || {
            log::info!("Starting background load: {:?}", path_buf);
            match Model::parse(&path_buf) {
                Ok(raw) => {
                    let _ = tx.send((path_buf, raw));
                }
                Err(e) => {
                    log::error!("Failed to parse model {:?}: {}", path_buf, e);
                }
            }
        });
    }

    /// Monitor asset changes and collect background loads.
    pub fn update(&mut self) {
        self.asset_cache.hot_reload();

        // Collect all finished background loads.
        while let Ok((path, raw)) = self.rx.try_recv() {
            log::info!("Background load finished: {:?}", path);
            self.loaded_raw_models.insert(path, raw);
        }
    }
}
