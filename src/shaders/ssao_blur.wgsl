@group(0) @binding(0)
var t_ssao: texture_2d<f32>;
@group(0) @binding(1)
var s_ssao: sampler;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    let x = f32(i32(in_vertex_index) & 1);
    let y = f32(i32(in_vertex_index) >> 1);
    out.uv = vec2<f32>(x * 2.0, y * 2.0);
    out.clip_position = vec4<f32>(out.uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0), 0.0, 1.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) f32 {
    let texel_size = 1.0 / vec2<f32>(textureDimensions(t_ssao));
    var result = 0.0;
    for (var x = -2; x < 2; x = x + 1) {
        for (var y = -2; y < 2; y = y + 1) {
            let offset = vec2<f32>(f32(x), f32(y)) * texel_size;
            result = result + textureSample(t_ssao, s_ssao, in.uv + offset).r;
        }
    }
    return result / 16.0;
}
