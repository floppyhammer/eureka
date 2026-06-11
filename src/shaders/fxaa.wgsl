struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
};

@vertex
fn vs_main(
    @builtin(vertex_index) in_vertex_index: u32,
) -> VertexOutput {
    var out: VertexOutput;
    // 使用 "大三角形" 技巧，3个顶点分别为 (-1, -1), (3, -1), (-1, 3)
    // 这样可以完全覆盖 (-1, -1) 到 (1, 1) 的全屏范围
    let x = f32(i32(in_vertex_index & 1u) * 4 - 1);
    let y = f32(i32(in_vertex_index & 2u) * 2 - 1);
    out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
    out.tex_coords = vec2<f32>((x + 1.0) * 0.5, 1.0 - (y + 1.0) * 0.5);
    return out;
}

@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(1)
var s_diffuse: sampler;
@group(0) @binding(2)
var<uniform> fxaa_enabled: u32;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    if (fxaa_enabled == 0u) {
        return textureSample(t_diffuse, s_diffuse, in.tex_coords);
    }

    let screen_size = vec2<f32>(textureDimensions(t_diffuse));
    let inv_screen_size = 1.0 / screen_size;

    let fxaa_span_max = 8.0;
    let fxaa_reduce_mul = 1.0 / 8.0;
    let fxaa_reduce_min = 1.0 / 128.0;

    let rgbNW = textureSample(t_diffuse, s_diffuse, in.tex_coords + vec2<f32>(-1.0, -1.0) * inv_screen_size).rgb;
    let rgbNE = textureSample(t_diffuse, s_diffuse, in.tex_coords + vec2<f32>(1.0, -1.0) * inv_screen_size).rgb;
    let rgbSW = textureSample(t_diffuse, s_diffuse, in.tex_coords + vec2<f32>(-1.0, 1.0) * inv_screen_size).rgb;
    let rgbSE = textureSample(t_diffuse, s_diffuse, in.tex_coords + vec2<f32>(1.0, 1.0) * inv_screen_size).rgb;
    let rgbM  = textureSample(t_diffuse, s_diffuse, in.tex_coords).rgb;

    let luma = vec3<f32>(0.299, 0.587, 0.114);
    let lumaNW = dot(rgbNW, luma);
    let lumaNE = dot(rgbNE, luma);
    let lumaSW = dot(rgbSW, luma);
    let lumaSE = dot(rgbSE, luma);
    let lumaM  = dot(rgbM,  luma);

    let lumaMin = min(lumaM, min(min(lumaNW, lumaNE), min(lumaSW, lumaSE)));
    let lumaMax = max(lumaM, max(max(lumaNW, lumaNE), max(lumaSW, lumaSE)));

    var dir: vec2<f32>;
    dir.x = -((lumaNW + lumaNE) - (lumaSW + lumaSE));
    dir.y =  ((lumaNW + lumaSW) - (lumaNE + lumaSE));

    let dirReduce = max(
        (lumaNW + lumaNE + lumaSW + lumaSE) * (0.25 * fxaa_reduce_mul),
        fxaa_reduce_min
    );

    let rcpDirMin = 1.0 / (min(abs(dir.x), abs(dir.y)) + dirReduce);

    dir = min(vec2<f32>(fxaa_span_max, fxaa_span_max),
          max(vec2<f32>(-fxaa_span_max, -fxaa_span_max),
          dir * rcpDirMin)) * inv_screen_size;

    let rgbA = 0.5 * (
        textureSample(t_diffuse, s_diffuse, in.tex_coords + dir * (1.0 / 3.0 - 0.5)).rgb +
        textureSample(t_diffuse, s_diffuse, in.tex_coords + dir * (2.0 / 3.0 - 0.5)).rgb);
    let rgbB = rgbA * 0.5 + 0.25 * (
        textureSample(t_diffuse, s_diffuse, in.tex_coords + dir * (0.0 / 3.0 - 0.5)).rgb +
        textureSample(t_diffuse, s_diffuse, in.tex_coords + dir * (3.0 / 3.0 - 0.5)).rgb);

    let lumaB = dot(rgbB, luma);
    if ((lumaB < lumaMin) || (lumaB > lumaMax)) {
        return vec4<f32>(rgbA, 1.0);
    } else {
        return vec4<f32>(rgbB, 1.0);
    }
}
