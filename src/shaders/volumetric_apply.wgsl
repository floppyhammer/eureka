#import eureka::camera::Camera

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

fn interleaved_gradient_noise(uv: vec2<f32>, frame: u32) -> f32 {
    let magic = vec3<f32>(0.06711056, 0.00583715, 52.9829189);
    let frame_offset = fract(f32(frame % 16u) * 0.61803398875);
    return fract(magic.z * fract(dot(uv, magic.xy) + frame_offset));
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let scene_color = textureSample(t_main_color, s_linear, in.tex_coords);

    if (camera.volumetric_enabled == 0u) {
        return scene_color;
    }

    let depth = textureLoad(t_main_depth, vec2<i32>(in.clip_position.xy), 0);

    // 关键修正：深度缓冲是带抖动的，所以还原到 View 空间时要用带抖动的逆矩阵
    let clip_pos = vec4<f32>(in.tex_coords.x * 2.0 - 1.0, (1.0 - in.tex_coords.y) * 2.0 - 1.0, depth, 1.0);
    let view_pos = camera.inv_proj * clip_pos;
    let view_z = -view_pos.z / view_pos.w;

    // 计算采样 3D 纹理的 W 坐标
    let slice = log2(max(view_z, config.z_near) / config.z_near) / log2(config.z_far / config.z_near);

    // 时间性抖动上采样
    // 只有在 TAA 开启时才让抖动动起来，否则保持静止以防止闪烁
    var effective_frame = 0u;
    if (camera.taa_enabled == 1u) {
        effective_frame = camera.frame_count;
    }
    let noise = interleaved_gradient_noise(in.clip_position.xy, effective_frame);
    let dither_offset = (noise - 0.5) * (1.0 / vec2<f32>(240.0, 135.0));

    // 体积光本身是 Unjittered 的，所以采样坐标也要补偿
    let unjittered_uv = in.tex_coords - camera.jitter.xy * 0.5;
    let volumetric_uvw = vec3<f32>(unjittered_uv, saturate(slice));
    let fog = textureSampleLevel(t_volumetric, s_linear, volumetric_uvw + vec3<f32>(dither_offset, 0.0), 0.0);

    let final_color = scene_color.rgb * fog.a + fog.rgb;
    return vec4<f32>(final_color, scene_color.a);
}
