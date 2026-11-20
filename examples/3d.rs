use eureka::core::App;
use eureka::math::color::ColorU;
use eureka::math::transform::{Transform2d, Transform3d};
use eureka::scene::{
    ActiveCamera, AssetPending, CTransform2d, CTransform3d, Camera3dComponent, Camera3dController,
    DirectionalLightComponent, GlobalTransform, LabelComponent, Name, PointLightComponent,
    SkyAssetPending,
};
use glam::{Quat, Vec2, Vec3};

// 示例专用的逻辑组件
struct RotatingLogic;
struct FloatingLogic {
    speed: f32,
    timer: f32,
}
struct SunLogic {
    timer: f32,
    speed: f32,
}

fn main() {
    let mut app = App::new();

    app.setup(|app| {
        let singletons = app.singletons.as_mut().unwrap();
        let world = &mut app.world;

        // 1. 添加 3D 摄像机和控制器
        let mut controller = Camera3dController::new(4.0, 0.003);
        controller.yaw = -90.0f32.to_radians();

        world.ecs.spawn((
            Name("MainCamera".into()),
            CTransform3d(Transform3d {
                position: Vec3::new(-5.0, 2.0, 0.0),
                ..Transform3d::default()
            }),
            GlobalTransform::default(),
            Camera3dComponent::new(),
            ActiveCamera,
            controller,
        ));

        // 2. 添加 UI 覆盖层 (2D 摄像机)
        world.ecs.spawn((
            Name("UICamera".into()),
            CTransform2d(Transform2d::default()),
            GlobalTransform::default(),
            eureka::scene::d2::camera2d::Camera2dComponent::default(),
            ActiveCamera,
        ));

        // 3. 添加标签实体
        world.ecs.spawn((
            Name("Settings".into()),
            LabelComponent::new("Text Overlay"),
            CTransform2d(Transform2d {
                position: Vec2::new(20.0, 20.0),
                ..Transform2d::default()
            }),
            GlobalTransform::default(),
        ));

        // 4. 添加环境
        let skybox_path = singletons.asset_server.asset_dir.join("images/Panorama_Sky_21-512x512.png");
        world
            .ecs
            .spawn((Name("Skybox".into()), SkyAssetPending(skybox_path)));

        // 5. 灯光
        world.ecs.spawn((
            "PointLight",
            CTransform3d(Transform3d {
                position: Vec3::new(0.0, 5.0, 0.0),
                ..Transform3d::default()
            }),
            GlobalTransform::default(),
            PointLightComponent {
                strength: 10.0,
                radius: 10.0,
                color: ColorU::new(255, 255, 255, 255),
                ..PointLightComponent::default()
            },
        ));

        world.ecs.spawn((
            Name("DirLight".into()),
            CTransform3d(Transform3d {
                rotation: Quat::from_rotation_x(-135.0f32.to_radians()),
                ..Transform3d::default()
            }),
            GlobalTransform::default(),
            DirectionalLightComponent {
                strength: 1.5,
                ..DirectionalLightComponent::default()
            },
            SunLogic {
                timer: 0.0,
                speed: 1.0, // 调整这个值控制日夜交替速度
            },
        ));

        // 6. 模型
        let asset_dir = singletons.asset_server.asset_dir.clone();

        // 螃蟹 (漂浮)
        world.ecs.spawn((
            Name("Ferris".into()),
            CTransform3d(Transform3d {
                position: Vec3::new(4.0, 0.1, 0.0),
                ..Transform3d::default()
            }),
            GlobalTransform::default(),
            AssetPending(asset_dir.join("models/ferris/ferris3d_v1.0.obj")),
            FloatingLogic {
                speed: 1.0,
                timer: 0.0,
            },
        ));

        // 旋转立方体
        world.ecs.spawn((
            Name("Cube".into()),
            CTransform3d(Transform3d {
                position: Vec3::new(2.0, 1.2, 2.0),
                scale: Vec3::splat(0.5),
                ..Transform3d::default()
            }),
            GlobalTransform::default(),
            AssetPending(asset_dir.join("models/cube/cube.obj")),
            RotatingLogic,
        ));

        // 金属球 (MetalRoughSpheres)
        world.ecs.spawn((
            Name("Spheres".into()),
            CTransform3d(Transform3d {
                position: Vec3::new(0.0, 1.0, -0.5),
                scale: Vec3::splat(0.1),
                rotation: Quat::from_rotation_z(90.0f32.to_radians()),
            }),
            GlobalTransform::default(),
            AssetPending(asset_dir.join("models/MetalRoughSpheres.gltf")),
        ));

        // 地面
        world.ecs.spawn((
            Name("Ground".to_string()),
            Transform3d::default(),
            GlobalTransform::default(),
            AssetPending(asset_dir.join("models/ground.glb")), // "models/Sponza/Sponza.gltf"
        ));

        // 镜面立方体 (Mirror Cube)
        world.ecs.spawn((
            Name("MirrorCube".into()),
            CTransform3d(Transform3d {
                position: Vec3::new(-2.0, 1.5, 0.0),
                ..Transform3d::default()
            }),
            GlobalTransform::default(),
            AssetPending(asset_dir.join("models/mirror_cube/gltf/mirror_cube.gltf")),
        ));
    });

    // 添加自定义更新逻辑：3D 旋转
    app.add_update(|app, dt| {
        let world = &mut app.world;
        for transform in world
            .ecs
            .query_mut::<&mut CTransform3d>()
            .with::<&RotatingLogic>()
        {
            transform.0.rotation *= Quat::from_rotation_y(dt);
        }
    });

    // 添加自定义更新逻辑：漂浮
    app.add_update(|app, dt| {
        let world = &mut app.world;
        for (transform, logic) in world
            .ecs
            .query_mut::<(&mut CTransform3d, &mut FloatingLogic)>()
        {
            logic.timer += dt * logic.speed;
            transform.0.position.y = 1.0 + logic.timer.sin() * 1.0;
        }
    });

    // 添加自定义更新逻辑：太阳（方向光）旋转
    app.add_update(|app, dt| {
        let world = &mut app.world;
        for (transform, _light, logic) in world.ecs.query_mut::<(
            &mut CTransform3d,
            &DirectionalLightComponent,
            &mut SunLogic,
        )>() {
            logic.timer += dt * logic.speed;

            // 让太阳绕 X 轴旋转（模拟东升西落）
            transform.0.rotation = Quat::from_rotation_x(logic.timer);
        }
    });

    app.run();
}
