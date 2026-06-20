use crate::core::singleton::Singletons;
use crate::render::render_world::RenderWorld;
use crate::scene::components::*;
use crate::scene::d2::sprite2d::{SpriteAssetPending, SpriteComponent};
use crate::scene::d3::model::{AssetPending, Model};
use crate::scene::d3::sky::{SkyAssetPending, SkyComponent};
use hecs::World;

pub fn update_assets(ecs: &mut World, singletons: &mut Singletons, render_world: &mut RenderWorld) {
    // 1. 模型加载 (模型通常包含多个子资源，暂不实现路径级缓存，但使用 take 避免内存泄漏)
    let mut model_to_finalize = Vec::new();
    for (id, pending) in ecs.query_mut::<(hecs::Entity, &AssetPending)>() {
        singletons.asset_server.request_load(&pending.0);

        if let Some(raw) = singletons.asset_server.take_model(&pending.0) {
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
        let _ = ecs.remove_one::<AssetPending>(id);
        let _ = ecs.insert_one(id, model);
    }

    // 2. 天空盒加载
    let mut sky_to_finalize = Vec::new();
    for (id, pending) in ecs.query_mut::<(hecs::Entity, &SkyAssetPending)>() {
        // 先检查 GPU 缓存
        if let Some(texture_id) = render_world
            .imported_texture_cache
            .read()
            .unwrap()
            .get_by_path(&pending.0)
        {
            sky_to_finalize.push((id, Some(texture_id), None));
            continue;
        }

        singletons.asset_server.request_cubemap(&pending.0);
        if let Some(raw) = singletons.asset_server.take_cubemap(&pending.0) {
            sky_to_finalize.push((id, None, Some((pending.0.clone(), raw))));
        }
    }

    for (id, texture_id, raw_data) in sky_to_finalize {
        let mut sky = SkyComponent::empty();
        if let Some(tid) = texture_id {
            sky.finalize_with_id(tid);
        } else if let Some((path, raw)) = raw_data {
            sky.finalize(
                raw,
                &singletons.render_context,
                &mut render_world.imported_texture_cache.write().unwrap(),
                Some(path),
            );
        }
        let _ = ecs.remove_one::<SkyAssetPending>(id);
        let _ = ecs.insert_one(id, sky);
    }

    // 3. Sprite 加载
    let mut sprite_to_finalize = Vec::new();
    for (id, pending) in ecs.query_mut::<(hecs::Entity, &SpriteAssetPending)>() {
        // 先检查 GPU 缓存
        if let Some(texture_id) = render_world
            .imported_texture_cache
            .read()
            .unwrap()
            .get_by_path(&pending.0)
        {
            sprite_to_finalize.push((id, Some(texture_id), None));
            continue;
        }

        singletons.asset_server.request_texture(&pending.0);
        if let Some(raw) = singletons.asset_server.take_texture(&pending.0) {
            sprite_to_finalize.push((id, None, Some((pending.0.clone(), raw))));
        }
    }

    for (id, texture_id, raw_data) in sprite_to_finalize {
        if let Ok(mut sprite) = ecs.remove_one::<SpriteComponent>(id) {
            let size = if let Some(tid) = texture_id {
                sprite.finalize_with_id(tid, &render_world.imported_texture_cache.read().unwrap())
            } else if let Some((path, raw)) = raw_data {
                sprite.finalize(
                    raw,
                    &singletons.render_context,
                    &mut render_world.imported_texture_cache.write().unwrap(),
                    Some(path),
                )
            } else {
                unreachable!()
            };

            let _ = ecs.insert_one(id, sprite);
            let _ = ecs.remove_one::<SpriteAssetPending>(id);
            let _ = ecs.insert_one(id, Size(size));
        }
    }
}
