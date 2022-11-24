use std::cmp::max;
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::time::Instant;
use cgmath::{Point2, Vector4};
use fontdue;
use image::Luma;
use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone)]
pub(crate) struct Grapheme {
    /// This can also be used as an unique ID in the font atlas image.
    pub(crate) text: String,
    /// Local rect w.r.t. baseline.
    pub(crate) layout: Vector4<i32>,
    /// Local bbox w.r.t. baseline.
    pub(crate) bounds: Vector4<f32>,
    /// Region in the font atlas.
    pub(crate) region: Vector4<u32>,
}

pub(crate) const FONT_ATLAS_SIZE: u32 = 2096;

pub(crate) struct DynamicFont {
    font: fontdue::Font,

    /// Font size.
    pub size: u32,

    /// Contains all the existing graphemes' bitmaps.
    pub(crate) atlas_image: image::GrayImage,

    /// Where should we put the nex grapheme in the atlas.
    next_grapheme_position: Point2<u32>,
    max_height_of_current_row: u32,

    grapheme_cache: HashMap<String, Grapheme>,
}

impl DynamicFont {
    pub(crate) fn load<P: AsRef<Path>>(path: P) -> Self {
        let now = Instant::now();

        // Read the font data.
        let mut f = File::open(path.as_ref()).expect("No font file found!");
        let metadata = fs::metadata(path.as_ref()).expect("Unable to read font file metadata!");
        let mut buffer = vec![0; metadata.len() as usize];
        f.read(&mut buffer).expect("Font buffer overflow!");

        let elapsed_time = now.elapsed();
        log::info!("Loading font file took {} milliseconds", elapsed_time.as_millis());

        // Parse it into the font type.
        let font = fontdue::Font::from_bytes(buffer, fontdue::FontSettings::default()).unwrap();

        let elapsed_time = now.elapsed();
        log::info!("Creating fontdue font took {} milliseconds", elapsed_time.as_millis());

        Self {
            font,
            size: 24,
            atlas_image: image::GrayImage::new(FONT_ATLAS_SIZE, FONT_ATLAS_SIZE),
            next_grapheme_position: Point2::new(0, 0),
            max_height_of_current_row: 0,
            grapheme_cache: HashMap::new(),
        }
    }

    pub(crate) fn get_graphemes(&mut self, text: String) -> Vec<Grapheme> {
        let mut graphemes = vec![];

        // for g in text.graphemes(true) {
        //     log::info!("Grapheme: {}", g);
        // }

        for c in text.chars() {
            let key = c.to_string();

            // Try find the grapheme in the cache.
            if let Some(g) = self.grapheme_cache.get(&key) {
                graphemes.push(g.clone());
                continue;
            }

            // Rasterize and get the layout metrics for the character.
            let (metrics, bitmap) = self.font.rasterize(c, self.size as f32);

            // log::info!("Character: {} {:?}", c, metrics);

            let buffer: &[u8] = &bitmap;

            // For debugging.
            // if metrics.width * metrics.height > 0 {
            //     image::save_buffer(&Path::new(&(format!("debug_output/{}.png", c.to_string()))),
            //                        buffer,
            //                        metrics.width as u32,
            //                        metrics.height as u32,
            //                        image::ColorType::L8).unwrap();
            // }

            // Add to the atlas.
            let region;
            {
                // Advance atlas row if necessary.
                if self.next_grapheme_position.x + metrics.width as u32 > FONT_ATLAS_SIZE {
                    self.next_grapheme_position.x = 0;
                    self.next_grapheme_position.y += self.max_height_of_current_row;
                    self.max_height_of_current_row = 0;
                }

                for col in 0..metrics.width {
                    for row in 0..metrics.height {
                        let x = self.next_grapheme_position.x + col as u32;
                        let y = self.next_grapheme_position.y + row as u32;

                        self.atlas_image.put_pixel(x,
                                                   y,
                                                   Luma([buffer[row * metrics.width + col]]));
                    }
                }

                region = Vector4::new(self.next_grapheme_position.x,
                                      self.next_grapheme_position.y,
                                      self.next_grapheme_position.x + metrics.width as u32,
                                      self.next_grapheme_position.y + metrics.height as u32);

                self.next_grapheme_position.x += metrics.width as u32;

                self.max_height_of_current_row = max(self.max_height_of_current_row, metrics.height as u32);
            }

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
                region,
            };

            self.grapheme_cache.insert(key, grapheme.clone());

            graphemes.push(grapheme);
        }

        // self.atlas_image.save("debug_output/font_atlas.png").expect("Failed to save font atlas as file!");

        graphemes
    }
}
