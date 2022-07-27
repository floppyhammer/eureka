use crate::resource::{Material2d, Mesh, Texture};
use crate::scene::node::WithDraw;

pub struct Sprite {
    pub name: String,

    pub position: cgmath::Vector2<f32>,
    pub size: cgmath::Vector2<f32>,
    pub scale: cgmath::Vector2<f32>,

    pub texture: Option<Texture>,

    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,

    pub mesh: Mesh,
}

// impl WithDraw for Sprite {
//     fn draw(&self, render_pass: &mut wgpu::RenderPass, camera_bind_group: &wgpu::BindGroup) {
//         render_pass.draw_sprite(&self.mesh, &self.bind_group, &camera_bind_group);
//     }
// }

impl Sprite {
    fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Sprite {
        let position = cgmath::Vector2::new(0.0 as f32, 0.0);
        let size = cgmath::Vector2::new(128.0 as f32, 128.0);
        let scale = cgmath::Vector2::new(1.0 as f32, 1.0);

        let mesh = Mesh::default_2d(device);

        let bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                }],
                label: None,
            });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[],
            label: None,
        });

        Self {
            name: "".to_string(),
            position,
            size,
            scale,
            texture: None,
            bind_group_layout,
            bind_group,
            mesh,
        }
    }

    fn set_texture(&mut self, device: &wgpu::Device, new_texture: Texture) {
        self.texture = Some(new_texture);

        self.bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&(self.texture.as_ref().unwrap().view)),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.texture.as_ref().unwrap().sampler),
                },
            ],
            label: None,
        });
    }

    fn draw<'a, 'b>(&'b self, render_pass: &'a mut wgpu::RenderPass<'b>, camera_bind_group: &'b wgpu::BindGroup)
        where 'b: 'a {
        render_pass.draw_sprite(&self.mesh, &self.bind_group, &camera_bind_group);
    }
}

pub trait DrawSprite<'a> {
    fn draw_sprite(
        &mut self,
        mesh: &'a Mesh,
        texture_bind_group: &'a wgpu::BindGroup,
        camera_bind_group: &'a wgpu::BindGroup,
    );
}

impl<'a, 'b> DrawSprite<'b> for wgpu::RenderPass<'a>
    where 'b: 'a, // This means 'b must outlive 'a.
{
    fn draw_sprite(
        &mut self,
        mesh: &'b Mesh,
        texture_bind_group: &'b wgpu::BindGroup,
        camera_bind_group: &'b wgpu::BindGroup,
    ) {
        // Set vertex buffer for VertexInput.
        self.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));

        self.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

        // Set texture.
        self.set_bind_group(0, &texture_bind_group, &[]);

        // Set camera uniform.
        self.set_bind_group(1, &camera_bind_group, &[]);

        self.draw_indexed(0..mesh.index_count, 0, 0..1);
    }
}
