struct Camera {
    view_pos: vec4<f32>,
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    view_proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    ssao_enabled: u32,
}

struct MeshMetadata {
    aabb_min: vec4<f32>,
    aabb_max: vec4<f32>,
    base_instance: u32,
    instance_count: u32,
    _pad0: u32,
    _pad1: u32,
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
@group(0) @binding(1) var<storage, read> mesh_metadatas: array<MeshMetadata>;
@group(0) @binding(2) var<storage, read> instances: array<Instance>;
@group(0) @binding(3) var<storage, read_write> visible_instances: array<Instance>;
@group(0) @binding(4) var<storage, read_write> indirect_commands: array<DrawIndexedIndirect>;

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
    let global_instance_idx = id.x;
    let total_instances = arrayLength(&instances);

    if (global_instance_idx >= total_instances) {
        return;
    }

    var mesh_idx = 0u;
    let num_meshes = arrayLength(&mesh_metadatas);
    for (var i = 0u; i < num_meshes; i = i + 1u) {
        let m_meta = mesh_metadatas[i];
        if (global_instance_idx >= m_meta.base_instance && global_instance_idx < m_meta.base_instance + m_meta.instance_count) {
            mesh_idx = i;
            break;
        }
    }

    let m_meta = mesh_metadatas[mesh_idx];
    let instance = instances[global_instance_idx];
    let model = mat4x4<f32>(instance.model_0, instance.model_1, instance.model_2, instance.model_3);

    let center = (m_meta.aabb_min.xyz + m_meta.aabb_max.xyz) * 0.5;
    let world_center = (model * vec4<f32>(center, 1.0)).xyz;
    let size = (m_meta.aabb_max.xyz - m_meta.aabb_min.xyz) * 0.5;

    let scale_x = length(instance.model_0.xyz);
    let scale_y = length(instance.model_1.xyz);
    let scale_z = length(instance.model_2.xyz);
    let radius = length(size) * max(scale_x, max(scale_y, scale_z));

    let m = camera.view_proj;
    var planes: array<vec4<f32>, 6>;
    planes[0] = vec4<f32>(m[0][3] + m[0][0], m[1][3] + m[1][0], m[2][3] + m[2][0], m[3][3] + m[3][0]);
    planes[1] = vec4<f32>(m[0][3] - m[0][0], m[1][3] - m[1][0], m[2][3] - m[2][0], m[3][3] - m[3][0]);
    planes[2] = vec4<f32>(m[0][3] + m[0][1], m[1][3] + m[1][1], m[2][3] + m[2][1], m[3][3] + m[3][1]);
    planes[3] = vec4<f32>(m[0][3] - m[0][1], m[1][3] - m[1][1], m[2][3] - m[2][1], m[3][3] - m[3][1]);
    planes[4] = vec4<f32>(m[0][3] + m[0][2], m[1][3] + m[1][2], m[2][3] + m[2][2], m[3][3] + m[3][2]);
    planes[5] = vec4<f32>(m[0][3] - m[0][2], m[1][3] - m[1][2], m[2][3] - m[2][2], m[3][3] - m[3][2]);

    for (var i = 0; i < 6; i = i + 1) {
        planes[i] = planes[i] / length(planes[i].xyz);
    }

    if (intersects_frustum(world_center, radius, planes)) {
        let out_idx = atomicAdd(&indirect_commands[mesh_idx].instance_count, 1u);
        let global_out_idx = m_meta.base_instance + out_idx;
        visible_instances[global_out_idx] = instance;
    }
}
