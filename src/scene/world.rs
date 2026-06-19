use crate::core::singleton::Singletons;
use crate::render::render_world::Extracted;
use crate::window::InputServer;
use hecs::World as EcsWorld;

pub struct World {
    pub ecs: EcsWorld,
}

impl World {
    pub fn new() -> Self {
        Self {
            ecs: EcsWorld::new(),
        }
    }

    /// 核心更新逻辑：它是“系统”的集合
    pub fn update(
        &mut self,
        dt: f32,
        singletons: &mut Singletons,
        render_world: &mut crate::render::render_world::RenderWorld,
    ) {
        // 0. 先更新资产服务器，从后台线程接收已加载的资产
        singletons.asset_server.update();

        // 1. 资产加载系统
        crate::scene::systems::update_assets(&mut self.ecs, singletons, render_world);

        // 2. 摄像机同步系统
        crate::scene::systems::update_cameras(&mut self.ecs, singletons);

        // 3. 动画系统
        crate::scene::systems::update_animations(&mut self.ecs, dt);

        // 4. 示例中的自定义逻辑系统
        crate::scene::systems::update_example_logic(&mut self.ecs, dt);

        // 5. 变换传播系统
        crate::scene::systems::propagate_transforms(&mut self.ecs);

        // 6. Label 系统
        crate::scene::systems::update_labels(&mut self.ecs, singletons);
    }

    /// 渲染提取系统：从 ECS 中提取渲染命令
    pub fn extract_render_objects(&mut self) -> Extracted {
        crate::scene::systems::extract_render_objects(&mut self.ecs)
    }

    pub fn input(&mut self, input_server: &mut InputServer) {
        crate::scene::systems::handle_input(&mut self.ecs, input_server);
    }
}
