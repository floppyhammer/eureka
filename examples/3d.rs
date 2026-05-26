use std::any::Any;
use std::path::PathBuf;
use eureka::core::{App, Singletons};
use eureka::render::render_world::RenderWorld;
use eureka::scene::{AsNode, AsNode3d, Camera3d, DirectionalLight, Model, NodeType, PointLight, Sky, Sprite2d};
use glam::{Quat, Vec3};
use eureka::render::draw_command::DrawCommands;

fn main() {
    let mut app = App::new();

    app.setup(|app| {
        // Destructure fields of App to leverage Rust's "split borrows" feature.
        let singletons = app.singletons.as_mut().unwrap();
        let world = &mut app.world;

        let camera3d = Camera3d::new(
            Vec3::new(-10.0, 0.0, 0.0),
            0.0,
            0.0,
            &singletons.render_server,
        );
        world.add_node(Box::new(camera3d), None);

        // Add a skybox
        let skybox_path = singletons.asset_server.asset_dir.join("images/skybox.jpg");
        let sky = Sky::at_path(skybox_path);
        world.add_node(Box::new(sky), None);

        // Add a point light
        let mut light = PointLight::new();
        light.set_position(Vec3::new(2.0, 5.0, 0.0));
        light.strength = 5.0;
        world.add_node(Box::new(light), None);

        // Add a directional light
        let mut light = DirectionalLight::new();
        light.strength = 1.5;
        light.transform.rotation = Quat::from_rotation_x(-135.0f32.to_radians());
        world.add_node(Box::new(light), None);

        // Add a crab
        let ferris_path = singletons
            .asset_server
            .asset_dir
            .join("models/ferris/ferris3d_v1.0.obj");
        let mut ferris = Ferris::at_path(ferris_path);
        ferris.set_position(Vec3::new(0.0, 0.1, 0.0));
        ferris.set_scale(Vec3::new(1.0, 1.0, 1.0));
        world.add_node(Box::new(ferris), None);

        let cube = singletons
            .asset_server
            .asset_dir
            .join("models/cube/cube.obj");
        let mut cube = MyCube::at_path(cube);
        cube.set_position(Vec3::new(2.0, 1.2, 2.0));
        cube.set_scale(Vec3::new(0.5, 0.5, 0.5));
        world.add_node(Box::new(cube), None);

        let spheres = singletons
            .asset_server
            .asset_dir
            .join("models/MetalRoughSpheres.gltf");
        let mut spheres = Model::at_path(spheres);
        spheres.set_position(Vec3::new(-2.0, 2.0, -2.0));
        spheres.set_scale(Vec3::new(0.1, 0.1, 0.1));
        let rotation_delta = Quat::from_rotation_z(90.0_f32.to_radians());
        spheres.set_rotation(rotation_delta);
        world.add_node(Box::new(spheres), None);

        // Add ground
        let ground_path = singletons
            .asset_server
            .asset_dir
            .join("models/granite_ground/granite_ground.obj");
        let mut ground = Model::at_path(ground_path);
        ground.set_scale(Vec3::new(5.0, 1.0, 5.0));
        world.add_node(Box::new(ground), None);
    });

    app.run();
}

pub struct MyCube {
    pub model: Model,
}

impl MyCube {
    pub fn new(model: Model) -> Self {
        Self {
            model,
        }
    }

    pub fn at_path(path: PathBuf) -> Self {
        Self {
            model: Model::at_path(path),
        }
    }

    pub fn set_position(&mut self, p: Vec3) {
        self.model.set_position(p);
    }

    pub fn set_scale(&mut self, s: Vec3) {
        self.model.set_scale(s);
    }
}

impl AsNode for MyCube {
    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }
    fn node_type(&self) -> NodeType { NodeType::Model }

    fn reconcile(&mut self, singletons: &mut Singletons, render_world: &mut RenderWorld) {
        self.model.reconcile(singletons, render_world);
    }

    fn update(&mut self, dt: f32, singletons: &mut Singletons) {
        let rotation_delta = Quat::from_rotation_y(dt);

        let new_rotation = rotation_delta * self.model.get_rotation();

        self.model.set_rotation(new_rotation);

        // Base model update
        self.model.update(dt, singletons);
    }

    fn draw(&self, draw_cmds: &mut DrawCommands) {
        self.model.draw(draw_cmds);
    }

    fn as_node_3d(&self) -> Option<&dyn AsNode3d> {
        self.model.as_node_3d()
    }

    fn as_node_3d_mut(&mut self) -> Option<&mut dyn AsNode3d> {
        self.model.as_node_3d_mut()
    }
}

pub struct Ferris {
    pub model: Model,
    pub speed: f32,
    pub timer: f32,
}

impl Ferris {
    pub fn new(model: Model, speed: f32) -> Self {
        Self {
            model,
            speed,
            timer: 0.0,
        }
    }

    pub fn at_path(path: PathBuf) -> Self {
        Self {
            model: Model::at_path(path),
            speed: 1.0,
            timer: 0.0,
        }
    }

    pub fn set_position(&mut self, p: Vec3) {
        self.model.set_position(p);
    }

    pub fn set_scale(&mut self, s: Vec3) {
        self.model.set_scale(s);
    }
}

impl AsNode for Ferris {
    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }
    fn node_type(&self) -> NodeType { NodeType::Model }

    fn reconcile(&mut self, singletons: &mut Singletons, render_world: &mut RenderWorld) {
        self.model.reconcile(singletons, render_world);
    }

    fn update(&mut self, dt: f32, singletons: &mut Singletons) {
        self.timer += dt * self.speed;

        let mut pos = self.model.get_position();
        pos.y = 1.0 + self.timer.sin() * 1.0;
        self.model.set_position(pos);

        // Base model update
        self.model.update(dt, singletons);
    }

    fn draw(&self, draw_cmds: &mut DrawCommands) {
        self.model.draw(draw_cmds);
    }

    fn as_node_3d(&self) -> Option<&dyn AsNode3d> {
        self.model.as_node_3d()
    }

    fn as_node_3d_mut(&mut self) -> Option<&mut dyn AsNode3d> {
        self.model.as_node_3d_mut()
    }
}
