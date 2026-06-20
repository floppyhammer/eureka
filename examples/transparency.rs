use eureka::core::App;
use eureka::math::transform::{Transform2d, Transform3d};
use eureka::scene::{
    ActiveCamera, CTransform2d, CTransform3d,
    Camera3dComponent, Camera3dController, DirectionalLightComponent, GlobalTransform,
    LabelComponent, Model, Name, MeshPrimitive, SkyAssetPending,
};
use eureka::window::InputContent;
use glam::{Quat, Vec2, Vec3};
use winit::keyboard::KeyCode;

fn main() {
    let mut app = App::new();

    app.setup(|app| {
        let singletons = app.singletons.as_mut().unwrap();
        let world = &mut app.world;
        let render_world = app.render_world.as_ref().unwrap();

        // 资源缓存的快捷引用
        let mut texture_cache = render_world.imported_texture_cache.write().unwrap();
        let mut material_cache = render_world.imported_material_cache.write().unwrap();
        let mut mesh_cache = render_world.imported_mesh_cache.write().unwrap();
        let mut mesh_allocator = render_world.imported_mesh_allocator.write().unwrap();

        // 1. 添加 3D 摄像机和控制器
        let mut controller = Camera3dController::new(4.0, 0.003);
        controller.yaw = -90.0f32.to_radians();

        world.ecs.spawn((
            Name("MainCamera".into()),
            CTransform3d(Transform3d {
                position: Vec3::new(-5.0, 3.0, 0.0),
                ..Transform3d::default()
            }),
            GlobalTransform::default(),
            Camera3dComponent::new(),
            ActiveCamera,
            controller,
        ));

        // 2. UI 覆盖层
        world.ecs.spawn((
            Name("UICamera".into()),
            CTransform2d(Transform2d::default()),
            GlobalTransform::default(),
            eureka::scene::d2::camera2d::Camera2dComponent::default(),
            ActiveCamera,
        ));

        world.ecs.spawn((
            Name("Settings".into()),
            LabelComponent::new("Testing Transparency: Opaque vs Transparent Primitives"),
            CTransform2d(Transform2d {
                position: Vec2::new(20.0, 20.0),
                ..Transform2d::default()
            }),
            GlobalTransform::default(),
        ));

        let rot_x = Quat::from_rotation_x(-45.0f32.to_radians());
        let rot_y = Quat::from_rotation_y(-45.0f32.to_radians());

        world.ecs.spawn((
            Name("DirLight".into()),
            CTransform3d(Transform3d {
                rotation: rot_x * rot_y,
                ..Transform3d::default()
            }),
            GlobalTransform::default(),
            DirectionalLightComponent {
                strength: 1.5,
                ..DirectionalLightComponent::default()
            },
        ));

        // --- 4. 创建基础几何体 ---

        // A. 地面 (不透明)
        let ground_model = Model::from_primitive(
            MeshPrimitive::Plane { size: 20.0 },
            &singletons.render_context,
            &mut texture_cache,
            &mut material_cache,
            &mut mesh_cache,
            &mut mesh_allocator,
        );
        world.ecs.spawn((
            Name("Ground".into()),
            CTransform3d(Transform3d::default()),
            GlobalTransform::default(),
            ground_model,
        ));

        // B. 不透明立方体 (红色)
        let mut opaque_cube = Model::from_primitive(
            MeshPrimitive::Cube,
            &singletons.render_context,
            &mut texture_cache,
            &mut material_cache,
            &mut mesh_cache,
            &mut mesh_allocator,
        );
        if let Some(Some(mat_id)) = opaque_cube.materials.get(0) {
            if let Some(mat) = material_cache.storage.get_mut(mat_id) {
                mat.base_color = [1.0, 0.0, 0.0, 1.0];
                mat.roughness = 0.2;
            }
        }
        world.ecs.spawn((
            Name("OpaqueCube".into()),
            CTransform3d(Transform3d {
                position: Vec3::new(0.0, 0.5, 0.0),
                ..Transform3d::default()
            }),
            GlobalTransform::default(),
            opaque_cube,
        ));

        // C. 透明球体 (蓝色，半透明)
        let mut transparent_sphere = Model::from_primitive(
            MeshPrimitive::Sphere { radius: 0.8, subdivisions: 32 },
            &singletons.render_context,
            &mut texture_cache,
            &mut material_cache,
            &mut mesh_cache,
            &mut mesh_allocator,
        );
        if let Some(Some(mat_id)) = transparent_sphere.materials.get(0) {
            if let Some(mat) = material_cache.storage.get_mut(mat_id) {
                mat.base_color = [0.0, 0.5, 1.0, 0.4]; // 40% 不透明度
                mat.transparent = true;
                mat.alpha_mode = eureka::render::material::AlphaMode::Blend;
                mat.metallic = 0.8;
                mat.roughness = 0.1;
            }
        }
        world.ecs.spawn((
            Name("TransparentSphere".into()),
            CTransform3d(Transform3d {
                position: Vec3::new(0.0, 1.0, 2.0), // 放在红色方块前面
                ..Transform3d::default()
            }),
            GlobalTransform::default(),
            transparent_sphere,
        ));

        // D. 另一个透明立方体 (绿色，放在更后面，测试深度排序)
        let mut transparent_cube = Model::from_primitive(
            MeshPrimitive::Cube,
            &singletons.render_context,
            &mut texture_cache,
            &mut material_cache,
            &mut mesh_cache,
            &mut mesh_allocator,
        );
        if let Some(Some(mat_id)) = transparent_cube.materials.get(0) {
            if let Some(mat) = material_cache.storage.get_mut(mat_id) {
                mat.base_color = [0.0, 1.0, 0.2, 0.5];
                mat.transparent = true;
                mat.alpha_mode = eureka::render::material::AlphaMode::Blend;
            }
        }
        world.ecs.spawn((
            Name("TransparentCube".into()),
            CTransform3d(Transform3d {
                position: Vec3::new(0.0, 0.5, -2.0),
                ..Transform3d::default()
            }),
            GlobalTransform::default(),
            transparent_cube,
        ));
    });

    app.run();
}
