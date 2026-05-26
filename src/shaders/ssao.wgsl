struct Camera {
    view_pos: vec4<f32>,
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    view_proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> camera: Camera;

struct SSAOUniform {
    samples: array<vec4<f32>, 64>,
}

@group(1) @binding(0)
var<uniform> ssao_uniform: SSAOUniform;
@group(1) @binding(1)
var t_normal: texture_2d<f32>;
@group(1) @binding(2)
var s_normal: sampler;
@group(1) @binding(3)
var t_depth: texture_depth_2d;
@group(1) @binding(4)
var t_noise: texture_2d<f32>;
@group(1) @binding(5)
var s_noise: sampler;

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
    out.uv = out.uv; // Full screen quad
    return out;
}

fn get_view_pos(uv: vec2<f32>) -> vec3<f32> {
    // Correctly map UV to NDC space: Y must be flipped (UV 0 is top, NDC 1 is top)
    let clip_pos = vec4<f32>(
        uv.x * 2.0 - 1.0,
        (1.0 - uv.y) * 2.0 - 1.0,
        textureSampleLevel(t_depth, s_normal, uv, 0),
        1.0
    );
    let view_pos_h = camera.inv_proj * clip_pos;
    return view_pos_h.xyz / view_pos_h.w;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) f32 {
    let screen_size = vec2<f32>(textureDimensions(t_normal));
    let noise_scale = screen_size / 4.0; // Noise texture is 4x4

    let frag_pos = get_view_pos(in.uv);
    // Reconstruct view-space normal from the G-Buffer
    let normal = normalize(textureSample(t_normal, s_normal, in.uv).xyz * 2.0 - 1.0);
    let random_vec = normalize(textureSample(t_noise, s_noise, in.uv * noise_scale).xyz * 2.0 - 1.0);

    // Create TBN matrix from view-space normal and random vector
    let tangent = normalize(random_vec - normal * dot(random_vec, normal));
    let bitangent = cross(normal, tangent);
    let tbn = mat3x3<f32>(tangent, bitangent, normal);

    var occlusion = 0.0;
    let radius = 0.5;
    let bias = 0.05; // Increased bias to prevent moving stripes on flat surfaces

    for (var i = 0; i < 64; i = i + 1) {
        // From tangent to view-space
        let sample_pos = tbn * ssao_uniform.samples[i].xyz;
        let sample_pos_view = frag_pos + sample_pos * radius;

        // Project sample position to find sample UV
        var offset = vec4<f32>(sample_pos_view, 1.0);
        offset = camera.proj * offset;
        offset.x = offset.x / offset.w;
        offset.y = offset.y / offset.w;
        // Project to UV: NDC (-1, 1) -> UV (0, 1)
        let sample_uv = vec2<f32>(offset.x * 0.5 + 0.5, 0.5 - offset.y * 0.5);

        // Get actual depth at the sample's UV location
        let actual_pos_view = get_view_pos(sample_uv);
        let sample_depth_view = actual_pos_view.z;

        // Range check to avoid "dark halos" behind objects far away
        let range_check = smoothstep(0.0, 1.0, radius / abs(frag_pos.z - sample_depth_view));

        // In View Space (Right Handed), Camera looks at -Z.
        // Higher Z value means closer to camera.
        // We add occlusion if the actual geometry is closer to camera than our sample point.
        if (sample_depth_view >= sample_pos_view.z + bias) {
            occlusion = occlusion + range_check;
        }
    }

    occlusion = 1.0 - (occlusion / 64.0);
    return pow(occlusion, 2.0); // Increase contrast
}
