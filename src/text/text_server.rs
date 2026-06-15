use crate::asset::AssetServer;
use crate::math::rect_to_vec4;
use crate::math::transform::Transform2d;
use crate::render::atlas::{Atlas, AtlasInstance};
use crate::render::{RenderContext, TextureCache};
use crate::text::{DynamicFont, FONT_ATLAS_SIZE};
use glam::{Vec2, Vec4};
use std::collections::HashMap;
use unicode_linebreak::BreakClass;

pub struct TextServer {
    fonts: HashMap<String, DynamicFont>,
}

impl TextServer {
    pub(crate) fn new(asset_server: &mut AssetServer) -> Self {
        #[cfg(target_family = "windows")]
        let default_font_name = "arial";

        #[cfg(not(target_family = "windows"))]
        let default_font_name = "Droid Sans Fallback";

        asset_server.request_font(format!("system://{}", default_font_name));

        Self {
            fonts: HashMap::new(),
        }
    }

    /// Load a new font from disk asynchronously.
    pub fn load_font(&mut self, font_path: &String, asset_server: &mut AssetServer) {
        asset_server.request_font(font_path);
    }

    pub(crate) fn update(
        &mut self,
        render_server: &RenderContext,
        imported_texture_cache: &mut TextureCache,
        asset_server: &AssetServer,
    ) {
        for (path, buffer) in &asset_server.loaded_raw_fonts {
            let path_str = path.to_string_lossy().to_string();

            // Map system font back to "default" key.
            let key = if path_str.starts_with("system://") {
                "default".to_string()
            } else {
                path_str.clone()
            };

            if !self.fonts.contains_key(&key) {
                let font = DynamicFont::load_from_memory(
                    buffer.clone(),
                    render_server,
                    imported_texture_cache,
                );
                self.fonts.insert(key, font);
            }
        }
    }

    pub(crate) fn prepare(
        &mut self,
        render_server: &RenderContext,
        imported_texture_cache: &mut TextureCache,
    ) {
        for (_key, font) in &mut self.fonts {
            font.upload(render_server, imported_texture_cache);
        }
    }

    pub(crate) fn get_default_font(&self) -> Option<&DynamicFont> {
        self.fonts.get("default")
    }

    pub(crate) fn get_atlas(
        &mut self,
        text: &str,
        font_id: Option<String>,
        xform: Transform2d,
        leading: f32,
    ) -> Atlas {
        let font = if let Some(id) = font_id {
            self.fonts.get_mut(&id)
        } else {
            self.fonts.get_mut("default")
        };

        let font = match font {
            Some(f) => f,
            None => {
                return Atlas {
                    texture: None,
                    instances: vec![],
                    texture_size: (FONT_ATLAS_SIZE, FONT_ATLAS_SIZE),
                };
            }
        };

        let (glyphs, paras) = font.get_glyphs(text);

        let ascent = font.get_ascent();

        // Update atlas data.
        let mut instances = vec![];

        // Move origin from baseline to top-left.
        let origin = xform.position + Vec2::new(0.0, ascent);

        let mut layout_pos = Vec2::new(0.0, 0.0);

        for para in paras {
            for i in para {
                let g = &glyphs[i];

                // We only draw valid glyphs.
                if let Some(region) = g.region {
                    let instance = AtlasInstance {
                        position: Vec2::new(
                            layout_pos.x + g.offset.x as f32,
                            layout_pos.y + g.offset.y as f32,
                        ) + origin,
                        size: Vec2::new(g.bitmap_size.x as f32, g.bitmap_size.y as f32),
                        region: rect_to_vec4(region.to_f32()) / FONT_ATLAS_SIZE as f32,
                        color: Vec4::new(1.0, 1.0, 1.0, 1.0),
                    };
                    instances.push(instance);
                }

                if g.break_property == BreakClass::LineFeed {
                    continue;
                }

                // Update next glyph's position.
                layout_pos.x += g.x_adv as f32;
            }

            layout_pos.x = 0.0;
            layout_pos.y += font.size as f32 + leading;
        }

        Atlas {
            texture: Some(font.atlas_texture),
            instances,
            texture_size: (FONT_ATLAS_SIZE, FONT_ATLAS_SIZE),
        }
    }
}
