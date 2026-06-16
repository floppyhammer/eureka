use crate::render::camera::CameraUniform;
use crate::render::create_render_pipeline;
use crate::render::render_backend::PreparedFrame;
use crate::render::render_graph::{standard_resources, FrameContext, Node, PooledBuffer};
use crate::render::sprite::ExtractedSprite2d;
use crate::render::vertex::{Vertex2d, VertexBuffer};
use crate::render::Texture;
use glam::Vec2;
use std::any::Any;
use std::ops::Range;

pub struct SpriteNode {
    pipeline: Option<wgpu::RenderPipeline>,
}

impl Default for SpriteNode {
    fn default() -> Self {
        Self { pipeline: None }
    }
}

impl Node for SpriteNode {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn node_resources(
        &self,
        prepared: &PreparedFrame,
    ) -> crate::render::render_graph::resource::NodeResources {
        use crate::render::render_graph::resource::{ResourceSpec, TextureKey};
        use crate::render::render_graph::standard_resources;
        use crate::render::Texture;

        let camera_buffer_size = CameraUniform::get_uniform_offset_unit()
            * crate::render::render_graph::nodes::prepare_view::MAX_CAMERAS;

        let material_buffer_size = prepared.material_uniforms.len()
            * size_of::<crate::render::material::MaterialUniform>();

        let total_quads = prepared.extracted.sprites.len();

        crate::render::render_graph::resource::NodeResources::new()
            .input(
                standard_resources::camera_buffer(),
                ResourceSpec::buffer(camera_buffer_size as u64, wgpu::BufferUsages::UNIFORM),
            )
            .input(
                standard_resources::material_storage_buffer(),
                ResourceSpec::buffer(
                    material_buffer_size as u64,
                    wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                ),
            )
            .output(
                standard_resources::main_depth(),
                ResourceSpec::Texture(TextureKey {
                    width: 0,
                    height: 0,
                    format: Some(Texture::DEPTH_FORMAT),
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING,
                    layers: 1,
                    dimension: wgpu::TextureDimension::D2,
                }),
            )
            .output(
                standard_resources::final_output(),
                ResourceSpec::Texture(TextureKey {
                    width: 0,
                    height: 0,
                    format: None,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    layers: 1,
                    dimension: wgpu::TextureDimension::D2,
                }),
            )
            .internal(
                standard_resources::sprite_vertex_buffer(),
                ResourceSpec::buffer(
                    (size_of::<Vertex2d>() * 4 * total_quads) as u64,
                    wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                ),
            )
            .internal(
                standard_resources::sprite_index_buffer(),
                ResourceSpec::buffer(
                    (size_of::<u32>() * 6 * total_quads) as u64,
                    wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                ),
            )
    }

    fn run(&mut self, context: &mut FrameContext) {
        let batches = create_batches(
            &context.extracted.sprites,
            context,
            &context.extracted.cameras,
        );

        if batches.is_empty() {
            return;
        }

        if self.pipeline.is_none() {
            let device = &context.render_context.device;

            let camera_bind_group_layout = context
                .backend
                .get_bind_group_layout("camera_bind_group_layout")
                .unwrap()
                .clone();

            let bindless_bind_group_layout = context
                .backend
                .get_bind_group_layout("bindless_bind_group_layout")
                .unwrap()
                .clone();

            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("sprite bindless pipeline layout"),
                bind_group_layouts: &[&camera_bind_group_layout, &bindless_bind_group_layout],
                push_constant_ranges: &[],
            });

            let shader = wgpu::ShaderModuleDescriptor {
                label: Some("sprite shader"),
                source: wgpu::ShaderSource::Wgsl(
                    include_str!("../../../shaders/sprite.wgsl").into(),
                ),
            };

            self.pipeline = Some(create_render_pipeline(
                device,
                &pipeline_layout,
                Some(context.render_context.surface_config.format),
                Some(Texture::DEPTH_FORMAT),
                &[Vertex2d::desc()],
                shader,
                "sprite bindless",
                true,
                None,
            ));
        }

        let main_depth = context.texture(&standard_resources::main_depth());
        let camera_buffer = context.buffer(&standard_resources::camera_buffer());

        let camera_bind_group = context
            .get_bind_group("camera_bind_group_layout", vec![camera_buffer.id])
            .clone();

        let materials_storage_buffer =
            context.buffer(&standard_resources::material_storage_buffer());

        let bindless_bind_group = context
            .get_bind_group(
                "bindless_bind_group_layout",
                vec![materials_storage_buffer.id],
            )
            .clone();

        let vertex_buffer = context.buffer(&standard_resources::sprite_vertex_buffer());
        let index_buffer = context.buffer(&standard_resources::sprite_index_buffer());

        let mut render_pass = context
            .encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("sprite render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: context.final_output_view, // 直接绘制到 Surface
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load, // 在 3D 映射后的画面上叠加 UI
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &main_depth.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0), // 关键：清除 3D 场景深度，开始 UI 深度测试
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

        render_pass.set_pipeline(self.pipeline.as_ref().unwrap());

        render_pass.set_vertex_buffer(0, vertex_buffer.buffer.slice(..));

        render_pass.set_index_buffer(index_buffer.buffer.slice(..), wgpu::IndexFormat::Uint32);

        render_pass.set_bind_group(1, &bindless_bind_group, &[]);

        for b in batches {
            let camera_offset = CameraUniform::get_uniform_offset_unit() * b.camera_index;

            render_pass.set_bind_group(0, &camera_bind_group, &[camera_offset]);

            render_pass.draw_indexed(b.index_range.clone(), 0, 0..1);
        }
    }
}

/// Prepare the sprite vertex buffer, index buffer, and the sprite batches.
fn create_batches(
    sprites: &Vec<ExtractedSprite2d>,
    context: &mut FrameContext,
    extracted_cameras: &crate::render::camera::ExtractedCameras,
) -> Vec<SpriteBatch> {
    if sprites.is_empty() {
        return vec![];
    }

    // 找到第一个 D2 类型的相机
    let camera_index = extracted_cameras
        .types
        .iter()
        .position(|t| *t == crate::render::camera::CameraType::D2);

    // 如果没有 2D 相机，则不渲染任何 2D 元素
    let camera_index = match camera_index {
        Some(idx) => idx as u32,
        None => return vec![],
    };

    let total_quads = sprites.len();

    let mut all_vertices = Vec::with_capacity(total_quads * 4);
    let mut all_indices = Vec::with_capacity(total_quads * 6);

    // 计算 Z 步长。我们希望越后抽取的元素 Z 越小（越靠近相机，在正交投影中，Z 越小越靠前）。
    let z_step = 1.0 / (total_quads as f32 + 1.0);
    let mut current_z = 0.0;

    // Fill the vertex and index buffer.
    for e in sprites {
        current_z -= z_step;

        let mut uvs = [
            Vec2::new(e.rect.x, e.rect.w), // BL
            Vec2::new(e.rect.z, e.rect.w), // BR
            Vec2::new(e.rect.z, e.rect.y), // TR
            Vec2::new(e.rect.x, e.rect.y), // TL
        ];

        if e.flip_x {
            uvs.swap(0, 1);
            uvs.swap(2, 3);
        }
        if e.flip_y {
            uvs.swap(0, 3);
            uvs.swap(1, 2);
        }

        let texture_idx = *context
            .prepared
            .texture_index_map
            .get(&e.texture_id)
            .unwrap_or(&0);
        let vertex_start = all_vertices.len() as u32;

        for i in 0..4 {
            let mut quad_pos = QUAD_VERTEX_POSITIONS[i];
            if !e.centered {
                quad_pos += Vec2::new(0.5, 0.5);
            }
            let new_pos = e.transform.transform_point(&(quad_pos * e.size));

            all_vertices.push(Vertex2d {
                position: [new_pos.x, new_pos.y, current_z],
                uv: uvs[i].into(),
                color: e.color,
                texture_idx,
                mode: e.mode,
            });
        }
        for i in QUAD_INDICES {
            all_indices.push(vertex_start + i);
        }
    }

    let vertex_buffer = context.buffer(&standard_resources::sprite_vertex_buffer());
    let index_buffer = context.buffer(&standard_resources::sprite_index_buffer());

    // Write to buffers.
    context.render_context.queue.write_buffer(
        &vertex_buffer.buffer,
        0,
        bytemuck::cast_slice(&all_vertices),
    );
    context.render_context.queue.write_buffer(
        &index_buffer.buffer,
        0,
        bytemuck::cast_slice(&all_indices),
    );

    let batches = vec![SpriteBatch {
        index_range: 0..all_indices.len() as u32,
        camera_index,
    }];

    batches
}

pub(crate) const QUAD_INDICES: [u32; 6] = [0, 2, 3, 0, 1, 2];

pub(crate) const QUAD_VERTEX_POSITIONS: [Vec2; 4] = [
    Vec2::new(-0.5, 0.5),
    Vec2::new(0.5, 0.5),
    Vec2::new(0.5, -0.5),
    Vec2::new(-0.5, -0.5),
];

#[derive(Debug, Clone)]
pub struct SpriteBatch {
    pub(crate) index_range: Range<u32>,
    pub(crate) camera_index: u32,
}

pub struct PreparedSprites {
    vertex_buffer: Option<PooledBuffer>,
    index_buffer: Option<PooledBuffer>,
    batches: Vec<SpriteBatch>,
}
