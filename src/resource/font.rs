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
use unicode_bidi::{BidiClass, BidiInfo, Level};
use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone)]
pub(crate) struct Glyph {
    /// Unique ID specific to a font.
    ///
    /// Codepoint cannot be used as an unique ID in the font atlas image due to ligature.
    /// For example, ر in مر and ر in م ر obviously have different glyphs.
    pub(crate) index: u16,
    pub(crate) offset: Vector2<i32>,
    pub(crate) bitmap_size: Vector2<i32>,
    /// Local bbox w.r.t. baseline.
    pub(crate) bounds: Vector4<f32>,
    pub(crate) x_adv: i32,
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
    level: Level,
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

    pub(crate) fn get_ascent(&mut self) -> f32 {
        let metrics = self.font.horizontal_line_metrics(self.size as f32).unwrap();
        return metrics.ascent;
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

    /// Uses rustybuzz for shaping.
    pub(crate) fn get_glyphs(&mut self, text: &str) -> (Vec<Glyph>, Vec<Range<usize>>) {
        // Debug
        for g in text.graphemes(true) {
            log::info!("Grapheme: {}", g);
        }

        // Debug
        for c in text.chars() {
            log::info!("Character: {}", c);
        }

        let mut face = rustybuzz::Face::from_slice(&self.font_data, 0).unwrap();

        let units_per_em = face.units_per_em();

        let bidi_info = BidiInfo::new(text, None);

        let mut glyphs = vec![];
        let mut glyph_lines = vec![];

        for para in &bidi_info.paragraphs {
            let line = para.range.clone();
            println!("Line text: {}", &text[line.clone()]);

            // Reorder line text only when where's RTL text.
            if bidi_info.has_rtl() {
                let reordered_text = bidi_info.reorder_line(para, line.clone());
                println!("Reordered RTL line text: {}", reordered_text);
            }

            let (_, level_runs) = bidi_info.visual_runs(para, line.clone());

            // For line ranges.
            let glyph_count = glyphs.len();

            for run in level_runs.iter() {
                let mut run = run.clone();

                // Skip paragraph separator.
                if bidi_info.original_classes[run.end - 1] == BidiClass::B {
                    run = Range {
                        start: run.start,
                        end: run.end - 1,
                    };
                }

                let run_text = &text[run.clone()];
                println!("Run text: {}", run_text);

                // Glyphs in the current run.
                let mut run_glyphs = vec![];

                // Run language.
                let lang_info = whatlang::detect(run_text);

                // Decide run script.
                let script;
                if let Some(lang_info) = lang_info {
                    script = match lang_info.script() {
                        whatlang::Script::Arabic => rustybuzz::script::ARABIC,
                        whatlang::Script::Hebrew => rustybuzz::script::HEBREW,
                        whatlang::Script::Bengali => rustybuzz::script::BENGALI,
                        _ => rustybuzz::script::LATIN,
                    };
                } else {
                    script = rustybuzz::script::LATIN;
                }

                // Levels should be the same in a run.
                // This is not the case for classes though.
                let level = bidi_info.levels[run.start];

                let dir = if level.is_rtl() {
                    rustybuzz::Direction::RightToLeft
                } else {
                    rustybuzz::Direction::LeftToRight
                };

                let mut unicode_buffer = rustybuzz::UnicodeBuffer::new();
                unicode_buffer.push_str(run_text);

                unicode_buffer.set_direction(dir);
                unicode_buffer.set_script(script);

                let codepoint_count = unicode_buffer.len();

                // Do shaping.
                let glyph_buffer = rustybuzz::shape(&face, &[], unicode_buffer);

                let glyph_count = glyph_buffer.len();

                // Handle run glyphs.
                for (info, pos) in glyph_buffer
                    .glyph_infos()
                    .iter()
                    .zip(glyph_buffer.glyph_positions())
                {
                    // Get glyph index (specific to a font).
                    let index = info.glyph_id as u16;

                    // Try find the glyph in the cache.
                    if let Some(g) = self.glyph_cache.get(&index) {
                        run_glyphs.push(g.clone());
                        continue;
                    }

                    // Rasterize and get the layout metrics for the character.
                    let (metrics, bitmap) = self.font.rasterize_indexed(index, self.size as f32);

                    // For debugging.
                    // let buffer: &[u8] = &bitmap;
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
                                        img.put_pixel(
                                            x,
                                            y,
                                            Luma([bitmap[row * metrics.width + col]]),
                                        );
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
                        offset: Vector2::new(metrics.xmin, metrics.ymin),
                        bitmap_size: Vector2::new(metrics.width as i32, metrics.height as i32),
                        bounds: Vector4::new(
                            metrics.bounds.xmin,
                            metrics.bounds.ymin,
                            metrics.bounds.xmin + metrics.bounds.width,
                            metrics.bounds.ymin + metrics.bounds.height,
                        ),
                        x_adv: (pos.x_advance as f32 * self.size as f32 / units_per_em as f32)
                            .round() as i32,
                        region,
                    };

                    self.glyph_cache.insert(index, glyph.clone());
                    self.need_upload = true;

                    run_glyphs.push(glyph);
                }

                glyphs.append(&mut run_glyphs);
            }

            glyph_lines.push(Range {
                start: glyph_count,
                end: glyphs.len(),
            });
        }

        // self.atlas_image.save("debug_output/font_atlas.png").expect("Failed to save font atlas as file!");

        (glyphs, glyph_lines)
    }

    /// Uses allsorts for shaping.
    /// Returned glyphs are text context independent.
    /// Returned glyph lines are text context dependent.
    pub(crate) fn get_glyphs_v2(&mut self, text: &str) -> (Vec<Glyph>, Vec<Range<usize>>) {
        use allsorts::binary::read::ReadScope;
        use allsorts::font::{Font, MatchingPresentation};
        use allsorts::font_data::FontData;
        use allsorts::glyph_position::{GlyphLayout, TextDirection};
        use allsorts::gsub::{FeatureMask, Features};
        use allsorts::tag;

        let scope = ReadScope::new(&self.font_data);
        let font_file = scope.read::<FontData<'_>>().unwrap();
        let provider = font_file.table_provider(0).unwrap();
        let mut font = Font::new(Box::new(provider)).unwrap().unwrap();

        let head = font
            .head_table()
            .expect("Unable to parse head table.")
            .expect("Font lacks a head table.");

        let units_per_em = head.units_per_em;

        let bidi_info = BidiInfo::new(text, None);

        let mut glyphs = vec![];
        let mut glyph_lines = vec![];

        for para in &bidi_info.paragraphs {
            let line = para.range.clone();
            println!("Line text: {}", &text[line.clone()]);

            // Reorder line text only when where's RTL text.
            if bidi_info.has_rtl() {
                let reordered_text = bidi_info.reorder_line(para, line.clone());
                println!("Reordered RTL line text: {}", reordered_text);
            }

            let (_, level_runs) = bidi_info.visual_runs(para, line.clone());

            let glyph_count = glyphs.len();

            for run in level_runs.iter() {
                // We need to modify run range later.
                let mut run = run.clone();

                // Skip paragraph separator.
                if bidi_info.original_classes[run.end - 1] == BidiClass::B {
                    run.end -= 1;
                }

                let run_text = &text[run.clone()];
                println!("Run text: {}", run_text);

                let mut run_glyphs = vec![];

                let lang_info = whatlang::detect(run_text);

                let script;
                if let Some(lang_info) = lang_info {
                    script = match lang_info.script() {
                        whatlang::Script::Arabic => tag::ARAB,
                        whatlang::Script::Hebrew => tag::BASE,
                        whatlang::Script::Bengali => tag::BENG,
                        _ => tag::LATN,
                    };
                } else {
                    script = tag::LATN;
                }

                // Levels should be the same in a run.
                // This is not the case for classes though.
                let level = bidi_info.levels[run.start];

                let dir = if level.is_rtl() {
                    TextDirection::LeftToRight
                } else {
                    TextDirection::RightToLeft
                };

                let raw_glyphs =
                    font.map_glyphs(run_text, script, MatchingPresentation::NotRequired);

                let infos = font
                    .shape(
                        raw_glyphs,
                        script,
                        None,
                        &Features::Mask(FeatureMask::default()),
                        true,
                    )
                    .map_err(|(err, _infos)| err)
                    .unwrap();

                let mut layout = GlyphLayout::new(&mut font, &infos, dir, false);
                let positions = layout.glyph_positions().unwrap();

                for (info, position) in infos.iter().zip(&positions) {
                    // Get glyph index (specific to a font).
                    let index = info.glyph.glyph_index;

                    // Try find the glyph in the cache.
                    if let Some(g) = self.glyph_cache.get(&index) {
                        run_glyphs.push(g.clone());
                        continue;
                    }

                    // Rasterize and get the layout metrics for the character.
                    let (metrics, bitmap) = self.font.rasterize_indexed(index, self.size as f32);

                    let buffer: &[u8] = &bitmap;

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
                                        img.put_pixel(
                                            x,
                                            y,
                                            Luma([buffer[row * metrics.width + col]]),
                                        );
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
                        offset: Vector2::new(metrics.xmin, metrics.ymin),
                        bitmap_size: Vector2::new(metrics.width as i32, metrics.height as i32),
                        bounds: Vector4::new(
                            metrics.bounds.xmin,
                            metrics.bounds.ymin,
                            metrics.bounds.xmin + metrics.bounds.width,
                            metrics.bounds.ymin + metrics.bounds.height,
                        ),
                        x_adv: (position.hori_advance as f32 * self.size as f32
                            / units_per_em as f32)
                            .round() as i32,
                        region,
                    };

                    self.glyph_cache.insert(index, glyph.clone());
                    self.need_upload = true;

                    run_glyphs.push(glyph);
                }

                println!("Run glyph count: {}", run_glyphs.len());

                if level.is_rtl() {
                    for g in run_glyphs.iter().rev() {
                        glyphs.push(g.clone());
                    }
                } else {
                    glyphs.append(&mut run_glyphs);
                }
            }

            glyph_lines.push(Range {
                start: glyph_count,
                end: glyphs.len(),
            });
        }

        (glyphs, glyph_lines)
    }
}
