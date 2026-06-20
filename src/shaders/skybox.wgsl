#import eureka::camera::Camera

@group(0) @binding(0)
var<uniform> camera: Camera;

struct VertexInput {
    @location(0) position: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec3<f32>,
}

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    var modified_view = mat4x4<f32>(camera.view);
    modified_view[3][0] = 0.0;
    modified_view[3][1] = 0.0;
    modified_view[3][2] = 0.0;

    // 天空盒也应该跟随抖动，这样 TAA 才能对其进行去抖
    let pos = camera.proj * modified_view * vec4<f32>(model.position, 1.0);
    out.clip_position = pos.xyww;
    out.tex_coords = model.position;

    return out;
}

@group(1) @binding(0)
var t_cubemap: texture_cube<f32>;

@group(1) @binding(1)
var s_cubemap: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(t_cubemap, s_cubemap, in.tex_coords);
}
