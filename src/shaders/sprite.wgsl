#import eureka::camera::Camera

@group(0) @binding(0)
var<uniform> camera: Camera;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) texture_idx: u32,
    @location(4) mode: u32,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) @interpolate(flat) texture_idx: u32,
    @location(3) @interpolate(flat) mode: u32,
}

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = camera.view_proj * vec4<f32>(model.position, 1.0);
    out.tex_coords = model.tex_coords;
    out.color = model.color;
    out.texture_idx = model.texture_idx;
    out.mode = model.mode;
    return out;
}

// Bindless Textures
@group(1) @binding(1)
var t_textures: binding_array<texture_2d<f32>>;
@group(1) @binding(2)
var s_sampler: sampler;

fn median(r: f32, g: f32, b: f32) -> f32 {
    return max(min(r, g), min(max(r, g), b));
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let tex_color = textureSample(t_textures[in.texture_idx], s_sampler, in.tex_coords);

    // mode 0: Sprite (RGBA, 假设已经是预乘的或者全不透明)
    if (in.mode == 0u) {
        return in.color * tex_color;
    }

    // mode 1: Text (Alpha Mask)
    // 字体图集是 R8Unorm，mask 在 r 通道。
    // 由于渲染器使用的是 Premultiplied Alpha 混合：
    // RGB = VertexColor * Mask, Alpha = Mask
    if (in.mode == 1u) {
        let mask = tex_color.r;
        let final_alpha = in.color.a * mask;
        return vec4<f32>(in.color.rgb * final_alpha, final_alpha);
    }

    // mode 2: MSDF Text
    if (in.mode == 2u) {
        let msd = tex_color.rgb;
        let sd = median(msd.r, msd.g, msd.b);

        // 使用 fwidth 进行高质量抗锯齿
        let opacity = clamp((sd - 0.5) / max(fwidth(sd), 0.0001) + 0.5, 0.0, 1.0);

        let final_alpha = in.color.a * opacity;
        return vec4<f32>(in.color.rgb * final_alpha, final_alpha);
    }

    return vec4<f32>(0.0);
}
