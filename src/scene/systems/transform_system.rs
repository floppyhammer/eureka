use crate::scene::components::*;
use hecs::{Entity, World};

pub fn propagate_transforms(ecs: &mut World) {
    // 1. 先更新所有 3D 根节点 (没有 Parent 的)
    for (local, global) in ecs
        .query_mut::<(&CTransform3d, &mut GlobalTransform)>()
        .without::<&Parent>()
    {
        global.0 = local.0.matrix();
    }

    // 2. 先更新所有 2D 根节点
    for (local, global) in ecs
        .query_mut::<(&CTransform2d, &mut GlobalTransform)>()
        .without::<&Parent>()
    {
        global.0 = local.0.to_mat4();
    }

    // 3. 处理子节点
    // 为了避免借用冲突，我们先收集所有带有 Parent 组件的实体 ID
    let child_entities: Vec<(Entity, Entity)> = ecs
        .query::<(hecs::Entity, &Parent)>()
        .iter()
        .map(|(id, p)| (id, p.0))
        .collect();

    for (child_id, parent_id) in child_entities {
        // 获取父节点的全局矩阵
        let parent_mat = if let Ok(parent_global) = ecs.get::<&GlobalTransform>(parent_id) {
            parent_global.0
        } else {
            continue;
        };

        // 获取子节点的局部矩阵
        let local_mat = if let Ok(t) = ecs.get::<&CTransform3d>(child_id) {
            t.0.matrix()
        } else if let Ok(t2d) = ecs.get::<&CTransform2d>(child_id) {
            t2d.0.to_mat4()
        } else {
            continue;
        };

        // 更新子节点的全局矩阵
        if let Ok(mut global) = ecs.get::<&mut GlobalTransform>(child_id) {
            global.0 = parent_mat * local_mat;
        }
    }
}
