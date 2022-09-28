use std::fs;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use cgmath::Vector4;
use fontdue;
use unicode_segmentation::UnicodeSegmentation;

pub(crate) struct Grapheme {
    text: String,
    bounds: Vector4<f32>,
    outline_bounds: Vector4<f32>,
    bitmap: Vec<u8>,
}

pub(crate) struct DynamicFont {
    glyphs: Vec<Grapheme>,
    font: fontdue::Font,
    size: u32,
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
        }
    }

    pub(crate) fn get_graphemes(&self, text: String) -> Vec<Grapheme> {
        let graphemes = vec![];

        for g in text.graphemes(true) {
            log::info!("Grapheme: {}", g);
        }

        for c in text.chars() {
            // Rasterize and get the layout metrics for the letter 'g' at 17px.
            let (metrics, bitmap) = self.font.rasterize(c, self.size as f32);

            log::info!("Character: {} {:?}", c, metrics);
            // let grapheme = {
            //     text: c
            // }
        }

        graphemes
    }
}
