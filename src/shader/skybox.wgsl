//////////////////////////////// Vertex shader ////////////////////////////////

struct Camera {
    view_pos: vec4<f32>,
    view_proj: mat4x4<f32>,
    proj: mat4x4<f32>,
}

// Bind group 1.
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

    let pos = camera.view_proj * vec4<f32>(model.position, 1.0);
    out.clip_position = pos.xyww;

    out.tex_coords = model.position;

    return out;
}

//////////////////////////////// Fragment shader ////////////////////////////////

@group(1) @binding(0)
var t_cubemap: texture_cube<f32>;

@group(1) @binding(1)
var s_cubemap: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(t_cubemap, s_cubemap, in.tex_coords);
}
