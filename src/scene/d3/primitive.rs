use crate::math::aabb::Aabb;
use crate::math::transform::Transform3d;
use crate::render::vertex::Vertex3d;
use crate::scene::d3::model::{RawMaterialData, RawMeshData, RawModelData, AlphaMode};
use glam::Vec3;

pub enum MeshPrimitive {
    Cube,
    Plane { size: f32 },
    Sphere { radius: f32, subdivisions: u32 },
}

impl MeshPrimitive {
    pub fn generate_raw_data(&self) -> RawModelData {
        let (vertices, indices) = match self {
            MeshPrimitive::Cube => generate_cube_data(),
            MeshPrimitive::Plane { size } => generate_plane_data(*size),
            MeshPrimitive::Sphere { radius, subdivisions } => generate_sphere_data(*radius, *subdivisions),
        };

        let aabb = Aabb::from_vertices(&vertices);

        let mesh = RawMeshData {
            name: "PrimitiveMesh".to_string(),
            vertices,
            indices,
            material_index: Some(0),
            aabb,
            local_transform: Transform3d::default(),
        };

        let material = RawMaterialData {
            name: "DefaultPrimitiveMaterial".to_string(),
            base_color: [1.0, 1.0, 1.0, 1.0],
            emissive: [0.0, 0.0, 0.0],
            emissive_strength: 1.0,
            metallic: 0.0,
            roughness: 0.5,
            color_texture: None,
            normal_texture: None,
            metallic_roughness_texture: None,
            occlusion_texture: None,
            emissive_texture: None,
            transparent: false,
            alpha_cutoff: 0.5,
            alpha_mode: AlphaMode::Opaque,
        };

        RawModelData {
            meshes: vec![mesh],
            materials: vec![material],
            aabb,
        }
    }
}

fn generate_cube_data() -> (Vec<Vertex3d>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    let pos = [
        // Front
        [-0.5, -0.5, 0.5], [0.5, -0.5, 0.5], [0.5, 0.5, 0.5], [-0.5, 0.5, 0.5],
        // Back
        [-0.5, -0.5, -0.5], [-0.5, 0.5, -0.5], [0.5, 0.5, -0.5], [0.5, -0.5, -0.5],
        // Top
        [-0.5, 0.5, -0.5], [-0.5, 0.5, 0.5], [0.5, 0.5, 0.5], [0.5, 0.5, -0.5],
        // Bottom
        [-0.5, -0.5, -0.5], [0.5, -0.5, -0.5], [0.5, -0.5, 0.5], [-0.5, -0.5, 0.5],
        // Right
        [0.5, -0.5, -0.5], [0.5, 0.5, -0.5], [0.5, 0.5, 0.5], [0.5, -0.5, 0.5],
        // Left
        [-0.5, -0.5, -0.5], [-0.5, -0.5, 0.5], [-0.5, 0.5, 0.5], [-0.5, 0.5, -0.5],
    ];

    let normals = [
        [0.0, 0.0, 1.0], [0.0, 0.0, -1.0], [0.0, 1.0, 0.0],
        [0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [-1.0, 0.0, 0.0],
    ];

    let uvs = [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]];

    let tangents = [
        [1.0, 0.0, 0.0], [-1.0, 0.0, 0.0], [1.0, 0.0, 0.0],
        [1.0, 0.0, 0.0], [0.0, 0.0, -1.0], [0.0, 0.0, 1.0],
    ];

    for i in 0..6 {
        for j in 0..4 {
            let n = normals[i];
            let t = tangents[i];
            let b = Vec3::from_array(n).cross(Vec3::from_array(t)).to_array();
            vertices.push(Vertex3d {
                position: pos[i * 4 + j],
                uv: uvs[j],
                normal: n,
                tangent: t,
                bi_tangent: b,
            });
        }
        let base = (i * 4) as u32;
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    (vertices, indices)
}

fn generate_plane_data(size: f32) -> (Vec<Vertex3d>, Vec<u32>) {
    let half = size * 0.5;
    let vertices = vec![
        Vertex3d {
            position: [-half, 0.0, half],
            uv: [0.0, 1.0],
            normal: [0.0, 1.0, 0.0],
            tangent: [1.0, 0.0, 0.0],
            bi_tangent: [0.0, 0.0, -1.0],
        },
        Vertex3d {
            position: [half, 0.0, half],
            uv: [1.0, 1.0],
            normal: [0.0, 1.0, 0.0],
            tangent: [1.0, 0.0, 0.0],
            bi_tangent: [0.0, 0.0, -1.0],
        },
        Vertex3d {
            position: [half, 0.0, -half],
            uv: [1.0, 0.0],
            normal: [0.0, 1.0, 0.0],
            tangent: [1.0, 0.0, 0.0],
            bi_tangent: [0.0, 0.0, -1.0],
        },
        Vertex3d {
            position: [-half, 0.0, -half],
            uv: [0.0, 0.0],
            normal: [0.0, 1.0, 0.0],
            tangent: [1.0, 0.0, 0.0],
            bi_tangent: [0.0, 0.0, -1.0],
        },
    ];

    let indices = vec![0, 1, 2, 0, 2, 3];
    (vertices, indices)
}

fn generate_sphere_data(radius: f32, subdivisions: u32) -> (Vec<Vertex3d>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    let sectors = subdivisions * 2;
    let stacks = subdivisions;

    let sector_step = 2.0 * std::f32::consts::PI / sectors as f32;
    let stack_step = std::f32::consts::PI / stacks as f32;

    for i in 0..=stacks {
        let stack_angle = std::f32::consts::PI / 2.0 - i as f32 * stack_step;
        let xy = radius * stack_angle.cos();
        let z = radius * stack_angle.sin();

        for j in 0..=sectors {
            let sector_angle = j as f32 * sector_step;

            let x = xy * sector_angle.cos();
            let y = xy * sector_angle.sin();

            // Re-order to Y up
            let position = [x, z, y];
            let normal = [x / radius, z / radius, y / radius];
            let uv = [j as f32 / sectors as f32, i as f32 / stacks as f32];

            // Calculate tangent (derivative with respect to sector angle)
            let tx = -sector_angle.sin();
            let ty = sector_angle.cos();
            let tangent = [tx, 0.0, ty];

            let bitangent = Vec3::from_array(normal).cross(Vec3::from_array(tangent)).to_array();

            vertices.push(Vertex3d {
                position,
                uv,
                normal,
                tangent,
                bi_tangent: bitangent,
            });
        }
    }

    for i in 0..stacks {
        let mut k1 = i * (sectors + 1);
        let mut k2 = k1 + sectors + 1;

        for _j in 0..sectors {
            if i != 0 {
                indices.push(k1);
                indices.push(k2);
                indices.push(k1 + 1);
            }

            if i != (stacks - 1) {
                indices.push(k1 + 1);
                indices.push(k2);
                indices.push(k2 + 1);
            }

            k1 += 1;
            k2 += 1;
        }
    }

    (vertices, indices)
}
