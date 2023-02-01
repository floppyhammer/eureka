use std::collections::HashMap;
use std::path::PathBuf;

pub enum AssetType {
    Image,
    Font,
    Mesh,
}

pub trait AsAsset {
    /// The returned ID will be used as the hashing key.
    fn get_unique_id(&self) -> String;
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

    pub fn get_asset(&mut self, id: String) -> Option<&Box<dyn AsAsset>> {
        self.assets.get(&id)
    }
}
