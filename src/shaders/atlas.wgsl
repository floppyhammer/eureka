// For text and 2D particles.

// Vertex shader //

struct AtlasParams {
    camera_view_size: vec2<f32>,
    atlas_size: vec2<f32>,
}

@group(0) @binding(0)
var<uniform> params: AtlasParams;

struct InstanceInput {
    @location(0) position: vec2<f32>,
    @location(1) size: vec2<f32>,
    @location(2) region: vec4<f32>,
    @location(3) color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) color: vec4<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32, instance: InstanceInput) -> VertexOutput {
    var out: VertexOutput;

    let u0 = ((in_vertex_index << 1u) & 2u) >> 1u; // [0, 1]
    let v0 = ((in_vertex_index & 2u)) >> 1u; // [0, 1]

    let position = vec4<f32>(f32(u0), 1.0 - f32(v0), 0.0, 1.0);

    let u = instance.region[u0 * 2u];
    let v = instance.region[v0 * 2u + 1u];

    var translation = mat4x4<f32>(
        1.0, 0.0, 0.0, 0.0,
        0.0, 1.0, 0.0, 0.0,
        0.0, 0.0, 1.0, 0.0,
        0.0, 0.0, 0.0, 1.0,
    );
    translation[3][0] = (instance.position.x / params.camera_view_size.x) * 2.0 - 1.0;
    translation[3][1] = 1.0 - (instance.position.y / params.camera_view_size.y) * 2.0;

    var scale = mat4x4<f32>(
        1.0, 0.0, 0.0, 0.0,
        0.0, 1.0, 0.0, 0.0,
        0.0, 0.0, 1.0, 0.0,
        0.0, 0.0, 0.0, 1.0,
    );
    scale[0][0] = instance.size.x * 2.0 / params.camera_view_size.x;
    scale[1][1] = instance.size.y * 2.0 / params.camera_view_size.y;

    out.clip_position = translation * scale * position;
    out.tex_coords = vec2<f32>(u, v);
    out.color = instance.color;

    return out;
}

// Fragment shader //

@group(1) @binding(0)
var t_diffuse: texture_2d<f32>;

@group(1) @binding(1)
var s_diffuse: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
#ifdef TEXT
        return in.color * textureSample(t_diffuse, s_diffuse, in.tex_coords).r;
#else
        return in.color * textureSample(t_diffuse, s_diffuse, in.tex_coords);
#endif
}
