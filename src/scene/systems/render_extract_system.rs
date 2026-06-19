use crate::render::render_world::Extracted;
use crate::scene::components::*;
use crate::scene::{ActiveCamera, Camera2dComponent, Camera3dComponent, PointLightComponent};
use glam::Vec3;
use hecs::World;

pub fn extract_render_objects(ecs: &mut World) -> Extracted {
    let mut extracted = Extracted::default();

    // 提取渲染设置 (从摄像机组件获取)
    for (_id, camera) in ecs
        .query::<&crate::scene::d3::camera3d::Camera3dComponent>()
        .iter()
    {
        extracted.fxaa_enabled = camera.fxaa_enabled;
        extracted.ssao_enabled = camera.ssao_enabled;
        extracted.taa_enabled = camera.taa_enabled;
        // 只取第一个摄像机的设置
        break;
    }

    // 1. 提取摄像机
    extract_cameras(ecs, &mut extracted);

    // 提取天空盒
    for (_id, sky) in ecs.query::<&crate::scene::d3::SkyComponent>().iter() {
        if let Some(texture) = sky.texture {
            extracted.sky = Some(crate::render::sky::ExtractedSky { texture });
        }
    }

    // 2. 提取点光源
    for (_id, (light, global)) in ecs
        .query::<(&PointLightComponent, &GlobalTransform)>()
        .iter()
    {
        let position = global.0.transform_point3(Vec3::ZERO);
        extracted
            .lights
            .point_lights
            .push(crate::render::light::PointLightUniform {
                position: position.into(),
                strength: light.strength,
                color: light.color.to_vec3().into(),
                radius: light.radius,
                shadow_near: light.shadow_near,
                shadow_far: light.shadow_far,
                _pad: [0.0; 2],
            });
    }

    // 提取方向光
    for (_id, (light, global)) in ecs
        .query::<(
            &crate::scene::d3::DirectionalLightComponent,
            &GlobalTransform,
        )>()
        .iter()
    {
        let (_, rotation, _) = global.0.to_scale_rotation_translation();
        let direction = rotation * Vec3::NEG_Z;

        extracted.lights.directional_light =
            Some(crate::render::light::DirectionalLightUniform {
                direction: direction.to_array(),
                strength: light.strength,
                color: light.color.to_vec3().into(),
                shadow_distance: light.shadow_distance,
            });
    }

    // 提取 2D Sprite
    for (_id, (sprite, global, size)) in ecs
        .query::<(
            &crate::scene::d2::sprite2d::SpriteComponent,
            &GlobalTransform,
            &Size,
        )>()
        .iter()
    {
        if let Some(texture_id) = sprite.texture {
            use crate::math::transform::Transform2d;
            use crate::render::sprite::ExtractedSprite2d;

            let (scale, rotation, translation) = global.0.to_scale_rotation_translation();
            let rotation_z = rotation.to_euler(glam::EulerRot::XYZ).2;

            extracted.sprites.push(ExtractedSprite2d {
                transform: Transform2d {
                    position: translation.truncate(),
                    rotation: rotation_z,
                    scale: scale.truncate(),
                },
                color: sprite.color,
                rect: sprite.region,
                size: size.0,
                texture_id,
                centered: sprite.centered,
                flip_x: sprite.flip_x,
                flip_y: sprite.flip_y,
                mode: 0,
            });
        }
    }

    // 提取 Label
    for (_id, label) in ecs
        .query::<&crate::scene::d2::label::LabelComponent>()
        .iter()
    {
        if let Some(atlas) = &label.atlas {
            if let Some(texture_id) = atlas.texture {
                use crate::math::transform::Transform2d;
                use crate::render::sprite::ExtractedSprite2d;
                use glam::Vec2;

                for instance in &atlas.instances {
                    let tl_pos =
                        Vec2::new(instance.position.x, instance.position.y - instance.size.y);
                    extracted.sprites.push(ExtractedSprite2d {
                        transform: Transform2d {
                            position: tl_pos,
                            rotation: 0.0,
                            scale: Vec2::ONE,
                        },
                        color: instance.color.into(),
                        rect: instance.region,
                        size: instance.size,
                        texture_id,
                        centered: false,
                        flip_x: false,
                        flip_y: false,
                        mode: 1,
                    });
                }
            }
        }
    }

    // 3. 提取 3D 模型
    for (_id, (model, global)) in ecs
        .query::<(&crate::scene::d3::Model, &GlobalTransform)>()
        .iter()
    {
        use crate::math::transform::Transform3d;
        use crate::render::ExtractedMesh;

        let (scale, rotation, translation) = global.0.to_scale_rotation_translation();
        let global_transform = Transform3d {
            position: translation,
            rotation,
            scale,
        };

        for i in 0..model.meshes.len() {
            let local_mesh_transform = model.mesh_transforms[i];
            let combined = global_transform.combine(&local_mesh_transform);

            extracted.meshes.push(ExtractedMesh {
                transform: combined,
                mesh_id: model.meshes[i],
                material_id: model.materials[i],
                transparent: model.mesh_transparency[i],
            });
        }
    }

    extracted
}

fn extract_cameras(ecs: &mut World, extracted: &mut Extracted) {
    use crate::render::camera::CameraType;

    // 提取 3D 摄像机
    for (_id, (camera, global, _)) in ecs
        .query_mut::<(
            &mut crate::scene::d3::camera3d::Camera3dComponent,
            &GlobalTransform,
            &ActiveCamera,
        )>()
    {
        let uniform = camera.build_uniform(&global.0);
        extracted.cameras.add(CameraType::D3, uniform);

        // 提取完成后，更新组件内的历史矩阵，供下一帧 build_uniform 使用
        camera.update_after_extract(&global.0);
    }

    // 提取 2D 摄像机
    for (_id, (camera, global, _)) in ecs
        .query_mut::<(
            &mut crate::scene::d2::Camera2dComponent,
            &GlobalTransform,
            &ActiveCamera,
        )>()
    {
        let uniform = camera.build_uniform(&global.0);
        extracted.cameras.add(CameraType::D2, uniform);
    }
}
