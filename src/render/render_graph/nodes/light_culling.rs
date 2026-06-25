use crate::render::camera::CameraUniform;
use crate::render::light::{
    Cluster, ClusterConfig, PointLightUniform, CLUSTER_GRID_SIZE, MAX_LIGHTS_PER_CLUSTER,
};
use crate::render::render_backend::PreparedFrame;
use crate::render::render_graph::{standard_resources, FrameContext, Node, ResourceSpec};
use std::any::Any;

const MAX_POINT_LIGHTS: usize = 1024;

/// LightCullingNode 是集群前向渲染 (Clustered Forward Rendering) 的核心组件。
///
/// 该节点利用 Compute Shader 实现光源的空间剔除，主要流程如下：
/// 1. **视锥体分块 (Clustering)**: 将摄像机视锥体在 3D 空间中划分为多层网格 (Clusters)。
/// 2. **光源剔除 (Culling)**: 遍历场景中所有的点光源，计算每个光源影响哪些 Cluster。
/// 3. **构建索引列表**: 生成 `light_grid` (存储每个网格的灯光偏移和数量) 和 `light_index_list` (存储具体的灯光索引)。
///
/// 通过该节点，后续的 `OpaqueMeshNode` 在着色时仅需根据像素位置定位到对应的 Cluster，
/// 即可直接获取对当前像素有贡献的灯光子集，从而支持场景中存在成百上千个动态光源。
pub struct LightCullingNode {
    pipeline: Option<wgpu::ComputePipeline>,
}

impl Default for LightCullingNode {
    fn default() -> Self {
        Self { pipeline: None }
    }
}

impl Node for LightCullingNode {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn node_resources(
        &self,
        _prepared: &PreparedFrame,
    ) -> crate::render::render_graph::resource::NodeResources {
        let total_clusters =
            (CLUSTER_GRID_SIZE[0] * CLUSTER_GRID_SIZE[1] * CLUSTER_GRID_SIZE[2]) as u64;

        let camera_buffer_size = CameraUniform::get_uniform_offset_unit()
            * crate::render::render_graph::nodes::prepare_view::MAX_CAMERAS;

        crate::render::render_graph::resource::NodeResources::new()
            .input(
                standard_resources::camera_buffer(),
                ResourceSpec::buffer(camera_buffer_size as u64, wgpu::BufferUsages::UNIFORM),
            )
            .output(
                standard_resources::point_light_storage_buffer(),
                ResourceSpec::buffer(
                    (size_of::<PointLightUniform>() * MAX_POINT_LIGHTS) as u64,
                    wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                ),
            )
            .output(
                standard_resources::light_grid_buffer(),
                ResourceSpec::buffer(
                    total_clusters * size_of::<Cluster>() as u64,
                    wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                ),
            )
            .output(
                standard_resources::light_index_list_buffer(),
                ResourceSpec::buffer(
                    total_clusters * MAX_LIGHTS_PER_CLUSTER as u64 * 4,
                    wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                ),
            )
            .output(
                standard_resources::light_index_count_buffer(),
                ResourceSpec::buffer(
                    4,
                    wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                ),
            )
            .internal(
                standard_resources::cluster_config_buffer(),
                ResourceSpec::buffer(
                    size_of::<ClusterConfig>() as u64,
                    wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                ),
            )
            .output(
                standard_resources::light_uniform_buffer(),
                ResourceSpec::buffer(
                    size_of::<crate::render::light::LightUniform>() as u64,
                    wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                ),
            )
    }

    fn run(&mut self, context: &mut FrameContext) {
        let device = &context.render_context.device;
        let queue = &context.render_context.queue;

        let light_culling_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: true,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 5,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
                label: None,
            });

        if self.pipeline.is_none() {
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Light Culling Pipeline Layout"),
                bind_group_layouts: &[Some(&light_culling_bind_group_layout)],
                immediate_size: 0,
            });

            let source = include_str!("../../../shaders/cluster_cull.wgsl").replace(
                "#import eureka::camera::Camera",
                crate::render::camera::CAMERA_STRUCT_WGSL,
            );

            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Light Culling Shader"),
                source: wgpu::ShaderSource::Wgsl(source.into()),
            });

            self.pipeline = Some(device.create_compute_pipeline(
                &wgpu::ComputePipelineDescriptor {
                    label: Some("Light Culling Pipeline"),
                    layout: Some(&layout),
                    module: &shader,
                    entry_point: Some("main"),
                    compilation_options: Default::default(),
                    cache: None,
                },
            ));
        }

        // 1. Update Buffers
        let lights_extracted = &context.extracted.lights;

        let light_uniform_buffer = context.buffer(&standard_resources::light_uniform_buffer());
        let mut light_uniform = crate::render::light::LightUniform::default();
        light_uniform.ambient_color = [1.0, 1.0, 1.0];
        light_uniform.ambient_strength = 0.01;

        // 设置默认的体积雾/体积光参数
        light_uniform.fog_color = [0.5, 0.6, 0.7]; // 浅蓝色调的雾
        light_uniform.fog_density = 1.0; // 基础密度
        light_uniform.fog_height_falloff = 0.2; // 随高度缓慢变稀
        light_uniform.fog_base_height = -1.0;
        light_uniform.fog_scattering = 0.8; // 散射系数
        light_uniform.fog_absorption = 0.01; // 吸收系数

        if let Some(dl) = lights_extracted.directional_light {
            light_uniform.directional_light = dl;
        }
        queue.write_buffer(
            &light_uniform_buffer.buffer,
            0,
            bytemuck::cast_slice(&[light_uniform]),
        );

        let point_lights = &lights_extracted.point_lights;
        let light_storage_buffer =
            context.buffer(&standard_resources::point_light_storage_buffer());
        queue.write_buffer(
            &light_storage_buffer.buffer,
            0,
            bytemuck::cast_slice(point_lights),
        );

        let config_buffer = context.buffer(&standard_resources::cluster_config_buffer());
        let config = ClusterConfig {
            screen_size: [
                context.render_context.surface_config.width as f32,
                context.render_context.surface_config.height as f32,
            ],
            _pad0: [0.0; 2],
            grid_size: CLUSTER_GRID_SIZE,
            num_lights: point_lights.len() as u32,
            z_near: 0.1,
            z_far: 100.0,
            _pad1: [0.0; 2],
        };
        queue.write_buffer(&config_buffer.buffer, 0, bytemuck::cast_slice(&[config]));

        // 使用 encoder 清理 Buffer 比 queue.write_buffer 快得多
        let count_buffer = context.buffer(&standard_resources::light_index_count_buffer());
        context.encoder.clear_buffer(&count_buffer.buffer, 0, None);

        let light_grid_buffer = context.buffer(&standard_resources::light_grid_buffer());
        context
            .encoder
            .clear_buffer(&light_grid_buffer.buffer, 0, None);

        // 2. Bind Group
        let camera_buffer = context.buffer(&standard_resources::camera_buffer());
        let index_list_buffer = context.buffer(&standard_resources::light_index_list_buffer());

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.pipeline.as_ref().unwrap().get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &camera_buffer.buffer,
                        offset: 0,
                        size: Some(
                            wgpu::BufferSize::new(size_of::<CameraUniform>() as u64).unwrap(),
                        ),
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: light_storage_buffer.buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: light_grid_buffer.buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: index_list_buffer.buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: count_buffer.buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: config_buffer.buffer.as_entire_binding(),
                },
            ],
            label: None,
        });

        // 3. Dispatch
        let mut cpass = context
            .encoder
            .begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Light Cull Pass"),
                timestamp_writes: None,
            });
        cpass.set_pipeline(self.pipeline.as_ref().unwrap());
        cpass.set_bind_group(0, &bind_group, &[0]);
        // 修正：Dispatch 1x1 即可，因为 Shader 内部 workgroup 大小已经设为 16x9
        cpass.dispatch_workgroups(1, 1, 1);
    }
}
