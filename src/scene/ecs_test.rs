use crate::render::atlas::{AtlasMode, DrawAtlas};
use crate::{AtlasInstance, RenderServer, Texture};
use bevy_ecs::prelude::*;
use cgmath::{Quaternion, Vector2, Vector3, Vector4};
use wgpu::RenderPass;

#[derive(Component)]
struct CpTransform {
    position: Vector3<f32>,
    rotation: Quaternion<f32>,
    scale: Vector3<f32>,
}

impl Default for CpTransform {
    fn default() -> Self {
        Self {
            position: Vector3::new(0.0, 0.0, 0.0),
            rotation: Quaternion::new(0.0, 0.0, 0.0, 0.0),
            scale: Vector3::new(1.0, 1.0, 1.0),
        }
    }
}

#[derive(Component, Default)]
pub(crate) struct CpAtlas {
    pub(crate) instances: Vec<AtlasInstance>,
    instance_buffer: Option<wgpu::Buffer>,
    texture: Option<Texture>,
    texture_bind_group: Option<wgpu::BindGroup>,
    atlas_params_buffer: Option<wgpu::Buffer>,
    atlas_params_bind_group: Option<wgpu::BindGroup>,
    mode: AtlasMode,
}

#[derive(Bundle, Default)]
struct SpriteBundle {
    atlas: CpAtlas,
    transform: CpTransform,
}

// fn render_atlas<'a>(query: Query<(Entity, &CpAtlas, &CpTransform)>, render_server: Res<RenderServer>, mut render_pass: ResMut<RenderPass<'a>>) {
//     for (entity, atlas, transform) in &query {
//         println!("Entity {:?} is at position: {:?}", entity, transform.position);
//
//         // render_pass.draw_atlas(
//         //     &render_server.atlas_pipeline,
//         //     &atlas.instance_buffer.unwrap(),
//         //     atlas.instances.len() as u32,
//         //     &atlas.texture_bind_group.unwrap(),
//         //     &atlas.atlas_params_bind_group.unwrap(),
//         // );
//     }
// }

struct EcsController {
    world: World,
    schedule: Schedule,
}

impl EcsController {
    fn new(render_server: RenderServer) -> Self {
        let mut world = World::new();

        world.insert_resource(render_server);

        // Spawn a new entity and insert the default PlayerBundle
        world.spawn(SpriteBundle::default());

        // Create a new Schedule, which defines an execution strategy for Systems
        let mut schedule = Schedule::default();

        // Add a Stage to our schedule. Each Stage in a schedule runs all of its systems
        // before moving on to the next Stage.
        // schedule.add_stage("update", SystemStage::parallel()
        //     .with_system(render_atlas),
        // );

        Self { world, schedule }
    }

    fn run(&mut self, render_pass: RenderPass) {
        // self.world.insert_resource(render_pass);

        // Run the schedule once. If your app has a "loop", you would run this once per loop.
        self.schedule.run(&mut self.world);
    }
}
