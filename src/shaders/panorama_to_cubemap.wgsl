// src/shaders/panorama_to_cubemap.wgsl

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var pos = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0)
    );
    var out: VertexOutput;
    out.position = vec4<f32>(pos[vertex_index], 0.0, 1.0);
    out.uv = pos[vertex_index] * 0.5 + 0.5;
    out.uv.y = 1.0 - out.uv.y;
    return out;
}

@group(0) @binding(0) var panorama_texture: texture_2d<f32>;
@group(0) @binding(1) var panorama_sampler: sampler;
@group(0) @binding(2) var<uniform> face_index: u32;

const PI: f32 = 3.14159265359;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // 将 UV 转换为 [-1, 1] 范围的坐标
    let tex_coord = in.uv * 2.0 - 1.0;

    var ray_dir: vec3<f32>;

    // 根据当前渲染的面，确定射线方向 (ray direction)
    // 0: +X, 1: -X, 2: +Y, 3: -Y, 4: +Z, 5: -Z
    switch (face_index) {
        case 0u: { ray_dir = vec3<f32>(1.0, -tex_coord.y, -tex_coord.x); }
        case 1u: { ray_dir = vec3<f32>(-1.0, -tex_coord.y, tex_coord.x); }
        case 2u: { ray_dir = vec3<f32>(tex_coord.x, 1.0, tex_coord.y); }
        case 3u: { ray_dir = vec3<f32>(tex_coord.x, -1.0, -tex_coord.y); }
        case 4u: { ray_dir = vec3<f32>(tex_coord.x, -tex_coord.y, 1.0); }
        case 5u: { ray_dir = vec3<f32>(-tex_coord.x, -tex_coord.y, -1.0); }
        default: { ray_dir = vec3<f32>(1.0, 0.0, 0.0); }
    }

    let normalized_dir = normalize(ray_dir);

    // 将 3D 方向转换为全景图的 2D 坐标 (Equirectangular mapping)
    let phi = atan2(normalized_dir.z, normalized_dir.x);
    let theta = asin(normalized_dir.y);

    var uv = vec2<f32>(
        (phi + PI) / (2.0 * PI),
        1.0 - (theta + PI / 2.0) / PI
    );

    return textureSample(panorama_texture, panorama_sampler, uv);
}
