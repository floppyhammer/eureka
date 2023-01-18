use std::collections::HashMap;
use std::path::PathBuf;

pub trait AsAsset {
    /// The returned unique ID will be used as hashmap key.
    fn get_unique_id(&self);
}

pub struct AssetServer {
    assets: HashMap<String, Box<dyn AsAsset>>,
    pub asset_dir: PathBuf,
}

impl AssetServer {
    pub fn new() -> Self {
        // Type inference lets us omit an explicit type signature (which
        // would be `HashMap<String, Box<dyn AsAsset>` in this example).
        let assets = HashMap::new();

        // Get the asset directory.
        let asset_dir = std::path::Path::new(env!("OUT_DIR")).join("assets");
        log::info!("Asset dir: {}", asset_dir.display());

        Self { assets, asset_dir }
    }
}
