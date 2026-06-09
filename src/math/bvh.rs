use crate::math::aabb::Aabb;
use crate::math::frustum::Frustum;

#[derive(Clone)]
pub enum BvhNode {
    Internal {
        aabb: Aabb,
        left: Box<BvhNode>,
        right: Box<BvhNode>,
    },
    Leaf {
        aabb: Aabb,
        object_indices: Vec<usize>,
    },
}

impl BvhNode {
    pub fn aabb(&self) -> &Aabb {
        match self {
            BvhNode::Internal { aabb, .. } => aabb,
            BvhNode::Leaf { aabb, .. } => aabb,
        }
    }
}

#[derive(Clone, Default)]
pub struct Bvh {
    pub root: Option<BvhNode>,
}

impl Bvh {
    pub fn build(objects: Vec<(Aabb, usize)>) -> Self {
        if objects.is_empty() {
            return Self { root: None };
        }
        let root = Self::build_recursive(objects);
        Self { root: Some(root) }
    }

    fn build_recursive(mut objects: Vec<(Aabb, usize)>) -> BvhNode {
        let mut aabb = objects[0].0;
        for i in 1..objects.len() {
            aabb = aabb.union(&objects[i].0);
        }

        if objects.len() <= 2 {
            return BvhNode::Leaf {
                aabb,
                object_indices: objects.into_iter().map(|(_, idx)| idx).collect(),
            };
        }

        let size = aabb.max - aabb.min;
        let axis = if size.x > size.y && size.x > size.z {
            0
        } else if size.y > size.z {
            1
        } else {
            2
        };

        objects.sort_by(|a, b| {
            a.0.center()[axis]
                .partial_cmp(&b.0.center()[axis])
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let mid = objects.len() / 2;
        let right_objects = objects.split_off(mid);
        let left_objects = objects;

        BvhNode::Internal {
            aabb,
            left: Box::new(Self::build_recursive(left_objects)),
            right: Box::new(Self::build_recursive(right_objects)),
        }
    }

    pub fn query(&self, frustum: &Frustum, result: &mut Vec<usize>) {
        if let Some(root) = &self.root {
            Self::query_recursive(root, frustum, result);
        }
    }

    fn query_recursive(node: &BvhNode, frustum: &Frustum, result: &mut Vec<usize>) {
        if !frustum.intersects_aabb(node.aabb()) {
            return;
        }

        match node {
            BvhNode::Internal { left, right, .. } => {
                Self::query_recursive(left, frustum, result);
                Self::query_recursive(right, frustum, result);
            }
            BvhNode::Leaf { object_indices, .. } => {
                result.extend(object_indices);
            }
        }
    }
}
