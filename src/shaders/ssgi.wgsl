#import eureka::camera::Camera

@group(0) @binding(0) var<uniform> camera: Camera;

@group(0) @binding(1) var t_color: texture_2d<f32>;
@group(0) @binding(2) var t_normal_roughness: texture_2d<f32>;
@group(0) @binding(3) var t_depth: texture_depth_2d;
@group(0) @binding(4) var s_nearest: sampler;
@group(0) @binding(5) var s_linear: sampler;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    let x = f32(i32(in_vertex_index & 1u) * 4 - 1);
    let y = f32(i32(in_vertex_index & 2u) * 2 - 1);
    out.position = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = vec2<f32>((x + 1.0) * 0.5, (1.0 - y) * 0.5);
    return out;
}

fn ndc_depth_to_view_z(depth: f32) -> f32 {
    let view_pos = camera.inv_proj * vec4<f32>(0.0, 0.0, depth, 1.0);
    return view_pos.z / view_pos.w;
}

fn reconstruct_view_pos(uv: vec2<f32>, depth: f32) -> vec3<f32> {
    let ndc = vec3<f32>(uv.x * 2.0 - 1.0, (1.0 - uv.y) * 2.0 - 1.0, depth);
    let view_pos_h = camera.inv_proj * vec4<f32>(ndc, 1.0);
    return view_pos_h.xyz / view_pos_h.w;
}

// 产生一个基于法线方向的伪随机采样方向 (余弦加权)
fn get_cos_sample_dir(n: vec3<f32>, uv: vec2<f32>, frame: u32) -> vec3<f32> {
    let noise = vec2<f32>(
        fract(sin(dot(uv + f32(frame) * 0.1, vec2<f32>(12.9898, 78.233))) * 43758.5453),
        fract(sin(dot(uv + f32(frame) * 0.1, vec2<f32>(26.6514, 41.4132))) * 43758.5453)
    );

    let phi = 2.0 * 3.14159265 * noise.x;
    let cos_theta = sqrt(noise.y);
    let sin_theta = sqrt(1.0 - noise.y);

    let local_dir = vec3<f32>(sin_theta * cos(phi), sin_theta * sin(phi), cos_theta);

    // 建立正交基
    var up = vec3<f32>(0.0, 1.0, 0.0);
    if (abs(n.y) > 0.99) { up = vec3<f32>(1.0, 0.0, 0.0); }
    let tangent = normalize(cross(up, n));
    let bitangent = cross(n, tangent);

    return tangent * local_dir.x + bitangent * local_dir.y + n * local_dir.z;
}

fn ray_march(origin: vec3<f32>, direction: vec3<f32>, noise: f32) -> vec2<f32> {
    let max_steps = 12u;
    let step_size = 0.2; // 漫反射需要更大的步长来探测远处的颜色
    let thickness = 0.5;

    var current_pos = origin + direction * step_size * (noise + 0.1);

    for (var i = 0u; i < max_steps; i++) {
        current_pos += direction * step_size;

        let clip_pos = camera.proj * vec4<f32>(current_pos, 1.0);
        let screen_pos = clip_pos.xyz / clip_pos.w;
        let uv = vec2<f32>(screen_pos.x * 0.5 + 0.5, 0.5 - screen_pos.y * 0.5);

        if (uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0) { return vec2<f32>(-1.0); }

        let scene_depth = textureSampleLevel(t_depth, s_nearest, uv, 0);
        let scene_view_z = ndc_depth_to_view_z(scene_depth);
        let delta = scene_view_z - current_pos.z;

        if (delta > 0.0 && delta < thickness) {
            return uv;
        }
    }
    return vec2<f32>(-1.0);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // 只有 SSR 开启时才运行 SSGI（共用开关）
    if (camera.ssr_enabled == 0u) { return vec4<f32>(0.0); }

    let depth = textureSampleLevel(t_depth, s_nearest, in.uv, 0);
    if (depth >= 1.0) { return vec4<f32>(0.0); }

    let nr_sample = textureSampleLevel(t_normal_roughness, s_nearest, in.uv, 0);
    let view_normal = normalize(nr_sample.xyz * 2.0 - 1.0);
    let view_pos = reconstruct_view_pos(in.uv, depth);

    // 随机噪声
    let noise = fract(sin(dot(in.uv, vec2<f32>(12.9898, 78.233))) * 43758.5453);

    // 为了性能，每像素只投射 2 条随机光线，依靠时域累积
    var gi_accum = vec3<f32>(0.0);
    let num_samples = 2u;

    for (var i = 0u; i < num_samples; i++) {
        let sample_dir = get_cos_sample_dir(view_normal, in.uv + f32(i), camera.frame_count);
        let hit_uv = ray_march(view_pos, sample_dir, noise);

        if (hit_uv.x >= 0.0) {
            let hit_color = textureSampleLevel(t_color, s_linear, hit_uv, 0.0).rgb;
            // 简单的距离衰减
            let hit_pos = reconstruct_view_pos(hit_uv, textureSampleLevel(t_depth, s_nearest, hit_uv, 0));
            let dist = length(hit_pos - view_pos);
            let falloff = 1.0 / (1.0 + dist * dist);
            gi_accum += hit_color * falloff;
        }
    }

    return vec4<f32>(gi_accum / f32(num_samples), 1.0);
}
