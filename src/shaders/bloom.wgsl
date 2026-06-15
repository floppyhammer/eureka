// Bloom Shader (Downsample & Upsample)
// Based on Call of Duty: Next Generation Post-Processing

@group(0) @binding(0) var t_input: texture_2d<f32>;
@group(0) @binding(1) var s_input: sampler;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    let x = f32(i32(in_vertex_index) / 2) * 4.0 - 1.0;
    let y = f32(i32(in_vertex_index) % 2) * 4.0 - 1.0;
    out.uv = vec2<f32>((x + 1.0) * 0.5, (1.0 - y) * 0.5);
    out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
    return out;
}

fn luminance(v: vec3<f32>) -> f32 {
    return dot(v, vec3<f32>(0.2126, 0.7152, 0.0722));
}

@fragment
fn fs_downsample(in: VertexOutput) -> @location(0) vec4<f32> {
    let src_size = vec2<f32>(textureDimensions(t_input));
    let texel_size = 1.0 / src_size;
    let x = texel_size.x;
    let y = texel_size.y;

    // 13-tap filter
    let a = textureSample(t_input, s_input, in.uv + vec2<f32>(-2.0*x, 2.0*y)).rgb;
    let b = textureSample(t_input, s_input, in.uv + vec2<f32>(0.0, 2.0*y)).rgb;
    let c = textureSample(t_input, s_input, in.uv + vec2<f32>(2.0*x, 2.0*y)).rgb;

    let d = textureSample(t_input, s_input, in.uv + vec2<f32>(-x, y)).rgb;
    let e = textureSample(t_input, s_input, in.uv + vec2<f32>(x, y)).rgb;

    let f = textureSample(t_input, s_input, in.uv + vec2<f32>(-2.0*x, 0.0)).rgb;
    let g = textureSample(t_input, s_input, in.uv + vec2<f32>(0.0, 0.0)).rgb;
    let h = textureSample(t_input, s_input, in.uv + vec2<f32>(2.0*x, 0.0)).rgb;

    let i = textureSample(t_input, s_input, in.uv + vec2<f32>(-x, -y)).rgb;
    let j = textureSample(t_input, s_input, in.uv + vec2<f32>(x, -y)).rgb;

    let k = textureSample(t_input, s_input, in.uv + vec2<f32>(-2.0*x, -2.0*y)).rgb;
    let l = textureSample(t_input, s_input, in.uv + vec2<f32>(0.0, -2.0*y)).rgb;
    let m = textureSample(t_input, s_input, in.uv + vec2<f32>(2.0*x, -2.0*y)).rgb;

    let color = (a+c+k+m) * 0.03125 + (b+f+h+l) * 0.0625 + (d+e+i+j) * 0.125 + g * 0.125;

    return vec4<f32>(color, 1.0);
}

@group(0) @binding(2) var t_upsample: texture_2d<f32>;

@fragment
fn fs_upsample(in: VertexOutput) -> @location(0) vec4<f32> {
    let x = 0.005;
    let y = 0.005;

    let a = textureSample(t_input, s_input, in.uv + vec2<f32>(-x, y)).rgb;
    let b = textureSample(t_input, s_input, in.uv + vec2<f32>(0.0, y)).rgb;
    let c = textureSample(t_input, s_input, in.uv + vec2<f32>(x, y)).rgb;

    let d = textureSample(t_input, s_input, in.uv + vec2<f32>(-x, 0.0)).rgb;
    let e = textureSample(t_input, s_input, in.uv + vec2<f32>(0.0, 0.0)).rgb;
    let f = textureSample(t_input, s_input, in.uv + vec2<f32>(x, 0.0)).rgb;

    let g = textureSample(t_input, s_input, in.uv + vec2<f32>(-x, -y)).rgb;
    let h = textureSample(t_input, s_input, in.uv + vec2<f32>(0.0, -y)).rgb;
    let i = textureSample(t_input, s_input, in.uv + vec2<f32>(x, -y)).rgb;

    let upsampled = (a + c + g + i) * 1.0 / 16.0 + (b + d + f + h) * 2.0 / 16.0 + e * 4.0 / 16.0;

    let base = textureSample(t_upsample, s_input, in.uv).rgb;
    return vec4<f32>(base + upsampled, 1.0);
}
