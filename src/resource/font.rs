use std::fs;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use cgmath::{Point2, Vector4};
use fontdue;
use unicode_segmentation::UnicodeSegmentation;

pub(crate) struct Grapheme {
    /// This can also be used as an unique ID in the font atlas image.
    text: String,
    /// Rect box w.r.t. baseline.
    layout: Vector4<i32>,
    /// Outlines' bbox.
    bounds: Vector4<f32>,
    /// Region in the font atlas.
    region: Vector4<u32>,
}

const FONT_ATLAS_SIZE: u32 = 2096;

pub(crate) struct DynamicFont {
    glyphs: Vec<Grapheme>,

    font: fontdue::Font,

    /// Font size.
    size: u32,

    /// Contains all the existing graphemes' bitmaps.
    atlas_image: image::GrayImage,
    /// Defines how big is the single grid in the atlas.
    max_grapheme_size: Point2<f32>,
}

impl DynamicFont {
    pub(crate) fn load<P: AsRef<Path>>(path: P) -> Self {
        // Read the font data.
        let mut f = File::open(path.as_ref()).expect("No font file found!");
        let metadata = fs::metadata(path.as_ref()).expect("Unable to read font file metadata!");
        let mut buffer = vec![0; metadata.len() as usize];
        f.read(&mut buffer).expect("Font buffer overflow!");

        // Parse it into the font type.
        let font = fontdue::Font::from_bytes(buffer, fontdue::FontSettings::default()).unwrap();

        Self {
            glyphs: vec![],
            font,
            size: 24,
            atlas_image: image::GrayImage::new(FONT_ATLAS_SIZE, FONT_ATLAS_SIZE),
            max_grapheme_size: Point2::new(0.0, 0.0),
        }
    }

    pub(crate) fn get_graphemes(&self, text: String) -> Vec<Grapheme> {
        let mut graphemes = vec![];

        for g in text.graphemes(true) {
            log::info!("Grapheme: {}", g);
        }

        for c in text.chars() {
            // Rasterize and get the layout metrics for the character.
            let (metrics, bitmap) = self.font.rasterize(c, self.size as f32);

            log::info!("Character: {} {:?}", c, metrics);

            let buffer: &[u8] = &bitmap;

            // For debugging.
            // if metrics.width * metrics.height > 0 {
            //     image::save_buffer(&Path::new(&(format!("debug_output/{}.png", c.to_string()))),
            //                        buffer,
            //                        metrics.width as u32,
            //                        metrics.height as u32,
            //                        image::ColorType::L8).unwrap();
            // }

            let grapheme = Grapheme {
                text: c.to_string(),
                layout: Vector4::new(metrics.xmin,
                                     metrics.ymin,
                                     metrics.xmin + metrics.width as i32,
                                     metrics.ymin + metrics.height as i32),
                bounds: Vector4::new(metrics.bounds.xmin,
                                     metrics.bounds.ymin,
                                     metrics.bounds.xmin + metrics.bounds.width,
                                     metrics.bounds.ymin + metrics.bounds.height),
                region: Vector4::new(0, 0, 0, 0),
            };

            graphemes.push(grapheme);
        }

        graphemes
    }
}
