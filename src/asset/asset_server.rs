use assets_manager::{loader, Asset, AssetCache, Compound, Handle};
use std::collections::HashMap;
use std::path::PathBuf;

pub struct AssetServer {
    pub asset_dir: PathBuf,
    pub asset_cache: AssetCache,
}

impl AssetServer {
    pub fn new() -> Self {
        // Get the asset directory.
        let asset_dir = std::path::Path::new(env!("OUT_DIR")).join("assets");
        log::info!("Asset dir: {}", asset_dir.display());

        // Create a new cache to load assets under the "./assets" folder.
        let cache = AssetCache::new("assets").unwrap();

        Self {
            asset_dir,
            asset_cache: cache,
        }
    }

    /// Monitor asset changes.
    pub fn update(&mut self) {
        self.asset_cache.hot_reload();
    }
}
