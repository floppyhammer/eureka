#import eureka::camera::Camera

struct CullParams {
    total_instances: u32,
    screen_width: u32,
    screen_height: u32,
    max_mip_level: u32,
    depth_mode: u32,    // 0: Standard Depth (0=Near, 1=Far), 1: Reverse-Z
    use_hiz: u32,       // 0: Disable Hi-Z test, 1: Enable Hi-Z test
    _pad0: u32,
    _pad1: u32,
}

struct MeshMetadata {
    aabb_min: vec4<f32>,
    aabb_max: vec4<f32>,
    base_instance: u32,
    instance_count: u32,
    _pad0: u32,
    _pad1: u32,
    cone_axis_cutoff: vec4<f32>,
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
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
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
@group(0) @binding(5) var<uniform> params: CullParams;
@group(0) @binding(6) var hiz_texture: texture_2d<f32>;
@group(0) @binding(7) var hiz_sampler: sampler;

// 1. 视锥体剔除测试
fn passes_frustum_culling(pos: vec3<f32>, radius: f32, planes: array<vec4<f32>, 6>) -> bool {
    for (var i = 0; i < 6; i = i + 1) {
        if (dot(planes[i], vec4<f32>(pos, 1.0)) < -radius) {
            return false;
        }
    }
    return true;
}

// 2. 法线锥背面剔除测试 (预留给 Sub-mesh / Meshlet)
fn passes_normal_cone_culling(m_meta: MeshMetadata, instance: Instance, camera_pos: vec3<f32>, world_center: vec3<f32>) -> bool {
    if (m_meta.cone_axis_cutoff.w <= -1.0) {
        return true;
    }
    let view_dir = normalize(camera_pos - world_center);
    let cone_axis = m_meta.cone_axis_cutoff.xyz;
    let cone_cutoff = m_meta.cone_axis_cutoff.w;
    return dot(cone_axis, view_dir) >= cone_cutoff;
}

// 3. Hi-Z 遮挡剔除测试
fn passes_hiz_occlusion_culling(m_meta: MeshMetadata, model: mat4x4<f32>) -> bool {
    if (params.use_hiz == 0u) {
        return true;
    }

    let min_p = m_meta.aabb_min.xyz;
    let max_p = m_meta.aabb_max.xyz;

    var corners = array<vec3<f32>, 8>(
        vec3<f32>(min_p.x, min_p.y, min_p.z), vec3<f32>(max_p.x, min_p.y, min_p.z),
        vec3<f32>(min_p.x, max_p.y, min_p.z), vec3<f32>(max_p.x, max_p.y, min_p.z),
        vec3<f32>(min_p.x, min_p.y, max_p.z), vec3<f32>(max_p.x, min_p.y, max_p.z),
        vec3<f32>(min_p.x, max_p.y, max_p.z), vec3<f32>(max_p.x, max_p.y, max_p.z)
    );

    var uv_min = vec2<f32>(1.0, 1.0);
    var uv_max = vec2<f32>(0.0, 0.0);
    var nearest_z = 1.0;
    if (params.depth_mode == 1u) { nearest_z = 0.0; }

    var near_clipped_count = 0u;

    for (var i = 0u; i < 8u; i = i + 1u) {
        let world_pos = model * vec4<f32>(corners[i], 1.0);
        let clip_pos = camera.view_proj * world_pos;

        if (clip_pos.w <= 0.0) {
            near_clipped_count = near_clipped_count + 1u;
            continue;
        }

        let ndc = clip_pos.xyz / clip_pos.w;
        let uv = vec2<f32>(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5);

        uv_min = min(uv_min, uv);
        uv_max = max(uv_max, uv);

        if (params.depth_mode == 0u) {
            nearest_z = min(nearest_z, ndc.z);
        } else {
            nearest_z = max(nearest_z, ndc.z);
        }
    }

    if (near_clipped_count == 8u) {
        return false;
    }
    if (near_clipped_count > 0u) {
        return true;
    }

    uv_min = clamp(uv_min, vec2<f32>(0.0), vec2<f32>(1.0));
    uv_max = clamp(uv_max, vec2<f32>(0.0), vec2<f32>(1.0));

    let bbox_pixel_size = max(
        (uv_max.x - uv_min.x) * f32(params.screen_width),
        (uv_max.y - uv_min.y) * f32(params.screen_height)
    );

    let mip_level = clamp(
        u32(ceil(log2(max(bbox_pixel_size, 1.0)))),
        0u,
        params.max_mip_level
    );

    let mip_dims = max(
        vec2<i32>(i32(params.screen_width) >> mip_level, i32(params.screen_height) >> mip_level),
        vec2<i32>(1)
    );

    let texel_min = clamp(vec2<i32>(uv_min * vec2<f32>(mip_dims)), vec2<i32>(0), mip_dims - vec2<i32>(1));
    let texel_max = clamp(vec2<i32>(uv_max * vec2<f32>(mip_dims)), vec2<i32>(0), mip_dims - vec2<i32>(1));

    let d00 = textureLoad(hiz_texture, vec2<i32>(texel_min.x, texel_min.y), i32(mip_level)).r;
    let d10 = textureLoad(hiz_texture, vec2<i32>(texel_max.x, texel_min.y), i32(mip_level)).r;
    let d01 = textureLoad(hiz_texture, vec2<i32>(texel_min.x, texel_max.y), i32(mip_level)).r;
    let d11 = textureLoad(hiz_texture, vec2<i32>(texel_max.x, texel_max.y), i32(mip_level)).r;

    var scene_hiz_depth = 0.0;
    if (params.depth_mode == 0u) {
        scene_hiz_depth = max(max(d00, d10), max(d01, d11));
        return nearest_z <= scene_hiz_depth + 0.05;
    } else {
        scene_hiz_depth = min(min(d00, d10), min(d01, d11));
        return nearest_z >= scene_hiz_depth - 0.05;
    }
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let global_instance_idx = id.x;
    if (global_instance_idx >= params.total_instances) {
        return;
    }

    var mesh_idx = 0u;
    var found = false;
    let num_meshes = arrayLength(&mesh_metadatas);
    for (var i = 0u; i < num_meshes; i = i + 1u) {
        let m_meta_check = mesh_metadatas[i];
        if (global_instance_idx >= m_meta_check.base_instance && global_instance_idx < m_meta_check.base_instance + m_meta_check.instance_count) {
            mesh_idx = i;
            found = true;
            break;
        }
    }
    if (!found) { return; }

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

    // --- 统一短路剔除流水线 (Unified Short-Circuit Pipeline) ---
    // 1. 视锥体剔除 (Early Exit)
    if (!passes_frustum_culling(world_center, radius, planes)) {
        return;
    }
    // 2. 法线锥背面剔除 (Early Exit)
    if (!passes_normal_cone_culling(m_meta, instance, camera.view_pos.xyz, world_center)) {
        return;
    }
    // 3. Hi-Z 遮挡剔除 (Early Exit)
    if (!passes_hiz_occlusion_culling(m_meta, model)) {
        return;
    }

    let out_idx = atomicAdd(&indirect_commands[mesh_idx].instance_count, 1u);
    if (out_idx < m_meta.instance_count) {
        let global_out_idx = m_meta.base_instance + out_idx;
        visible_instances[global_out_idx] = instance;
    }
}
