use assets_manager::{loader, Asset, AssetCache, AssetGuard, Compound, Handle};
use std::collections::HashMap;
use std::path::PathBuf;

pub struct AssetServer {
    pub asset_dir: PathBuf,
    asset_cache: AssetCache,
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

    pub fn load<A: Compound>(&mut self, id: &str) -> Option<Handle<A>> {
        // Get a handle on the asset.
        let handle = self.asset_cache.load::<A>(id).ok()?;

        // // Lock the asset for reading.
        // // Any number of read locks can exist at the same time,
        // // but none can exist when the asset is reloaded.
        // let asset = handle.read();
        //
        // han

        Some(handle)
    }

    /// Monitor asset changes.
    pub fn update(&mut self) {
        self.asset_cache.hot_reload();
    }
}
