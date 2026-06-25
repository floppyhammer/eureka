#import eureka::camera::Camera

@group(0) @binding(0)
var<uniform> camera: Camera;

struct Material {
    base_color: vec4<f32>,
    emissive: vec3<f32>,
    emissive_strength: f32,
    metallic: f32,
    roughness: f32,
    alpha_cutoff: f32,
    color_texture_idx: i32,
    normal_texture_idx: i32,
    metallic_roughness_texture_idx: i32,
    occlusion_texture_idx: i32,
    emissive_texture_idx: i32,
    alpha_mode: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
}

@group(1) @binding(0)
var<storage, read> materials: array<Material>;
@group(1) @binding(1)
var t_textures: binding_array<texture_2d<f32>>;
@group(1) @binding(2)
var s_sampler: sampler;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) normal: vec3<f32>,
    @location(3) tangent: vec3<f32>,
    @location(4) bitangent: vec3<f32>,
}

struct InstanceInput {
    @location(5) model_matrix_0: vec4<f32>,
    @location(6) model_matrix_1: vec4<f32>,
    @location(7) model_matrix_2: vec4<f32>,
    @location(8) model_matrix_3: vec4<f32>,
    @location(9) normal_matrix_0: vec4<f32>,
    @location(10) normal_matrix_1: vec4<f32>,
    @location(11) normal_matrix_2: vec4<f32>,
    @location(12) material_idx: u32,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_normal: vec3<f32>,
    @location(1) world_tangent: vec3<f32>,
    @location(2) world_bitangent: vec3<f32>,
    @location(3) tex_coords: vec2<f32>,
    @location(4) @interpolate(flat) material_idx: u32,
    @location(5) cur_clip_pos: vec4<f32>,
    @location(6) prev_clip_pos: vec4<f32>,
}

@vertex
fn vs_main(vertex: VertexInput, instance: InstanceInput) -> VertexOutput {
    let model_matrix = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3);

    let normal_matrix = mat3x3<f32>(
        instance.normal_matrix_0.xyz,
        instance.normal_matrix_1.xyz,
        instance.normal_matrix_2.xyz);

    let world_pos = model_matrix * vec4<f32>(vertex.position, 1.0);
    let world_normal = normalize(normal_matrix * vertex.normal);
    let world_tangent = normalize(normal_matrix * vertex.tangent);
    let world_bitangent = normalize(normal_matrix * vertex.bitangent);

    var out: VertexOutput;
    // 使用抖动的矩阵用于渲染深度，确保与后续 Pass 对齐
    out.clip_position = camera.view_proj * world_pos;
    out.world_normal = world_normal;
    out.world_tangent = world_tangent;
    out.world_bitangent = world_bitangent;
    out.tex_coords = vertex.tex_coords;
    out.material_idx = instance.material_idx;

    // Velocity 计算：使用不带抖动的矩阵计算 NDC 差异
    out.cur_clip_pos = camera.unjittered_view_proj * world_pos;
    out.prev_clip_pos = camera.prev_view_proj * world_pos;

    return out;
}

struct FragmentOutput {
    @location(0) normal: vec4<f32>,
    @location(1) velocity: vec2<f32>,
}

@fragment
fn fs_main(in: VertexOutput) -> FragmentOutput {
    let material = materials[in.material_idx];

    // Alpha Clipping (支持半透遮罩)
    if (material.alpha_mode == 1u) {
        if (material.color_texture_idx >= 0) {
            let sampled_alpha = textureSample(t_textures[u32(material.color_texture_idx)], s_sampler, in.tex_coords).a;
            if (sampled_alpha * material.base_color.a < material.alpha_cutoff) {
                discard;
            }
        }
    }

    // Normal mapping
    let tbn = mat3x3<f32>(normalize(in.world_tangent), normalize(in.world_bitangent), normalize(in.world_normal));
    var world_normal = tbn[2];
    if (material.normal_texture_idx >= 0) {
        let normal_map = textureSample(t_textures[u32(material.normal_texture_idx)], s_sampler, in.tex_coords).xyz * 2.0 - 1.0;
        world_normal = normalize(tbn * normal_map);
    }
    let view_normal = normalize((camera.view * vec4<f32>(world_normal, 0.0)).xyz);

    // Roughness (SSAO/SSR 需要)
    var roughness: f32 = material.roughness;
    if (material.metallic_roughness_texture_idx >= 0) {
        let mr_sample = textureSample(t_textures[u32(material.metallic_roughness_texture_idx)], s_sampler, in.tex_coords);
        roughness = mr_sample.g;
    }

    // Velocity calculation: (Current NDC - Previous NDC)
    // 转换为 [0, 1] UV 空间的偏移
    let cur_pos = (in.cur_clip_pos.xy / in.cur_clip_pos.w) * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5, 0.5);
    let prev_pos = (in.prev_clip_pos.xy / in.prev_clip_pos.w) * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5, 0.5);
    let velocity = cur_pos - prev_pos;

    var out: FragmentOutput;
    out.normal = vec4<f32>(view_normal * 0.5 + 0.5, roughness);
    out.velocity = velocity;
    return out;
}
