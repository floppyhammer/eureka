use eureka::animation::{AnimationClip, AnimationCurve, AnimationPlayer, Interpolation, Keyframe};
use eureka::core::App;
use eureka::scene::{AsNode2d, Camera2d};
use eureka::scene::Label;
use glam::Vec2;

fn main() {
    let mut app = App::new();

    app.setup(|app| {
        let world = &mut app.world;

        // Add a 2D camera
        let camera = Camera2d::default();
        world.add_node(Box::new(camera), None);

        // Create a label to animate
        let mut label = Label::new("Animated Text!");
        label.set_position(Vec2::new(640.0, 360.0));
        let label_id = world.add_node(Box::new(label), None);

        // Create animation clip with position.x curve (left-right movement)
        let position_x_curve = AnimationCurve::new(vec![
            Keyframe::new(0.0, 500.0),
            Keyframe::new(1.0, 780.0),
            Keyframe::new(2.0, 500.0),
        ]);

        // Create animation clip with position.y curve (up-down movement)
        let position_y_curve = AnimationCurve::new(vec![
            Keyframe::new(0.0, 300.0),
            Keyframe::new(1.0, 420.0),
            Keyframe::new(2.0, 300.0),
        ]);

        let clip = AnimationClip::new("bounce".to_string())
            .add_curve("transform.position.x".to_string(), position_x_curve)
            .add_curve("transform.position.y".to_string(), position_y_curve);

        // Create a smooth version with Catmull-Rom interpolation
        let smooth_position_x_curve = AnimationCurve::new(vec![
            Keyframe::new(0.0, 500.0).with_interpolation(Interpolation::Smooth),
            Keyframe::new(1.0, 780.0).with_interpolation(Interpolation::Smooth),
            Keyframe::new(2.0, 500.0).with_interpolation(Interpolation::Smooth),
        ]);

        let smooth_position_y_curve = AnimationCurve::new(vec![
            Keyframe::new(0.0, 300.0).with_interpolation(Interpolation::Smooth),
            Keyframe::new(1.0, 420.0).with_interpolation(Interpolation::Smooth),
            Keyframe::new(2.0, 300.0).with_interpolation(Interpolation::Smooth),
        ]);

        let smooth_clip = AnimationClip::new("smooth_bounce".to_string())
            .add_curve("transform.position.x".to_string(), smooth_position_x_curve)
            .add_curve("transform.position.y".to_string(), smooth_position_y_curve);

        // Create animation player
        let mut player = AnimationPlayer::new();
        player.add_clip(clip);
        player.add_clip(smooth_clip);

        // Bind to the label's properties
        player.bind_to(label_id, "transform.position.x", "transform.position.x");
        player.bind_to(label_id, "transform.position.y", "transform.position.y");

        // Play the animation (loop forever)
        player.play("bounce", -1);

        // Add the animation player to the world
        let _player_id = world.add_node(Box::new(player), None);

        // Create a static label to show instructions
        let mut instructions = Label::new("Animation Demo: Text bounces smoothly");
        instructions.set_position(Vec2::new(10.0, 30.0));
        let _instructions_id = world.add_node(Box::new(instructions), None);

        println!("Animation Example:");
        println!("  - Label text will bounce with smooth animation");
        println!("  - Animation is looped and runs automatically");
    });

    app.run();
}
