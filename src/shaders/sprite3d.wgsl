//////////////////////////////// Vertex shader ////////////////////////////////

struct Camera {
    view_pos: vec4<f32>,
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> camera: Camera;

struct Params {
    model_matrix: mat4x4<f32>,
    billboard_mode: f32,
    pad0: f32,
    pad1: f32,
    pad2: f32,
}

@group(2) @binding(0)
var<uniform> params: Params;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) normal: vec3<f32>,
    @location(3) tangent: vec3<f32>,
    @location(4) bitangent: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
}

@vertex
fn vs_main(
    model: VertexInput
) -> VertexOutput {
    var out: VertexOutput;

    var model_view = camera.view * params.model_matrix;

    let billboard_mode = u32(params.billboard_mode);

    if (billboard_mode == 1u) {
        // Spherical billboarding.
        model_view[0][0] = 1.0;
        model_view[0][1] = 0.0;
        model_view[0][2] = 0.0;

        model_view[1][0] = 0.0;
        model_view[1][1] = 1.0;
        model_view[1][2] = 0.0;

        model_view[2][0] = 0.0;
        model_view[2][1] = 0.0;
        model_view[2][2] = 1.0;
    }

    out.clip_position = camera.proj * model_view * vec4<f32>(model.position, 1.0);
    out.tex_coords = model.tex_coords;

    return out;
}

//////////////////////////////// Fragment shader ////////////////////////////////

@group(1) @binding(0)
var t_diffuse: texture_2d<f32>;

@group(1) @binding(1)
var s_diffuse: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(t_diffuse, s_diffuse, in.tex_coords);
}
