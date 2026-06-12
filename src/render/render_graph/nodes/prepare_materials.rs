use crate::render::render_backend::PreparedFrame;
use crate::render::render_graph::{standard_resources, FrameContext, Node, SamplerKey};
use std::any::Any;

#[derive(Default)]
pub struct PrepareMaterialsNode;

impl Node for PrepareMaterialsNode {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn node_resources(
        &self,
        prepared: &PreparedFrame,
    ) -> crate::render::render_graph::resource::NodeResources {
        use crate::render::render_graph::resource::ResourceSpec;

        let material_buffer_size = prepared.material_uniforms.len()
            * size_of::<crate::render::material::MaterialUniform>();

        crate::render::render_graph::resource::NodeResources::new().output(
            standard_resources::material_storage_buffer(),
            ResourceSpec::buffer(
                material_buffer_size as u64,
                wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            ),
        )
    }

    fn run(&mut self, context: &mut FrameContext) {
        let materials_storage_buffer =
            context.buffer(&standard_resources::material_storage_buffer());

        context.render_context.queue.write_buffer(
            &materials_storage_buffer.buffer,
            0,
            bytemuck::cast_slice(&context.prepared.material_uniforms),
        );

        let bindless_bind_group_layout = context
            .backend
            .get_bind_group_layout("bindless_bind_group_layout")
            .unwrap()
            .clone();

        let dummy_sampler = context.get_sampler(SamplerKey {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        // 占位纹理
        let placeholder_view = if !context.prepared.bindless_texture_ids.is_empty() {
            let texture_cache = context.backend.imported_texture_cache.read().unwrap();

            let texture_id = context.prepared.bindless_texture_ids[0];
            texture_cache.get(texture_id).unwrap().view.clone()
        } else {
            context.backend.dummy_2d_view.clone()
        };

        let mut final_bindless_views = vec![placeholder_view; 1024];
        for (i, texture_id) in context.prepared.bindless_texture_ids.iter().enumerate() {
            let texture_cache = context.backend.imported_texture_cache.read().unwrap();
            final_bindless_views[i] = texture_cache.get(*texture_id).unwrap().view.clone();
        }

        let bindless_views_ref: Vec<&wgpu::TextureView> = final_bindless_views.iter().collect();

        // FIXME: add view keys.
        let _bindless_bind_group = context.create_bind_group(
            "bindless_bind_group_layout",
            vec![materials_storage_buffer.id],
            |ctx| {
                ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    layout: &bindless_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: materials_storage_buffer.buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::TextureViewArray(&bindless_views_ref),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: wgpu::BindingResource::Sampler(&dummy_sampler),
                        },
                    ],
                    label: Some("bindless bind group"),
                })
            },
        );
    }
}
