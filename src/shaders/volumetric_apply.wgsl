struct Camera {
    view_pos: vec4<f32>,
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    view_proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    inv_view: mat4x4<f32>,
    ssao_enabled: u32,
    volumetric_enabled: u32,
}

struct ClusterConfig {
    screen_size: vec2<f32>,
    _pad0: vec2<f32>,
    grid_size: vec3<u32>,
    num_lights: u32,
    z_near: f32,
    z_far: f32,
    _pad1: vec2<f32>,
}

@group(0) @binding(0) var<uniform> camera: Camera;
@group(0) @binding(1) var<uniform> config: ClusterConfig;
@group(0) @binding(2) var t_main_color: texture_2d<f32>;
@group(0) @binding(3) var t_main_depth: texture_depth_2d;
@group(0) @binding(4) var t_volumetric: texture_3d<f32>;
@group(0) @binding(5) var s_linear: sampler;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    let x = f32(i32(in_vertex_index & 1u) * 4 - 1);
    let y = f32(i32(in_vertex_index & 2u) * 2 - 1);
    out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
    out.tex_coords = vec2<f32>((x + 1.0) * 0.5, 1.0 - (y + 1.0) * 0.5);
    return out;
}

fn hash_2d(p: vec2<f32>) -> f32 {
    let p3 = fract(vec3<f32>(p.xyx) * 0.1031);
    let p3_2 = p3 + dot(p3, p3.yzx + 33.33);
    return fract((p3_2.x + p3_2.y) * p3_2.z);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let scene_color = textureSample(t_main_color, s_linear, in.tex_coords);

    if (camera.volumetric_enabled == 0u) {
        return scene_color;
    }

    let depth = textureLoad(t_main_depth, vec2<i32>(in.clip_position.xy), 0);

    // 将深度转换回 View 空间深度
    // 如果是天空盒，depth 应该为 0.0 (Reverse-Z) 或 1.0
    // 我们需要将其处理为 view-space z
    let clip_pos = vec4<f32>(in.tex_coords.x * 2.0 - 1.0, (1.0 - in.tex_coords.y) * 2.0 - 1.0, depth, 1.0);
    let view_pos = camera.inv_proj * clip_pos;
    let view_z = -view_pos.z / view_pos.w;

    // 计算采样 3D 纹理的 W 坐标
    // 对应 volumetric.wgsl 中的对数分布逻辑
    let slice = log2(max(view_z, config.z_near) / config.z_near) / log2(config.z_far / config.z_near);

    // 抖动上采样以消除马赛克
    let noise = hash_2d(in.clip_position.xy);
    let dither_offset = (noise - 0.5) * (1.0 / vec2<f32>(240.0, 135.0));

    let volumetric_uvw = vec3<f32>(in.tex_coords, saturate(slice));
    let fog = textureSampleLevel(t_volumetric, s_linear, volumetric_uvw + vec3<f32>(dither_offset, 0.0), 0.0);

    let final_color = scene_color.rgb * fog.a + fog.rgb;
    return vec4<f32>(final_color, scene_color.a);
}
