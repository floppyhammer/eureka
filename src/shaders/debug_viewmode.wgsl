// Debug Viewmode shader: visualizes intermediate render buffers
// Viewmode enum: 0=Default(pass-through), 1=Normals, 2=Depth, 3=Velocity, 4=Prepass

#import eureka::camera::Camera

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    let x = f32(i32(in_vertex_index & 1u) * 4 - 1);
    let y = f32(i32(in_vertex_index & 2u) * 2 - 1);
    out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
    out.tex_coords = vec2<f32>((x + 1.0) * 0.5, 1.0 - (y + 1.0) * 0.5);
    return out;
}

// Uniform buffer: viewmode type (u32) + padding to 16 bytes
struct DebugUniforms {
    viewmode: u32,
    _padding: vec3<u32>,
};

@group(0) @binding(0)
var<uniform> debug_uniforms: DebugUniforms;
@group(0) @binding(1)
var<uniform> camera: Camera;

// Input textures
@group(0) @binding(2)
var t_main_color: texture_2d<f32>;
@group(0) @binding(3)
var t_main_depth: texture_depth_2d;
@group(0) @binding(4)
var t_prepass_normal: texture_2d<f32>;
@group(0) @binding(5)
var t_prepass_velocity: texture_2d<f32>;

@group(0) @binding(6)
var s_linear: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let mode = debug_uniforms.viewmode;
    let uv = in.tex_coords;

    // Mode 0: Default - pass through main_color
    if (mode == 0u) {
        return textureSample(t_main_color, s_linear, uv);
    }

    // Mode 1: Normals - visualize view-space normals as RGB
    if (mode == 1u) {
        let normal_raw = textureSample(t_prepass_normal, s_linear, uv);
        // Normal is stored in RGB channels (view-space normal)
        let normal = normalize(normal_raw.rgb);
        // Map from [-1,1] to [0,1] for visualization
        let color = normal * 0.5 + 0.5;
        return vec4<f32>(color.x, color.y, color.z, 1.0);
    }

    // Mode 2: Depth - visualize depth buffer
    if (mode == 2u) {
        let dims = textureDimensions(t_main_depth);
        // Clamp texture coordinates to valid range for textureLoad
        let coord = vec2<i32>(i32(uv.x * f32(dims.x)), i32(uv.y * f32(dims.y)));
        let clampedCoord = vec2<i32>(min(coord.x, i32(dims.x) - 1), min(coord.y, i32(dims.y) - 1));
        let depth = textureLoad(t_main_depth, clampedCoord, 0);
        // Standard depth: already in [0, 1] range (0=near, 1=far)
        // Invert for visualization: near=white, far=black (more intuitive)
        let depthColor = 1.0 - depth;
        return vec4<f32>(depthColor, depthColor, depthColor, 1.0);
    }

    // Mode 3: Velocity - visualize motion vectors
    if (mode == 3u) {
        let velocity = textureSample(t_prepass_velocity, s_linear, uv).rg;
        // Visualize velocity as color
        let velLength = length(velocity);
        // Use a large multiplier to make small motion vectors visible
        // Motion vectors are typically in the range of 0.0001 to 0.01
        let scaledLength = velLength * 500.0;
        let dir = velocity / max(velLength, 0.0001);
        // Color based on direction (vec2)
        let color = dir * 0.5 + 0.5;
        // Use logarithmic scale for better visibility across magnitudes
        let intensity = min(scaledLength, 1.0);
        // Add a base brightness so even zero-velocity pixels are distinguishable from background
        let finalColor = mix(vec3<f32>(0.01), vec3<f32>(color, 0.0), intensity);
        return vec4<f32>(finalColor.x, finalColor.y, finalColor.z, 1.0);
    }

    // Mode 4: Prepass - visualize full prepass normal/roughness buffer
    if (mode == 4u) {
        let prepass = textureSample(t_prepass_normal, s_linear, uv);
        // RGB = view-space normal, A = roughness
        let normal = normalize(prepass.rgb);
        let color = normal * 0.5 + 0.5;
        // Overlay roughness as brightness
        let roughness = prepass.a;
        let finalColor = mix(color, vec3<f32>(roughness), 0.3);
        return vec4<f32>(finalColor.x, finalColor.y, finalColor.z, 1.0);
    }

    // Fallback
    return textureSample(t_main_color, s_linear, uv);
}
