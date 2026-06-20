@group(0) @binding(0) var t_color: texture_2d<f32>;
@group(0) @binding(1) var t_ssr: texture_2d<f32>;
@group(0) @binding(2) var s_linear: sampler;

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

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(t_color, s_linear, in.uv);
    let ssr = textureSample(t_ssr, s_linear, in.uv);
    
    // --- DEBUG: 显示SSR强度 ---
    // 取消下面的注释可以看到SSR在哪些像素上产生了反射
    //return vec4<f32>(vec3<f32>(ssr.a), 1.0);
    
    // SSR 的 alpha 存储了反射强度
    // 使用 additive blending 将反射叠加到颜色上
    let result = color.rgb + ssr.rgb * ssr.a;
    
    return vec4<f32>(result, color.a);
}