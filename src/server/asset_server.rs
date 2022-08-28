use std::collections::HashMap;

pub trait AsAsset {
    /// The returned unique ID will be used as hashmap key.
    fn get_unique_id(&self);
}

struct AssetServer {
    assets: HashMap<String, Box<dyn AsAsset>>,
}

impl AssetServer {
    fn new() -> Self {
        // Type inference lets us omit an explicit type signature (which
        // would be `HashMap<String, Box<dyn AsAsset>` in this example).
        let assets = HashMap::new();

        Self {
            assets,
        }
    }
}
