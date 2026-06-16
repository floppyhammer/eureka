struct Camera {
    view_pos: vec4<f32>,
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    view_proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    ssao_enabled: u32,
}

struct PointLight {
    position: vec3<f32>,
    strength: f32,
    color: vec3<f32>,
    radius: f32,
    shadow_near: f32,
    shadow_far: f32,
    _pad: vec2<f32>,
}

struct Cluster {
    offset: u32,
    count: u32,
}

struct ClusterConfig {
    screen_size: vec2<f32>,
    _pad0: vec2<f32>,
    grid_size: vec3<u32>,
    num_lights: u32,
    z_near: f32,
    z_far: f32,
    _pad1: vec2<f32>,
}

@group(0) @binding(0) var<uniform> camera: Camera;
@group(0) @binding(1) var<storage, read> all_point_lights: array<PointLight>;
@group(0) @binding(2) var<storage, read_write> light_grid: array<Cluster>;
@group(0) @binding(3) var<storage, read_write> light_index_list: array<u32>;
@group(0) @binding(4) var<storage, read_write> global_index_count: atomic<u32>;
@group(0) @binding(5) var<uniform> config: ClusterConfig;

fn screen_to_view(screen_pos: vec4<f32>) -> vec4<f32> {
    let tex_coord = screen_pos.xy / config.screen_size;
    let clip = vec4<f32>(
        tex_coord.x * 2.0 - 1.0,
        (1.0 - tex_coord.y) * 2.0 - 1.0,
        screen_pos.z,
        screen_pos.w
    );
    let view = camera.inv_proj * clip;
    return view / view.w;
}

@compute @workgroup_size(16, 9, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    // 每一个线程处理一个 Z 层级的 Cluster
    let g_id = global_id;

    // 我们让 workgroup 处理 XY 平面，循环处理 Z 轴，这样可以复用一些计算
    for (var z = 0u; z < config.grid_size.z; z++) {
        let cluster_idx = g_id.x +
                          g_id.y * config.grid_size.x +
                          z * config.grid_size.x * config.grid_size.y;

        // Step 1: AABB Calculation
        let tile_size = config.screen_size / vec2<f32>(config.grid_size.xy);
        let z_near = config.z_near;
        let z_far = config.z_far;
        let p0_z = -z_near * pow(z_far / z_near, f32(z) / f32(config.grid_size.z));
        let p1_z = -z_near * pow(z_far / z_near, f32(z + 1u) / f32(config.grid_size.z));

        let min_p_screen = vec2<f32>(g_id.xy) * tile_size;
        let max_p_screen = vec2<f32>(g_id.xy + 1u) * tile_size;

        let p0 = screen_to_view(vec4<f32>(min_p_screen, 0.0, 1.0));
        let p1 = screen_to_view(vec4<f32>(max_p_screen, 0.0, 1.0));
        let p2 = screen_to_view(vec4<f32>(min_p_screen, 1.0, 1.0));
        let p3 = screen_to_view(vec4<f32>(max_p_screen, 1.0, 1.0));

        let min_v = min(min(p0.xy * (p0_z / p0.z), p1.xy * (p0_z / p1.z)), min(p2.xy * (p0_z / p2.z), p3.xy * (p0_z / p3.z)));
        let max_v = max(max(p0.xy * (p0_z / p0.z), p1.xy * (p0_z / p1.z)), max(p2.xy * (p0_z / p2.z), p3.xy * (p0_z / p3.z)));
        let min_v2 = min(min(p0.xy * (p1_z / p0.z), p1.xy * (p1_z / p1.z)), min(p2.xy * (p1_z / p2.z), p3.xy * (p1_z / p3.z)));
        let max_v2 = max(max(p0.xy * (p1_z / p0.z), p1.xy * (p1_z / p1.z)), max(p2.xy * (p1_z / p1.z), p3.xy * (p1_z / p3.z)));

        let cluster_min = vec3<f32>(min(min_v, min_v2), min(p0_z, p1_z));
        let cluster_max = vec3<f32>(max(max_v, max_v2), max(p0_z, p1_z));

        // Step 2: Culling
        var visible_light_count = 0u;
        var visible_light_indices: array<u32, 64>; // 降低单 Cluster 上限以节省寄存器

        for (var i = 0u; i < config.num_lights; i++) {
            let light = all_point_lights[i];
            let light_view_pos = (camera.view * vec4<f32>(light.position, 1.0)).xyz;

            let delta = max(cluster_min - light_view_pos, vec3<f32>(0.0)) + max(light_view_pos - cluster_max, vec3<f32>(0.0));
            if (dot(delta, delta) <= (light.radius * light.radius)) {
                visible_light_indices[visible_light_count] = i;
                visible_light_count++;
                if (visible_light_count >= 64u) { break; }
            }
        }

        // Step 3: Write
        let offset = atomicAdd(&global_index_count, visible_light_count);
        light_grid[cluster_idx].offset = offset;
        light_grid[cluster_idx].count = visible_light_count;

        for (var i = 0u; i < visible_light_count; i++) {
            light_index_list[offset + i] = visible_light_indices[i];
        }
    }
}
