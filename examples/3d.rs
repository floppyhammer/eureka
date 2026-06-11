use eureka::core::{App, Singletons};
use eureka::render::draw_command::DrawCommands;
use eureka::render::render_world::RenderWorld;
use eureka::scene::{AsNode, AsNode2d, AsNode3d, Camera2d, Camera3d, DirectionalLight, Label, Model, NodeType, PointLight, Sky};
use eureka::window::{InputEvent, InputServer};
use glam::{Quat, Vec2, Vec3};
use std::any::Any;
use std::path::PathBuf;
use winit::keyboard::KeyCode;

fn main() {
    let mut app = App::new();

    app.setup(|app| {
        let singletons = app.singletons.as_mut().unwrap();
        let world = &mut app.world;

        let camera3d = Camera3d::new(
            Vec3::new(-10.0, 2.0, 0.0),
            0.0,
            0.0,
            &singletons.render_context,
        );
        world.add_node(Box::new(camera3d), None);

        // Add an orthographic 2D camera for the UI overlay.
        let mut camera2d = Camera2d::default();
        camera2d.when_view_size_changes(glam::UVec2::new(
            singletons.render_context.surface_config.width,
            singletons.render_context.surface_config.height,
        ));
        world.add_node(Box::new(camera2d), None);

        // Add settings controller (which also acts as a label)
        let controller = SettingsController::new();
        world.add_node(Box::new(controller), None);

        // Add a skybox
        let skybox_path = singletons.asset_server.asset_dir.join("images/skybox.jpg");
        let sky = Sky::at_path(skybox_path);
        world.add_node(Box::new(sky), None);

        // Add a point light
        let mut light = PointLight::new();
        light.set_position(Vec3::new(0.0, 5.0, 0.0));
        light.strength = 5.0;
        world.add_node(Box::new(light), None);

        // Add a directional light
        let mut light = DirectionalLight::new();
        light.strength = 1.5;
        light.node_3d.transform.rotation = Quat::from_rotation_x(-135.0f32.to_radians());
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
            .join("models/ground.glb"); // Sponza/Sponza.gltf
        let ground = Model::at_path(ground_path);
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

pub struct SettingsController {
    label: Label,
    ssao: bool,
    fxaa: bool,
}

impl SettingsController {
    pub fn new() -> Self {
        let mut label = Label::new("");
        label.set_position(Vec2::new(20.0, 20.0));
        let mut s = Self {
            label,
            ssao: true,
            fxaa: true,
        };
        s.refresh_text();
        s
    }

    fn refresh_text(&mut self) {
        self.label.set_text(format!(
            "SSAO (1): {} | FXAA (2): {}",
            if self.ssao { "ON" } else { "OFF" },
            if self.fxaa { "ON" } else { "OFF" }
        ));
    }
}

impl AsNode for SettingsController {
    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }
    fn node_type(&self) -> NodeType { NodeType::Label }

    fn input(&mut self, input_event: &mut InputEvent, _input_server: &mut InputServer) {
        if let InputEvent::Key(key) = input_event {
            if key.pressed {
                match key.key_code {
                    KeyCode::Digit1 => {
                        self.ssao = !self.ssao;
                        self.refresh_text();
                    }
                    KeyCode::Digit2 => {
                        self.fxaa = !self.fxaa;
                        self.refresh_text();
                    }
                    _ => {}
                }
            }
        }
    }

    fn update(&mut self, dt: f32, singletons: &mut Singletons) {
        self.label.update(dt, singletons);
    }

    fn draw(&self, draw_cmds: &mut DrawCommands) {
        self.label.draw(draw_cmds);
        draw_cmds.extracted.fxaa_enabled = self.fxaa;
        draw_cmds.extracted.ssao_enabled = self.ssao;
    }

    fn as_node_2d(&self) -> Option<&dyn AsNode2d> {
        Some(&self.label)
    }

    fn as_node_2d_mut(&mut self) -> Option<&mut dyn AsNode2d> {
        Some(&mut self.label)
    }
}
