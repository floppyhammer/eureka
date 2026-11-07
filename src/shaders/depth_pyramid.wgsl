struct DownsampleParams {
    src_width: u32,
    src_height: u32,
    depth_mode: u32, // 0: Standard Depth (0=Near, 1=Far), 1: Reverse-Z (1=Near, 0=Far)
    _pad: u32,
}

// Pass 0: 拷贝/读取 Depth Prepass (Depth32Float) 并写入 Hi-Z Mip 0 (R32Float)
@group(0) @binding(0) var u_main_depth: texture_depth_2d;
@group(0) @binding(1) var u_hiz_mip0: texture_storage_2d<r32float, write>;

@compute @workgroup_size(16, 16, 1)
fn init_mip0(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let coords = vec2<i32>(global_id.xy);
    let dims = vec2<i32>(textureDimensions(u_main_depth));

    if (coords.x >= dims.x || coords.y >= dims.y) {
        return;
    }

    let depth = textureLoad(u_main_depth, coords, 0);
    textureStore(u_hiz_mip0, coords, vec4<f32>(depth, 0.0, 0.0, 0.0));
}

// Pass 1..N: 从上一级 Mip 下采样 2x2 到当前 Mip
@group(0) @binding(0) var u_src_depth: texture_2d<f32>;
@group(0) @binding(1) var u_dst_depth: texture_storage_2d<r32float, write>;
@group(0) @binding(2) var<uniform> params: DownsampleParams;

fn merge_depth(d0: f32, d1: f32, d2: f32, d3: f32, depth_mode: u32) -> f32 {
    if (depth_mode == 0u) {
        // Standard Depth: 保留最远深度 (Max Depth)
        return max(max(d0, d1), max(d2, d3));
    } else {
        // Reverse-Z: 保留最远深度 (Min Depth)
        return min(min(d0, d1), min(d2, d3));
    }
}

@compute @workgroup_size(16, 16, 1)
fn downsample(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let dst_coords = vec2<i32>(global_id.xy);
    let src_base = dst_coords * 2;

    let max_src_x = i32(params.src_width) - 1;
    let max_src_y = i32(params.src_height) - 1;

    let c0 = min(src_base, vec2<i32>(max_src_x, max_src_y));
    let c1 = min(src_base + vec2<i32>(1, 0), vec2<i32>(max_src_x, max_src_y));
    let c2 = min(src_base + vec2<i32>(0, 1), vec2<i32>(max_src_x, max_src_y));
    let c3 = min(src_base + vec2<i32>(1, 1), vec2<i32>(max_src_x, max_src_y));

    let d0 = textureLoad(u_src_depth, c0, 0).r;
    let d1 = textureLoad(u_src_depth, c1, 0).r;
    let d2 = textureLoad(u_src_depth, c2, 0).r;
    let d3 = textureLoad(u_src_depth, c3, 0).r;

    let conservative_depth = merge_depth(d0, d1, d2, d3, params.depth_mode);

    let dst_dims = vec2<i32>(textureDimensions(u_dst_depth));
    if (dst_coords.x < dst_dims.x && dst_coords.y < dst_dims.y) {
        textureStore(u_dst_depth, dst_coords, vec4<f32>(conservative_depth, 0.0, 0.0, 0.0));
    }
}
