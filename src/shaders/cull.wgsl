struct Camera {
    view_pos: vec4<f32>,
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    view_proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    ssao_enabled: u32,
}

struct MeshAabb {
    min: vec4<f32>,
    max: vec4<f32>,
}

struct Instance {
    model_0: vec4<f32>,
    model_1: vec4<f32>,
    model_2: vec4<f32>,
    model_3: vec4<f32>,
    normal_0: vec4<f32>,
    normal_1: vec4<f32>,
    normal_2: vec4<f32>,
    material_idx: u32,
}

struct DrawIndexedIndirect {
    index_count: u32,
    instance_count: atomic<u32>,
    first_index: u32,
    base_vertex: i32,
    first_instance: u32,
}

@group(0) @binding(0) var<uniform> camera: Camera;
@group(0) @binding(1) var<uniform> mesh_aabb: MeshAabb;
@group(0) @binding(2) var<storage, read> instances: array<Instance>;
@group(0) @binding(3) var<storage, read_write> visible_instances: array<Instance>;
@group(0) @binding(4) var<storage, read_write> indirect: DrawIndexedIndirect;

fn intersects_frustum(pos: vec3<f32>, radius: f32, planes: array<vec4<f32>, 6>) -> bool {
    for (var i = 0; i < 6; i = i + 1) {
        if (dot(planes[i], vec4<f32>(pos, 1.0)) < -radius) {
            return false;
        }
    }
    return true;
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let instance_idx = id.x;
    let total_instances = arrayLength(&instances);

    if (instance_idx >= total_instances) {
        return;
    }

    let instance = instances[instance_idx];
    let model = mat4x4<f32>(instance.model_0, instance.model_1, instance.model_2, instance.model_3);

    // 计算世界空间 AABB 的近似球体（简单处理）
    let center = (mesh_aabb.min.xyz + mesh_aabb.max.xyz) * 0.5;
    let world_center = (model * vec4<f32>(center, 1.0)).xyz;
    let size = (mesh_aabb.max.xyz - mesh_aabb.min.xyz) * 0.5;
    // 考虑缩放的最大半径
    let scale_x = length(instance.model_0.xyz);
    let scale_y = length(instance.model_1.xyz);
    let scale_z = length(instance.model_2.xyz);
    let radius = length(size) * max(scale_x, max(scale_y, scale_z));

    // 提取视锥体平面
    let m = camera.view_proj;
    var planes: array<vec4<f32>, 6>;
    planes[0] = vec4<f32>(m[0][3] + m[0][0], m[1][3] + m[1][0], m[2][3] + m[2][0], m[3][3] + m[3][0]); // Left
    planes[1] = vec4<f32>(m[0][3] - m[0][0], m[1][3] - m[1][0], m[2][3] - m[2][0], m[3][3] - m[3][0]); // Right
    planes[2] = vec4<f32>(m[0][3] + m[0][1], m[1][3] + m[1][1], m[2][3] + m[2][1], m[3][3] + m[3][1]); // Bottom
    planes[3] = vec4<f32>(m[0][3] - m[0][1], m[1][3] - m[1][1], m[2][3] - m[2][1], m[3][3] - m[3][1]); // Top
    planes[4] = vec4<f32>(m[0][3] + m[0][2], m[1][3] + m[1][2], m[2][3] + m[2][2], m[3][3] + m[3][2]); // Near
    planes[5] = vec4<f32>(m[0][3] - m[0][2], m[1][3] - m[1][2], m[2][3] - m[2][2], m[3][3] - m[3][2]); // Far

    for (var i = 0; i < 6; i = i + 1) {
        planes[i] = planes[i] / length(planes[i].xyz);
    }

    if (intersects_frustum(world_center, radius, planes)) {
        let out_idx = atomicAdd(&indirect.instance_count, 1u);
        visible_instances[out_idx] = instance;
    }
}
