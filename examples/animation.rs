use eureka::animation::{AnimationClip, AnimationCurve, AnimationPlayer, Keyframe};
use eureka::core::App;
use eureka::math::transform::Transform2d;
use eureka::scene::{
    ActiveCamera, Camera2dComponent, GlobalTransform, LabelComponent, Name, Transform2dComponent,
};
use glam::Vec2;

fn main() {
    let mut app = App::new();

    app.setup(|app| {
        let world = &mut app.world;

        // 1. Add a 2D camera
        world.ecs.spawn((
            Name("MainCamera2D".into()),
            Transform2dComponent(Transform2d::default()),
            GlobalTransform::default(),
            Camera2dComponent::default(),
            ActiveCamera,
        ));

        // 2. Create a label to animate
        let label_id = world.ecs.spawn((
            Name("AnimatedLabel".into()),
            LabelComponent::new("Animated Text!"),
            Transform2dComponent(Transform2d {
                position: Vec2::new(640.0, 360.0),
                ..Transform2d::default()
            }),
            GlobalTransform::default(),
        ));

        // 3. Create animation clips
        let position_x_curve = AnimationCurve::new(vec![
            Keyframe::new(0.0, 500.0),
            Keyframe::new(1.0, 780.0),
            Keyframe::new(2.0, 500.0),
        ]);

        let position_y_curve = AnimationCurve::new(vec![
            Keyframe::new(0.0, 300.0),
            Keyframe::new(1.0, 420.0),
            Keyframe::new(2.0, 300.0),
        ]);

        let clip = AnimationClip::new("bounce".to_string())
            .add_curve("transform.position.x".to_string(), position_x_curve)
            .add_curve("transform.position.y".to_string(), position_y_curve);

        // 4. Create animation player
        let mut player = AnimationPlayer::new();
        player.add_clip(clip);

        // Bind to the label's properties (using Entity ID now)
        player.bind_to(label_id, "transform.position.x", "transform.position.x");
        player.bind_to(label_id, "transform.position.y", "transform.position.y");

        // Play the animation
        player.play("bounce", -1);

        // 5. Add the animation player to the world as a component
        // In ECS, we can add it to the same entity or a separate one.
        // Here we add it to its own entity for clarity.
        world.ecs.spawn((Name("AnimationPlayer".into()), player));

        // 6. Create a static label to show instructions
        world.ecs.spawn((
            Name("Instructions".into()),
            LabelComponent::new("Animation Demo: Text bounces using ECS systems"),
            Transform2dComponent(Transform2d {
                position: Vec2::new(10.0, 30.0),
                ..Transform2d::default()
            }),
            GlobalTransform::default(),
        ));

        println!("Animation Example (ECS):");
        println!("  - Label text will bounce with animation system");
        println!("  - AnimationPlayer is now a component");
    });

    app.run();
}
