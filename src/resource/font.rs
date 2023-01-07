use crate::resource::{RenderServer, Texture};
use cgmath::{Point2, Vector2, Vector4};
use fontdue;
use image::{DynamicImage, Luma};
use std::cmp::max;
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::ops::Range;
use std::path::Path;
use std::str::FromStr;
use std::time::Instant;
use unicode_segmentation::UnicodeSegmentation;
use unicode_bidi::{BidiClass, BidiInfo};

#[derive(Clone)]
pub(crate) struct Glyph {
    /// Unique ID specific to a font.
    pub(crate) index: u16,
    /// This cannot be used as an unique ID in the font atlas image due to ligature.
    /// For example, ر in مر and ر in م ر obviously have different glyphs.
    pub(crate) text: String,
    /// Local rect w.r.t. baseline.
    pub(crate) layout: Vector4<i32>,
    /// Local bbox w.r.t. baseline.
    pub(crate) bounds: Vector4<f32>,
    pub(crate) x_adv: f32,
    /// Region in the font atlas.
    pub(crate) region: Vector4<u32>,
}

pub(crate) const FONT_ATLAS_SIZE: u32 = 2096;

pub(crate) struct DynamicFont {
    font: fontdue::Font,

    /// Font size in pixel.
    pub size: u32,

    /// Contains all cached glyphs' bitmaps.
    atlas_image: DynamicImage,

    /// GPU texture.
    atlas_texture: Texture,
    pub(crate) atlas_bind_group: wgpu::BindGroup,

    /// Atlas has been changed, the GPU texture needs to be updated.
    need_upload: bool,

    /// Where should we put the next glyph in the atlas.
    next_glyph_position: Point2<u32>,
    max_height_of_current_row: u32,

    glyph_cache: HashMap<u16, Glyph>,

    font_data: Vec<u8>,
}

struct TextRun {
    range: Range<usize>,
    class: BidiClass,
}

impl DynamicFont {
    pub(crate) fn load<P: AsRef<Path>>(path: P, render_server: &RenderServer) -> Self {
        let now = Instant::now();

        // Read the font data.
        let mut f = File::open(path.as_ref()).expect("No font file found!");
        let metadata = fs::metadata(path.as_ref()).expect("Unable to read font file metadata!");
        let mut buffer = vec![0; metadata.len() as usize];
        f.read(&mut buffer).expect("Font buffer overflow!");

        let font_data = buffer.clone();

        let elapsed_time = now.elapsed();
        log::info!(
            "Loading font file took {} milliseconds",
            elapsed_time.as_millis()
        );

        // Parse it into the font type.
        let font = fontdue::Font::from_bytes(buffer, fontdue::FontSettings::default()).unwrap();

        let elapsed_time = now.elapsed();
        log::info!(
            "Creating fontdue font took {} milliseconds",
            elapsed_time.as_millis()
        );

        let atlas_image =
            DynamicImage::ImageLuma8(image::GrayImage::new(FONT_ATLAS_SIZE, FONT_ATLAS_SIZE));

        let atlas_texture = Texture::from_image(
            &render_server.device,
            &render_server.queue,
            &atlas_image,
            "default font atlas".into(),
        )
            .unwrap();

        let atlas_bind_group = render_server.create_sprite2d_bind_group(&atlas_texture);

        Self {
            font,
            size: 32,
            atlas_image,
            atlas_texture,
            atlas_bind_group,
            need_upload: false,
            next_glyph_position: Point2::new(0, 0),
            max_height_of_current_row: 0,
            glyph_cache: HashMap::new(),
            font_data,
        }
    }

    /// Upload atlas data to the atlas texture.
    pub(crate) fn upload(&mut self, render_server: &RenderServer) {
        if self.need_upload {
            self.need_upload = false;

            let queue = &render_server.queue;

            // TODO: do not copy the whole atlas but only the changed portion.
            let img_copy_texture = wgpu::ImageCopyTexture {
                aspect: wgpu::TextureAspect::All,
                texture: &self.atlas_texture.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            };

            let size = wgpu::Extent3d {
                width: FONT_ATLAS_SIZE,
                height: FONT_ATLAS_SIZE,
                depth_or_array_layers: 1,
            };

            match &self.atlas_image {
                DynamicImage::ImageLuma8(gray) => {
                    queue.write_texture(
                        img_copy_texture,
                        &gray,
                        wgpu::ImageDataLayout {
                            offset: 0,
                            bytes_per_row: std::num::NonZeroU32::new(FONT_ATLAS_SIZE),
                            rows_per_image: std::num::NonZeroU32::new(FONT_ATLAS_SIZE),
                        },
                        size,
                    );
                }
                _ => {}
            }
        }
    }

    pub(crate) fn get_glyphs(&mut self, text: &str) -> Vec<Glyph> {
        // // Debug
        // for g in text.graphemes(true) {
        //     log::info!("Grapheme: {}", g);
        // }
        //
        // // Debug
        // for c in text.chars() {
        //     log::info!("Character: {}", c);
        // }

        let mut face = rustybuzz::Face::from_slice(&self.font_data, 0).unwrap();

        let bidi_info = BidiInfo::new(text, None);

        // for para in &bidi_info.paragraphs {
        //     let line = para.range.clone();
        //     let reordered_text = bidi_info.reorder_line(para, line);
        // }

        let mut runs = vec!();
        let mut last_char_class = None;
        let mut char_index = 0;
        let mut run_start_index = 0;

        for class in &bidi_info.original_classes {
            if let Some(c) = last_char_class {
                if c != *class {
                    runs.push(TextRun {
                        range: Range { start: run_start_index, end: char_index },
                        class: c,
                    });
                    run_start_index = char_index;
                }
            } else {
                last_char_class = Some(class.clone());
            }

            last_char_class = Some(class.clone());
            char_index += 1;
        }

        // Last run.
        if let Some(c) = last_char_class {
            runs.push(TextRun {
                range: Range { start: run_start_index, end: char_index },
                class: c,
            });
        }

        let mut glyphs = vec![];

        for run in runs {
            let mut buffer = rustybuzz::UnicodeBuffer::new();
            buffer.push_str(&text[run.range]);

            match run.class {
                BidiClass::AL => { // Right-to-Left Arabic
                    // FIXME: no effect for unifont (other fonts are good).
                    // But the same snippet works in C++ for unifont.
                    buffer.set_direction(rustybuzz::Direction::RightToLeft);
                    buffer.set_language(rustybuzz::Language::from_str("ar").unwrap());
                    buffer.set_script(rustybuzz::script::ARABIC);
                }
                BidiClass::AN => { // Arabic Number
                    buffer.set_direction(rustybuzz::Direction::LeftToRight);
                    buffer.set_language(rustybuzz::Language::from_str("ar").unwrap());
                    buffer.set_script(rustybuzz::script::ARABIC);
                }
                _ => {
                    buffer.set_direction(rustybuzz::Direction::LeftToRight);
                    buffer.set_language(rustybuzz::Language::from_str("en").unwrap());
                    buffer.set_script(rustybuzz::script::LATIN);
                }
            }

            let codepoint_count = buffer.len();

            let glyph_buffer = rustybuzz::shape(&face, &[], buffer);

            let glyph_count = glyph_buffer.len();

            for i in 0..glyph_buffer.glyph_infos().len() {
                let info = &glyph_buffer.glyph_infos()[i];
                let pos = &glyph_buffer.glyph_positions()[i];

                // Get glyph index (specific to a font).
                let index = info.glyph_id as u16;
                // log::info!("Glyph index: {}", index);

                // Try find the glyph in the cache.
                if let Some(g) = self.glyph_cache.get(&index) {
                    glyphs.push(g.clone());
                    continue;
                }

                // Rasterize and get the layout metrics for the character.
                let (metrics, bitmap) = self.font.rasterize_indexed(index, self.size as f32);

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
                    if self.next_glyph_position.x + metrics.width as u32 > FONT_ATLAS_SIZE {
                        self.next_glyph_position.x = 0;
                        self.next_glyph_position.y += self.max_height_of_current_row;
                        self.max_height_of_current_row = 0;
                    }

                    for col in 0..metrics.width {
                        for row in 0..metrics.height {
                            let x = self.next_glyph_position.x + col as u32;
                            let y = self.next_glyph_position.y + row as u32;

                            match &mut self.atlas_image {
                                DynamicImage::ImageLuma8(img) => {
                                    img.put_pixel(x, y, Luma([buffer[row * metrics.width + col]]));
                                }
                                _ => {
                                    panic!()
                                }
                            }
                        }
                    }

                    region = Vector4::new(
                        self.next_glyph_position.x,
                        self.next_glyph_position.y,
                        self.next_glyph_position.x + metrics.width as u32,
                        self.next_glyph_position.y + metrics.height as u32,
                    );

                    self.next_glyph_position.x += metrics.width as u32;

                    self.max_height_of_current_row =
                        max(self.max_height_of_current_row, metrics.height as u32);
                }

                let glyph = Glyph {
                    index,
                    text: "".to_string(), // TODO
                    layout: Vector4::new(
                        metrics.xmin,
                        metrics.ymin,
                        metrics.xmin + metrics.width as i32,
                        metrics.ymin + metrics.height as i32,
                    ),
                    bounds: Vector4::new(
                        metrics.bounds.xmin,
                        metrics.bounds.ymin,
                        metrics.bounds.xmin + metrics.bounds.width,
                        metrics.bounds.ymin + metrics.bounds.height,
                    ),
                    x_adv: (pos.x_advance as f32 / self.size as f32).round(),
                    region,
                };

                self.glyph_cache.insert(index, glyph.clone());
                self.need_upload = true;

                glyphs.push(glyph);
            }
        }

        // self.atlas_image.save("debug_output/font_atlas.png").expect("Failed to save font atlas as file!");

        glyphs
    }
}
