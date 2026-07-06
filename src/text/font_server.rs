use crate::asset::AssetServer;
use crate::math::rect_to_vec4;
use crate::math::transform::Transform2d;
use crate::render::atlas::{Atlas, AtlasInstance};
use crate::render::{RenderContext, TextureCache};
use crate::text::{DynamicFont, FONT_ATLAS_SIZE};
use glam::{Vec2, Vec4};
use std::collections::HashMap;
use unicode_linebreak::BreakClass;

pub struct FontServer {
    fonts: HashMap<String, DynamicFont>,
}

impl FontServer {
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
        asset_server: &mut AssetServer,
    ) {
        // 收集所有可能的键：已加载的和失败的（用于捕获虚拟 MSDF 路径）
        let mut keys_to_process = Vec::new();
        for k in asset_server.get_fonts().keys() {
            keys_to_process.push(k.clone());
        }

        let mut temp_buffer_cache = HashMap::new();

        for path in keys_to_process {
            let path_str = path.to_string_lossy().to_string();
            let key = if path_str.starts_with("system://") {
                "default".to_string()
            } else {
                path_str.clone()
            };

            if !self.fonts.contains_key(&key) {
                if let Some(buffer) = asset_server.take_font(&path) {
                    temp_buffer_cache.insert(path.clone(), buffer);
                }
            }
        }

        // 处理缓存中的 Buffer，并根据需要派生 MSDF 模式
        let cached_paths: Vec<_> = temp_buffer_cache.keys().cloned().collect();
        for actual_path in cached_paths {
            let buffer = temp_buffer_cache.get(&actual_path).unwrap();
            let path_str = actual_path.to_string_lossy().to_string();

            // 1. 创建原始模式（Raster）
            let raster_key = if path_str.starts_with("system://") { "default".to_string() } else { path_str.clone() };
            if !self.fonts.contains_key(&raster_key) {
                let font = crate::text::DynamicFont::load_from_memory(
                    buffer.clone(),
                    render_server,
                    imported_texture_cache,
                    crate::text::FontMode::Raster,
                );
                self.fonts.insert(raster_key, font);
            }

            // 2. 自动为每个加载的字体预备一个 MSDF 版本（虚拟键）
            // 这样无论 Label 请求的是 base.ttf 还是 base_msdf.ttf，都能找到
            let msdf_key = path_str.replace(".ttf", "_msdf.ttf");
            if !self.fonts.contains_key(&msdf_key) {
                log::info!("Auto-creating MSDF variant for font: {}", path_str);
                let font = crate::text::DynamicFont::load_from_memory(
                    buffer.clone(),
                    render_server,
                    imported_texture_cache,
                    crate::text::FontMode::Msdf,
                );
                self.fonts.insert(msdf_key, font);
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

        // Update atlas data in LOCAL SPACE.
        let mut instances = vec![];

        // Origin at top-left (0,0) of the label.
        // Move baseline down by ascent.
        let mut layout_pos = Vec2::new(0.0, ascent);

        for para in paras {
            for i in para {
                let g = &glyphs[i];

                if let Some(region) = g.region {
                    let instance = AtlasInstance {
                        position: Vec2::new(
                            layout_pos.x + g.offset.x as f32,
                            layout_pos.y + g.offset.y as f32,
                        ),
                        size: Vec2::new(g.bitmap_size.x as f32, g.bitmap_size.y as f32),
                        region: rect_to_vec4(region.to_f32()) / FONT_ATLAS_SIZE as f32,
                        color: Vec4::new(1.0, 1.0, 1.0, 1.0),
                    };
                    instances.push(instance);
                }

                if g.break_property == BreakClass::LineFeed {
                    continue;
                }
                layout_pos.x += g.x_adv as f32;
            }

            layout_pos.x = 0.0;
            layout_pos.y += (font.size as f32 + leading);
        }

        Atlas {
            texture: Some(font.atlas_texture),
            instances,
            texture_size: (FONT_ATLAS_SIZE, FONT_ATLAS_SIZE),
        }
    }
}
