use std::any::Any;
use std::path::PathBuf;
use eureka::core::{App, Singletons};
use eureka::scene::{AsNode, AsNodeUi, NodeType};
use eureka::scene::Camera2d;
use eureka::scene::Sprite2d;
use glam::Vec2;
use eureka::render::draw_command::DrawCommands;
use eureka::render::render_world::RenderWorld;

fn main() {
    let mut app = App::new();

    app.setup(|app| {
        let singletons = app.singletons.as_mut().unwrap();
        let world = &mut app.world;

        // Add a 2D camera
        let camera = Camera2d::default();
        world.add_node(Box::new(camera), None);

        // Texture paths
        let tree_path = singletons
            .asset_server
            .asset_dir
            .join("images/happy-tree.png");
        let texture_path = singletons.asset_server.asset_dir.join("images/texture.jpg");

        // Add a sprite with texture
        let mut sprite1 = MySprite::at_path(tree_path.clone());
        sprite1.set_position(Vec2::new(200f32, 200f32));
        let sprite1_id = world.add_node(Box::new(sprite1), None);

        // Add another sprite with the same texture (only loaded once)
        let mut sprite2 = Sprite2d::at_path(tree_path);
        sprite2.set_position(Vec2::new(200f32, 200f32));
        world.add_node(Box::new(sprite2), Some(sprite1_id));

        // Add third sprite with a different texture
        let mut sprite3 = Sprite2d::at_path(texture_path);
        sprite3.set_position(Vec2::new(400f32, 400f32));
        world.add_node(Box::new(sprite3), None);
    });

    app.run();
}

pub struct MySprite {
    pub sprite2d: Sprite2d,
}

impl MySprite {
    pub fn new(sprite2d: Sprite2d) -> Self {
        Self {
            sprite2d,
        }
    }

    pub fn at_path(path: PathBuf) -> Self {
        Self {
            sprite2d: Sprite2d::at_path(path),
        }
    }

    pub fn set_position(&mut self, p: Vec2) {
        self.sprite2d.set_position(p);
    }
}

impl AsNode for MySprite {
    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }
    fn node_type(&self) -> NodeType { NodeType::Sprite2d }

    fn as_node_ui(&self) -> Option<&dyn AsNodeUi> {
        self.sprite2d.as_node_ui()
    }

    fn as_node_ui_mut(&mut self) -> Option<&mut dyn AsNodeUi> {
        self.sprite2d.as_node_ui_mut()
    }

    fn reconcile(&mut self, singletons: &mut Singletons, render_world: &mut RenderWorld) {
        self.sprite2d.reconcile(singletons, render_world);
    }

    fn update(&mut self, dt: f32, singletons: &mut Singletons) {
        self.sprite2d.set_rotation(self.sprite2d.get_rotation() + dt);

        // Base update
        self.sprite2d.update(dt, singletons);
    }

    fn draw(&self, draw_cmds: &mut DrawCommands) {
        self.sprite2d.draw(draw_cmds);
    }
}
