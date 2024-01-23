use std::borrow::Cow;
use assets_manager::{loader::Loader, Asset, BoxedError};
use assets_manager::loader::ImageLoader;
use image::DynamicImage;

pub struct Image(pub DynamicImage);

impl Asset for Image {
    const EXTENSIONS: &'static [&'static str] = &["png", "jpg"];
    type Loader = ImageLoader;
    const HOT_RELOADED: bool = true;
}

impl Loader<Image> for ImageLoader {
    fn load(content: Cow<[u8]>, ext: &str) -> Result<Image, BoxedError> {
        let dynamic_image = ImageLoader::load(content, ext)?;

        Ok(Image(dynamic_image))
    }
}
