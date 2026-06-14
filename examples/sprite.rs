use eureka::core::App;
use eureka::scene::{
    ActiveCamera, Camera2dComponent, GlobalTransform, Name,
    SpriteAssetPending, SpriteComponent, Transform2dComponent, Size, Parent,
};
use eureka::math::transform::Transform2d;
use glam::Vec2;

// 示例专用的逻辑组件
struct RotatingLogic;

fn main() {
    let mut app = App::new();

    app.setup(|app| {
        let world = &mut app.world;
        let asset_dir = app.singletons.as_ref().unwrap().asset_server.asset_dir.clone();

        // 1. 2D 摄像机
        world.ecs.spawn((
            Name("MainCamera2D".into()),
            Transform2dComponent(Transform2d::default()),
            GlobalTransform::default(),
            Camera2dComponent::default(),
            ActiveCamera,
        ));

        // 2. 演示父子 Transform 关系
        // 父节点 - 绕中心旋转的大圆
        let parent = world.ecs.spawn((
            Name("Parent_Circle".into()),
            Transform2dComponent(Transform2d {
                position: Vec2::new(640.0, 360.0), // 屏幕中心
                scale: Vec2::splat(3.0),
                ..Transform2d::default()
            }),
            GlobalTransform::default(),
            SpriteComponent::empty(),
            SpriteAssetPending(asset_dir.join("images/happy-tree.png")),
            RotatingLogic,
            Size(Vec2::new(100.0, 100.0)),
        ));

        // 子节点1 - 跟随父节点旋转的小圆 (在父节点内部)
        world.ecs.spawn((
            Name("Child_Circle_1".into()),
            Transform2dComponent(Transform2d {
                position: Vec2::new(150.0, 0.0), // 相对于父节点
                scale: Vec2::splat(0.8),
                ..Transform2d::default()
            }),
            GlobalTransform::default(),
            SpriteComponent::empty(),
            SpriteAssetPending(asset_dir.join("images/happy-tree.png")),
            Parent(parent), // 指定父节点
            Size(Vec2::new(80.0, 80.0)),
        ));

        // 子节点2 - 跟随父节点旋转的小圆
        world.ecs.spawn((
            Name("Child_Circle_2".into()),
            Transform2dComponent(Transform2d {
                position: Vec2::new(-150.0, 0.0), // 相对于父节点
                scale: Vec2::splat(0.8),
                ..Transform2d::default()
            }),
            GlobalTransform::default(),
            SpriteComponent::empty(),
            SpriteAssetPending(asset_dir.join("images/happy-tree.png")),
            Parent(parent), // 指定父节点
            Size(Vec2::new(80.0, 80.0)),
        ));

        // 3. 另一组父子关系 - 上下摆动的小精灵
        let swing_parent = world.ecs.spawn((
            Name("Swing_Parent".into()),
            Transform2dComponent(Transform2d {
                position: Vec2::new(200.0, 100.0),
                scale: Vec2::splat(1.5),
                ..Transform2d::default()
            }),
            GlobalTransform::default(),
            SpriteComponent::empty(),
            SpriteAssetPending(asset_dir.join("images/happy-tree.png")),
            Size(Vec2::new(60.0, 60.0)),
        ));

        // 挂在摆动父节点上的子节点
        world.ecs.spawn((
            Name("Swing_Child".into()),
            Transform2dComponent(Transform2d {
                position: Vec2::new(0.0, 120.0), // 相对于父节点，向下偏移
                scale: Vec2::splat(0.6),
                ..Transform2d::default()
            }),
            GlobalTransform::default(),
            SpriteComponent::empty(),
            SpriteAssetPending(asset_dir.join("images/happy-tree.png")),
            Parent(swing_parent), // 指定父节点
            Size(Vec2::new(50.0, 50.0)),
        ));
    });

    // 添加自定义更新逻辑
    app.add_update(|app, dt| {
        let world = &mut app.world;
        let time = app.singletons.as_ref().unwrap().time.get_delta() as f32;

        // 1. 让标记 RotatingLogic 的精灵旋转
        for (_id, transform) in world
            .ecs
            .query_mut::<&mut Transform2dComponent>()
            .with::<&RotatingLogic>()
        {
            transform.0.rotation += dt;
        }

        // 2. 让摆动父节点上下移动
        for (_id, transform) in world
            .ecs
            .query_mut::<&mut Transform2dComponent>()
            .with::<&Name>()
            .with::<&Parent>()
            .without::<&RotatingLogic>()
        {
            // 只处理标记为 Parent_Circle 相关的...这里简化处理
        }

        // 更精确地处理摆动
        let swing_parent_id = world.ecs.query::<&Name>()
            .iter()
            .find(|(_, name)| name.0 == "Swing_Parent")
            .map(|(id, _)| id);

        if let Some(id) = swing_parent_id {
            if let Ok(mut transform) = world.ecs.get::<&mut Transform2dComponent>(id) {
                transform.0.position.y = 100.0 + (time * 2.0).sin() * 80.0;
            }
        }
    });

    app.run();
}
