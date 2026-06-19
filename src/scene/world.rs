use crate::core::singleton::Singletons;
use crate::math::transform::{Transform2d, Transform3d};
use crate::render::render_world::Extracted;
use crate::scene::components::*;
use crate::scene::{Camera2dComponent, Camera3dComponent, PointLightComponent};
use crate::window::InputServer;
use glam::{UVec2, Vec3};
use hecs::{Entity, World as EcsWorld};

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
        singletons.asset_manager.update();

        // 1. 资产加载系统
        self.update_assets(singletons, render_world);

        // 2. 摄像机同步系统
        self.update_cameras(singletons);

        // 3. 动画系统
        self.update_animations(dt);

        // 4. 示例中的自定义逻辑系统
        self.update_example_logic(dt);

        // 5. 变换传播系统
        self.propagate_transforms();

        // 6. Label 系统
        self.update_labels(singletons);
    }

    fn update_cameras(&mut self, singletons: &Singletons) {
        let width = singletons.render_context.surface_config.width as f32;
        let height = singletons.render_context.surface_config.height as f32;

        if width <= 0.0 || height <= 0.0 {
            return;
        }

        // 同步 2D 摄像机投影
        for (_id, camera) in self.ecs.query_mut::<&mut Camera2dComponent>() {
            camera.viewport_size = UVec2::new(width as u32, height as u32);
        }

        // 同步 3D 摄像机视口
        for (_id, (camera, global)) in self.ecs.query_mut::<(&mut Camera3dComponent, &GlobalTransform)>() {
            camera.viewport_size = UVec2::new(width as u32, height as u32);
            camera.frame_count = camera.frame_count.wrapping_add(1);

            // 注意：这里不再立即更新 camera.prev_view_proj
            // 我们在渲染提取逻辑中先使用它，然后再在每帧结束时更新它
        }
    }

    fn update_example_logic(&mut self, dt: f32) {
        // 摄像机移动系统
        for (_id, (transform, controller)) in self.ecs.query_mut::<(
            &mut CTransform3d,
            &mut crate::scene::d3::camera3d::Camera3dController,
        )>() {
            transform.0.rotation =
                glam::Quat::from_euler(glam::EulerRot::ZYX, 0.0, controller.yaw, controller.pitch);

            let forward = transform.0.rotation * glam::Vec3::NEG_Z;
            let right = transform.0.rotation * glam::Vec3::X;

            transform.0.position += forward
                * (controller.amount_forward - controller.amount_backward)
                * controller.speed
                * dt;
            transform.0.position +=
                right * (controller.amount_right - controller.amount_left) * controller.speed * dt;
            transform.0.position.y +=
                (controller.amount_up - controller.amount_down) * controller.speed * dt;
        }
    }

    fn update_labels(&mut self, singletons: &mut Singletons) {
        use crate::math::transform::Transform2d;
        use crate::scene::d2::label::LabelComponent;

        for (_id, (label, global)) in self
            .ecs
            .query_mut::<(&mut LabelComponent, &GlobalTransform)>()
        {
            // 确保字体被请求
            if let Some(font_id) = &label.font_id {
                singletons.asset_manager.request_font(font_id);
            }

            let (_, rotation, translation) = global.0.to_scale_rotation_translation();
            let rotation_z = rotation.to_euler(glam::EulerRot::XYZ).2;
            let current_global_transform = Transform2d {
                position: translation.truncate(),
                rotation: rotation_z,
                scale: glam::Vec2::ONE,
            };

            let transform_changed = (current_global_transform.position
                - label.last_global_transform.position)
                .length_squared()
                > 0.0001
                || (current_global_transform.rotation - label.last_global_transform.rotation).abs()
                    > 0.0001;

            if label.text_is_dirty
                || label.atlas.as_ref().map_or(true, |a| a.texture.is_none())
                || transform_changed
            {
                let atlas = singletons.text_server.get_atlas(
                    label.text.as_str(),
                    label.font_id.clone(),
                    current_global_transform,
                    label.leading,
                );

                if atlas.texture.is_some() {
                    label.text_is_dirty = false;
                    label.last_global_transform = current_global_transform;
                }
                label.atlas = Some(atlas);
            }
        }
    }

    fn update_animations(&mut self, dt: f32) {
        use crate::animation::player::AnimationPlayer;
        use crate::animation::property::PropertyValue;
        use glam::FloatExt;

        let mut all_changes = Vec::new();

        // 收集所有动画播放器的变更
        for (_id, player) in self.ecs.query_mut::<&mut AnimationPlayer>() {
            player.update(dt);
            all_changes.extend(player.take_changes());
        }

        // 应用变更
        for change in all_changes {
            let path = change.property_path.path();

            // 尝试应用到 3D Transform
            if let Ok(mut transform) = self.ecs.get::<&mut CTransform3d>(change.target_entity) {
                match (path, &change.value) {
                    ("transform.position", PropertyValue::Vec3(v)) => {
                        transform.0.position = transform.0.position.lerp(*v, change.weight)
                    }
                    ("transform.position.x", PropertyValue::Float(v)) => {
                        transform.0.position.x = transform.0.position.x.lerp(*v, change.weight)
                    }
                    ("transform.position.y", PropertyValue::Float(v)) => {
                        transform.0.position.y = transform.0.position.y.lerp(*v, change.weight)
                    }
                    ("transform.position.z", PropertyValue::Float(v)) => {
                        transform.0.position.z = transform.0.position.z.lerp(*v, change.weight)
                    }
                    ("transform.rotation", PropertyValue::Quat(q)) => {
                        transform.0.rotation = transform.0.rotation.slerp(*q, change.weight)
                    }
                    ("transform.scale", PropertyValue::Vec3(v)) => {
                        transform.0.scale = transform.0.scale.lerp(*v, change.weight)
                    }
                    _ => {}
                }
            }

            // 尝试应用到 2D Transform
            if let Ok(mut transform) = self.ecs.get::<&mut CTransform2d>(change.target_entity) {
                match (path, &change.value) {
                    ("transform.position", PropertyValue::Vec2(v)) => {
                        transform.0.position = transform.0.position.lerp(*v, change.weight)
                    }
                    ("transform.position.x", PropertyValue::Float(v)) => {
                        transform.0.position.x = transform.0.position.x.lerp(*v, change.weight)
                    }
                    ("transform.position.y", PropertyValue::Float(v)) => {
                        transform.0.position.y = transform.0.position.y.lerp(*v, change.weight)
                    }
                    ("transform.rotation", PropertyValue::Float(v)) => {
                        transform.0.rotation = transform.0.rotation.lerp(*v, change.weight)
                    }
                    ("transform.scale", PropertyValue::Vec2(v)) => {
                        transform.0.scale = transform.0.scale.lerp(*v, change.weight)
                    }
                    _ => {}
                }
            }
        }
    }

    fn update_assets(
        &mut self,
        singletons: &mut Singletons,
        render_world: &mut crate::render::render_world::RenderWorld,
    ) {
        use crate::scene::d2::sprite2d::{SpriteAssetPending, SpriteComponent};
        use crate::scene::d3::model::{AssetPending, Model};
        use crate::scene::d3::sky::{SkyAssetPending, SkyComponent};

        // 1. 模型加载
        let mut model_to_finalize = Vec::new();
        for (id, pending) in self.ecs.query_mut::<&AssetPending>() {
            // 只请求一次加载（AssetManager 通常会处理重复请求，但这里主动控制更安全）
            singletons.asset_manager.request_load(&pending.0);

            if let Some(raw) = singletons.asset_manager.loaded_raw_models.remove(&pending.0) {
                model_to_finalize.push((id, raw));
            }
        }

        for (id, raw) in model_to_finalize {
            let mut model = Model::empty();
            model.finalize(
                raw,
                &singletons.render_context,
                &mut render_world.imported_texture_cache.write().unwrap(),
                &mut render_world.imported_material_cache.write().unwrap(),
                &mut render_world.imported_mesh_cache.write().unwrap(),
                &mut render_world.imported_mesh_allocator.write().unwrap(),
            );
            // 务必移除 AssetPending，否则下一帧还会进这个循环
            let _ = self.ecs.remove_one::<AssetPending>(id);
            let _ = self.ecs.insert_one(id, model);
        }

        // 2. 天空盒加载
        let mut sky_to_finalize = Vec::new();
        for (id, pending) in self.ecs.query_mut::<&SkyAssetPending>() {
            singletons.asset_manager.request_cubemap(&pending.0);
            if let Some(raw) = singletons.asset_manager.loaded_raw_cubemaps.get(&pending.0) {
                sky_to_finalize.push((id, raw.clone()));
            }
        }

        for (id, raw) in sky_to_finalize {
            let mut sky = SkyComponent::empty();
            sky.finalize(
                raw,
                &singletons.render_context,
                &mut render_world.imported_texture_cache.write().unwrap(),
            );
            self.ecs.remove_one::<SkyAssetPending>(id).unwrap();
            self.ecs.insert_one(id, sky).unwrap();
        }

        // 3. Sprite 加载
        let mut sprite_to_finalize = Vec::new();
        for (id, pending) in self.ecs.query_mut::<&SpriteAssetPending>() {
            singletons.asset_manager.request_texture(&pending.0);
            if let Some(raw) = singletons.asset_manager.loaded_raw_textures.get(&pending.0) {
                sprite_to_finalize.push((id, raw.clone()));
            }
        }

        for (id, raw) in sprite_to_finalize {
            // 获取现有的组件，保留其配置（如颜色、对齐等）
            let size = if let Ok(mut sprite) = self.ecs.remove_one::<SpriteComponent>(id) {
                // 我们直接在原地 finalize，这样用户在 spawn 时设置的属性会被保留
                let size = sprite.finalize(
                    raw,
                    &singletons.render_context,
                    &mut render_world.imported_texture_cache.write().unwrap(),
                );
                // 将修改后的组件插回去
                let _ = self.ecs.insert_one(id, sprite);
                Some(size)
            } else {
                None
            };

            // 移除 Pending 标记
            let _ = self.ecs.remove_one::<SpriteAssetPending>(id);

            // 补充 Size 组件，供渲染系统提取
            if let Some(s) = size {
                let _ = self.ecs.insert_one(id, Size(s));
            }
        }
    }

    /// 计算父子层级的全局变换
    fn propagate_transforms(&mut self) {
        // 1. 先更新所有 3D 根节点 (没有 Parent 的)
        for (_id, (local, global)) in self
            .ecs
            .query_mut::<(&CTransform3d, &mut GlobalTransform)>()
            .without::<&Parent>()
        {
            global.0 = local.0.matrix();
        }

        // 2. 先更新所有 2D 根节点
        for (_id, (local, global)) in self
            .ecs
            .query_mut::<(&CTransform2d, &mut GlobalTransform)>()
            .without::<&Parent>()
        {
            global.0 = local.0.to_mat4();
        }

        // 3. 处理子节点
        // 为了避免借用冲突，我们先收集所有带有 Parent 组件的实体 ID
        let child_entities: Vec<(Entity, Entity)> = self
            .ecs
            .query::<&Parent>()
            .iter()
            .map(|(id, p)| (id, p.0))
            .collect();

        for (child_id, parent_id) in child_entities {
            // 获取父节点的全局矩阵
            let parent_mat = if let Ok(parent_global) = self.ecs.get::<&GlobalTransform>(parent_id)
            {
                parent_global.0
            } else {
                continue;
            };

            // 获取子节点的局部矩阵
            let local_mat = if let Ok(t) = self.ecs.get::<&CTransform3d>(child_id) {
                t.0.matrix()
            } else if let Ok(t2d) = self.ecs.get::<&CTransform2d>(child_id) {
                t2d.0.to_mat4()
            } else {
                continue;
            };

            // 更新子节点的全局矩阵
            if let Ok(mut global) = self.ecs.get::<&mut GlobalTransform>(child_id) {
                global.0 = parent_mat * local_mat;
            }
        }
    }

    /// 渲染提取系统：从 ECS 中提取渲染命令
    pub fn extract_render_objects(&mut self) -> Extracted {
        let mut extracted = Extracted::default();

        // 提取渲染设置 (从摄像机组件获取)
        for (_id, camera) in self
            .ecs
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
        self.extract_cameras(&mut extracted);

        // 提取天空盒
        for (_id, sky) in self.ecs.query::<&crate::scene::d3::SkyComponent>().iter() {
            if let Some(texture) = sky.texture {
                extracted.sky = Some(crate::render::sky::ExtractedSky { texture });
            }
        }

        // 2. 提取点光源
        for (_id, (light, global)) in self
            .ecs
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
        for (_id, (light, global)) in self
            .ecs
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
        for (_id, (sprite, global, size)) in self
            .ecs
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
        for (_id, label) in self
            .ecs
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
        for (_id, (model, global)) in self
            .ecs
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

    fn extract_cameras(&mut self, extracted: &mut Extracted) {
        use crate::render::camera::CameraType;

        // 提取 3D 摄像机
        for (_id, (camera, global, _)) in self
            .ecs
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
        for (_id, (camera, global, _)) in self
            .ecs
            .query_mut::<(
                &mut crate::scene::d2::Camera2dComponent,
                &GlobalTransform,
                &ActiveCamera,
            )>()
        {
            let uniform = camera.build_uniform(&global.0);
            extracted.cameras.add(CameraType::D2, uniform);
        }

        // 提取 2D 摄像机
        for (_id, (camera, global, _)) in self
            .ecs
            .query::<(
                &crate::scene::d2::Camera2dComponent,
                &GlobalTransform,
                &ActiveCamera,
            )>()
            .iter()
        {
            let uniform = camera.build_uniform(&global.0);
            extracted.cameras.add(CameraType::D2, uniform);
        }
    }

    pub fn input(&mut self, input_server: &mut InputServer) {
        // 摄像机控制器输入处理
        for event in input_server.input_events.clone() {
            for (_id, controller) in self
                .ecs
                .query_mut::<&mut crate::scene::d3::camera3d::Camera3dController>()
            {
                controller.handle_input(&event, input_server);
            }
        }
    }
}
