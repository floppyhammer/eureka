use crate::render::shader_maker::ShaderMaker;
use crate::render::vertex::VertexBuffer;
use crate::render::{RenderContext, TextureCache, TextureId};
use glam::{UVec2, Vec2, Vec4};
use std::collections::HashMap;
use std::mem;
use wgpu::{BufferAddress, DynamicOffset, RenderPass, SamplerBindingType};

pub struct AtlasRenderResources {
    params_bind_group_layout: wgpu::BindGroupLayout,
    params_bind_group: Option<wgpu::BindGroup>,
    params_buffer: Option<wgpu::Buffer>,
    params_buffer_capacity: usize,
    instance_buffer: Option<wgpu::Buffer>,
    instance_buffer_capacity: usize,
    pub(crate) texture_bind_group_layout: wgpu::BindGroupLayout,
    pub(crate) texture_bind_group_cache: HashMap<TextureId, wgpu::BindGroup>,
    pub(crate) pipeline_cache: HashMap<AtlasMode, wgpu::RenderPipeline>,
}

impl AtlasRenderResources {
    pub(crate) fn new(render_server: &RenderContext) -> Self {
        let params_bind_group_layout = render_server.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::VERTEX, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: true, min_binding_size: None }, count: None }],
            label: Some("atlas params bind group layout"),
        });
        let texture_bind_group_layout = render_server.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Texture { multisampled: false, view_dimension: wgpu::TextureViewDimension::D2, sample_type: wgpu::TextureSampleType::Float { filterable: true } }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(SamplerBindingType::Filtering), count: None },
            ],
            label: Some("atlas texture bind group layout"),
        });
        Self { params_bind_group_layout, params_bind_group: None, params_buffer: None, params_buffer_capacity: 0, instance_buffer: None, texture_bind_group_layout, texture_bind_group_cache: HashMap::new(), instance_buffer_capacity: 0, pipeline_cache: Default::default() }
    }

    fn create_pipeline(&mut self, mode: AtlasMode, render_server: &RenderContext, shader_maker: &mut ShaderMaker) {
        if self.pipeline_cache.contains_key(&mode) { return; }
        let device = &render_server.device;
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("atlas layout"), bind_group_layouts: &[&self.params_bind_group_layout, &self.texture_bind_group_layout], push_constant_ranges: &[] });
        let defs = if mode == AtlasMode::Text { vec!["TEXT"] } else { vec![] };
        let shader = wgpu::ShaderModuleDescriptor { label: Some("atlas shader"), source: shader_maker.make_shader(include_str!("../shaders/atlas.wgsl"), &defs).unwrap() };
        let shader_module = device.create_shader_module(shader);
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("atlas pipeline"), layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState { module: &shader_module, entry_point: Some("vs_main"), compilation_options: Default::default(), buffers: &[AtlasInstanceRaw::desc()] },
            fragment: Some(wgpu::FragmentState { module: &shader_module, entry_point: Some("fs_main"), compilation_options: Default::default(), targets: &[Some(wgpu::ColorTargetState { format: render_server.surface_config.format, blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING), write_mask: wgpu::ColorWrites::ALL })] }),
            primitive: wgpu::PrimitiveState { topology: wgpu::PrimitiveTopology::TriangleStrip, front_face: wgpu::FrontFace::Cw, cull_mode: None, ..Default::default() },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(), multiview: None, cache: None,
        });
        self.pipeline_cache.insert(mode, pipeline);
    }
}

#[derive(Clone)] pub struct AtlasInstance { pub(crate) position: Vec2, pub(crate) size: Vec2, pub(crate) region: Vec4, pub(crate) color: Vec4 }
#[derive(Clone)] pub struct ExtractedAtlas { pub(crate) atlas: Atlas, pub(crate) view_size: UVec2 }
#[repr(C)] #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)] pub(crate) struct AtlasInstanceRaw { position: [f32; 2], size: [f32; 2], region: [f32; 4], color: [f32; 4] }
#[repr(C)] #[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)] pub(crate) struct AtlasParamsUniform { camera_view_size: [f32; 2], atlas_size: [f32; 2] }
impl AtlasParamsUniform {
    pub(crate) fn get_uniform_offset_unit() -> u32 {
        let offset_alignment = wgpu::Limits::downlevel_defaults().min_uniform_buffer_offset_alignment;
        let size = size_of::<AtlasParamsUniform>() as u32;
        (size + offset_alignment - 1) & !(offset_alignment - 1)
    }
    pub(crate) fn new(atlas_size: UVec2, camera_view_size: UVec2) -> Self { Self { camera_view_size: [camera_view_size.x as f32, camera_view_size.y as f32], atlas_size: [atlas_size.x as f32, atlas_size.y as f32] } }
}
#[derive(Default, Copy, Clone, Eq, Hash, PartialEq)] pub(crate) enum AtlasMode { #[default] Sprite = 0x1, Text = 0x2 }
impl AtlasInstance { fn to_raw(&self) -> AtlasInstanceRaw { AtlasInstanceRaw { position: self.position.into(), size: self.size.into(), region: self.region.into(), color: self.color.into() } } }
impl VertexBuffer for AtlasInstanceRaw {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout { array_stride: mem::size_of::<AtlasInstanceRaw>() as wgpu::BufferAddress, step_mode: wgpu::VertexStepMode::Instance, attributes: &[
            wgpu::VertexAttribute { offset: 0, shader_location: 0, format: wgpu::VertexFormat::Float32x2 },
            wgpu::VertexAttribute { offset: 8, shader_location: 1, format: wgpu::VertexFormat::Float32x2 },
            wgpu::VertexAttribute { offset: 16, shader_location: 2, format: wgpu::VertexFormat::Float32x4 },
            wgpu::VertexAttribute { offset: 32, shader_location: 3, format: wgpu::VertexFormat::Float32x4 },
        ]}
    }
}
#[derive(Default, Clone)] pub(crate) struct Atlas { pub(crate) texture: Option<TextureId>, pub(crate) instances: Vec<AtlasInstance>, pub(crate) texture_size: (u32, u32), pub(crate) mode: AtlasMode }

pub fn prepare_atlas(extracted: &Vec<ExtractedAtlas>, render_resources: &mut AtlasRenderResources, render_server: &RenderContext, texture_cache: &TextureCache, shader_maker: &mut ShaderMaker) {
    if extracted.is_empty() { return; }
    let device = &render_server.device;
    let mut all_instances = vec![];
    for e in extracted { all_instances.extend(e.atlas.instances.clone()); }
    let instance_count = all_instances.len();
    if render_resources.instance_buffer_capacity < instance_count || render_resources.instance_buffer.is_none() {
        render_resources.instance_buffer_capacity = instance_count;
        render_resources.instance_buffer = Some(device.create_buffer(&wgpu::BufferDescriptor { label: Some("atlas instance"), size: (mem::size_of::<AtlasInstanceRaw>() * instance_count) as BufferAddress, usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false }));
    }
    let instance_data: Vec<_> = all_instances.iter().map(AtlasInstance::to_raw).collect();
    render_server.queue.write_buffer(render_resources.instance_buffer.as_ref().unwrap(), 0, bytemuck::cast_slice(&instance_data));

    let offset = AtlasParamsUniform::get_uniform_offset_unit();
    let atlas_count = extracted.len();
    if render_resources.params_buffer_capacity < atlas_count {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor { label: Some("atlas params"), size: (offset * atlas_count as u32) as BufferAddress, usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor { layout: &render_resources.params_bind_group_layout, entries: &[wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding { buffer: &buffer, offset: 0, size: Some(wgpu::BufferSize::new(mem::size_of::<AtlasParamsUniform>() as u64).unwrap()) }) }], label: None });
        render_resources.params_buffer = Some(buffer);
        render_resources.params_bind_group = Some(bind_group);
        render_resources.params_buffer_capacity = atlas_count;
    }
    let mut aligned_up_data = vec![0u8; offset as usize * atlas_count];
    for (i, e) in extracted.iter().enumerate() {
        let uniform = AtlasParamsUniform::new(e.atlas.texture_size.into(), e.view_size);
        let bytes = bytemuck::bytes_of(&uniform);
        aligned_up_data[i * offset as usize..i * offset as usize + bytes.len()].copy_from_slice(bytes);
    }
    render_server.queue.write_buffer(render_resources.params_buffer.as_ref().unwrap(), 0, &aligned_up_data);

    for e in extracted {
        let texture_id = e.atlas.texture.unwrap();
        if !render_resources.texture_bind_group_cache.contains_key(&texture_id) {
            let texture = texture_cache.get(texture_id).unwrap();
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor { layout: &render_resources.texture_bind_group_layout, entries: &[wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&texture.view) }, wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&texture.sampler) }], label: None });
            render_resources.texture_bind_group_cache.insert(texture_id, bind_group);
        }
        render_resources.create_pipeline(e.atlas.mode, render_server, shader_maker);
    }
}

pub fn render_atlas<'a, 'b: 'a>(atlases: &'b Vec<ExtractedAtlas>, render_resources: &'b AtlasRenderResources, render_pass: &mut RenderPass<'a>) {
    let mut instance_offset = 0u32;
    let offset_unit = AtlasParamsUniform::get_uniform_offset_unit();
    for (i, e) in atlases.iter().enumerate() {
        let a = &e.atlas;
        let pipeline = render_resources.pipeline_cache.get(&a.mode).unwrap();
        let texture_bg = render_resources.texture_bind_group_cache.get(&a.texture.unwrap()).unwrap();
        render_pass.set_pipeline(pipeline);
        render_pass.set_vertex_buffer(0, render_resources.instance_buffer.as_ref().unwrap().slice(..));
        render_pass.set_bind_group(0, render_resources.params_bind_group.as_ref().unwrap(), &[(i as u32 * offset_unit) as DynamicOffset]);
        render_pass.set_bind_group(1, texture_bg, &[]);
        render_pass.draw(0..4, instance_offset..instance_offset + a.instances.len() as u32);
        instance_offset += a.instances.len() as u32;
    }
}
