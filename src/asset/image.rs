use assets_manager::loader::ImageLoader;
use assets_manager::{loader::Loader, Asset, BoxedError};
use image::DynamicImage;
use std::borrow::Cow;

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
