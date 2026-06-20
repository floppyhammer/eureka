#import eureka::camera::Camera

@group(0) @binding(0) var<uniform> camera: Camera;
@group(0) @binding(1) var current_color: texture_2d<f32>;
@group(0) @binding(2) var history_color: texture_2d<f32>;
@group(0) @binding(3) var depth_tex: texture_depth_2d;
@group(0) @binding(4) var linear_sampler: sampler;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    let x = f32(i32(in_vertex_index & 1u) * 4 - 1);
    let y = f32(i32(in_vertex_index & 2u) * 2 - 1);
    out.position = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = vec2<f32>((x + 1.0) * 0.5, 1.0 - (y + 1.0) * 0.5);
    return out;
}

fn RGBToYCbCr(rgb: vec3<f32>) -> vec3<f32> {
    let y = 0.299 * rgb.r + 0.587 * rgb.g + 0.114 * rgb.b;
    let cb = (rgb.b - y) * 0.565;
    let cr = (rgb.r - y) * 0.713;
    return vec3<f32>(y, cb, cr);
}

fn YCbCrToRGB(ycbcr: vec3<f32>) -> vec3<f32> {
    let r = ycbcr.x + 1.403 * ycbcr.z;
    let g = ycbcr.x - 0.344 * ycbcr.y - 0.714 * ycbcr.z;
    let b = ycbcr.x + 1.770 * ycbcr.y;
    return vec3<f32>(r, g, b);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let tex_size = vec2<f32>(textureDimensions(current_color));
    let inv_tex_size = 1.0 / tex_size;

    // 核心修正：补偿当前帧的亚像素抖动
    let unjittered_uv = in.uv - camera.jitter.xy * 0.5;
    let color_rgb = textureSample(current_color, linear_sampler, unjittered_uv).rgb;

    if (camera.taa_enabled == 0u) {
        return vec4<f32>(color_rgb, 1.0);
    }

    // --- Reprojection ---
    // 采样抖动后的深度
    let depth = textureSample(depth_tex, linear_sampler, in.uv);

    // 利用抖动的 inv_view_proj 将抖动的像素还原到世界空间 (这是稳定的)
    let ndc = vec3<f32>(in.uv.x * 2.0 - 1.0, (1.0 - in.uv.y) * 2.0 - 1.0, depth);
    let world_pos_h = camera.inv_view_proj * vec4<f32>(ndc, 1.0);
    let world_pos = world_pos_h.xyz / world_pos_h.w;

    // 重投影到上一帧的 unjittered UV
    let prev_ndc_h = camera.prev_view_proj * vec4<f32>(world_pos, 1.0);
    let prev_ndc = prev_ndc_h.xyz / prev_ndc_h.w;
    let prev_uv = vec2<f32>(prev_ndc.x * 0.5 + 0.5, 0.5 - prev_ndc.y * 0.5);

    // --- Neighborhood Clamping in YCbCr Space ---
    var m1 = vec3<f32>(0.0);
    var m2 = vec3<f32>(0.0);

    for (var x = -1; x <= 1; x++) {
        for (var y = -1; y <= 1; y++) {
            let neighbor_rgb = textureSample(current_color, linear_sampler, unjittered_uv + vec2<f32>(f32(x), f32(y)) * inv_tex_size).rgb;
            let neighbor_ycbcr = RGBToYCbCr(neighbor_rgb);
            m1 += neighbor_ycbcr;
            m2 += neighbor_ycbcr * neighbor_ycbcr;
        }
    }

    let mean = m1 / 9.0;
    let std_dev = sqrt(max(vec3<f32>(0.0), (m2 / 9.0) - (mean * mean)));

    let gamma = 2.0;
    let min_ycbcr = mean - gamma * std_dev;
    let max_ycbcr = mean + gamma * std_dev;

    // Sample history and convert to YCbCr
    let history_rgb = textureSample(history_color, linear_sampler, prev_uv).rgb;
    var history_ycbcr = RGBToYCbCr(history_rgb);

    // Clamp history
    history_ycbcr = clamp(history_ycbcr, min_ycbcr, max_ycbcr);

    // --- Final Blend ---
    var alpha = 0.1;

    // Discard history if out of bounds
    if (prev_uv.x < 0.0 || prev_uv.x > 1.0 || prev_uv.y < 0.0 || prev_uv.y > 1.0) {
        alpha = 1.0;
    }

    let current_ycbcr = RGBToYCbCr(color_rgb);
    let final_ycbcr = mix(history_ycbcr, current_ycbcr, alpha);
    let final_rgb = YCbCrToRGB(final_ycbcr);

    return vec4<f32>(final_rgb, 1.0);
}
