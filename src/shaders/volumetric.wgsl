struct Camera {
    view_pos: vec4<f32>,
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    view_proj: mat4x4<f32>,
    unjittered_proj: mat4x4<f32>,
    unjittered_view_proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    inv_view: mat4x4<f32>,
    inv_view_proj: mat4x4<f32>,
    inv_unjittered_view_proj: mat4x4<f32>,
    prev_view_proj: mat4x4<f32>,
    jitter: vec4<f32>,
    ssao_enabled: u32,
    volumetric_enabled: u32,
    taa_enabled: u32,
    frame_count: u32,
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

struct DirectionalLight {
    direction: vec3<f32>,
    strength: f32,
    color: vec3<f32>,
    shadow_distance: f32,
}

struct Lights {
    ambient_color: vec3<f32>,
    ambient_strength: f32,
    directional_light: DirectionalLight,
    fog_color: vec3<f32>,
    fog_density: f32,
    fog_height_falloff: f32,
    fog_base_height: f32,
    fog_scattering: f32,
    fog_absorption: f32,
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

struct CascadeUniform {
    view_proj: array<mat4x4<f32>, 3>,
    splits: vec4<f32>,
}

@group(0) @binding(0) var<uniform> camera: Camera;
@group(0) @binding(1) var<uniform> lights: Lights;
@group(0) @binding(2) var<storage, read> all_point_lights: array<PointLight>;
@group(0) @binding(3) var<uniform> config: ClusterConfig;
@group(0) @binding(4) var t_volumetric: texture_storage_3d<rgba16float, write>;
@group(0) @binding(5) var t_point_shadow: texture_depth_cube_array;
@group(0) @binding(6) var s_shadow: sampler_comparison;
@group(0) @binding(7) var t_dir_shadow: texture_depth_2d_array;
@group(0) @binding(8) var<uniform> cascade_uniform: CascadeUniform;

const PI: f32 = 3.14159265359;

fn get_world_pos(uv: vec2<f32>, view_z: f32) -> vec3<f32> {
    let clip_x = uv.x * 2.0 - 1.0;
    let clip_y = (1.0 - uv.y) * 2.0 - 1.0;

    // 体积光计算在稳定的低分辨率网格上进行，必须使用不带抖动的矩阵
    let clip_z = (camera.unjittered_proj[2][2] * view_z + camera.unjittered_proj[3][2]) / -view_z;
    let clip_pos = vec4<f32>(clip_x, clip_y, clip_z, 1.0);

    // 直接使用稳定的逆矩阵
    let world_pos_h = camera.inv_unjittered_view_proj * clip_pos;
    return world_pos_h.xyz / world_pos_h.w;
}

fn interleaved_gradient_noise(uv: vec2<f32>, frame: u32) -> f32 {
    let magic = vec3<f32>(0.06711056, 0.00583715, 52.9829189);
    // 加入金分割序列的帧偏移，使噪声在时间上分布均匀
    let frame_offset = fract(f32(frame % 16u) * 0.61803398875);
    return fract(magic.z * fract(dot(uv, magic.xy) + frame_offset));
}

// Henyey-Greenstein Phase Function
fn phase_hg(cos_theta: f32, g: f32) -> f32 {
    let g2 = g * g;
    return (1.0 - g2) / (4.0 * PI * pow(1.0 + g2 - 2.0 * g * cos_theta, 1.5));
}

fn get_fog_density(world_pos: vec3<f32>) -> f32 {
    // 指数高度雾: 模拟地面附近的浓雾
    return lights.fog_density * exp(-lights.fog_height_falloff * max(world_pos.y - lights.fog_base_height, 0.0));
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let res = textureDimensions(t_volumetric);
    if (global_id.x >= res.x || global_id.y >= res.y) { return; }

    let screen_uv = (vec2<f32>(global_id.xy) + 0.5) / vec2<f32>(res.xy);

    // 智能抖动逻辑：
    // 如果开启了 TAA，使用随时间跳变的帧号来打破规律性，让 TAA 能够平滑它。
    // 如果关闭了 TAA，使用固定的帧号（0），让噪点保持静止，避免画面闪烁。
    var effective_frame = 0u;
    if (camera.taa_enabled == 1u) {
        effective_frame = camera.frame_count;
    }
    let jitter = interleaved_gradient_noise(vec2<f32>(global_id.xy), effective_frame);

    var accumulated_scattering = vec3<f32>(0.0);
    var accumulated_transmittance = 1.0;

    let z_near = config.z_near;
    let z_far = config.z_far;
    let grid_z = f32(res.z);

    // 视角方向 (从摄像机指向像素)
    let view_dir = normalize(get_world_pos(screen_uv, -1.0) - camera.view_pos.xyz);

    for (var z = 0u; z < res.z; z++) {
        // 对数深度分布
        let z0 = -z_near * pow(z_far / z_near, f32(z) / grid_z);
        let z1 = -z_near * pow(z_far / z_near, f32(z + 1u) / grid_z);
        let step_size = abs(z1 - z0);

        let sample_z = mix(z0, z1, jitter);
        let world_pos = get_world_pos(screen_uv, sample_z);
        let density = get_fog_density(world_pos);

        if (density <= 0.0) {
             accumulated_transmittance *= 1.0;
             textureStore(t_volumetric, vec3<u32>(global_id.xy, z), vec4<f32>(accumulated_scattering, accumulated_transmittance));
             continue;
        }

        var local_scattering = vec3<f32>(0.0);

        // 1. 定向光 (主光源)
        {
            let light_dir = normalize(-lights.directional_light.direction);
            let cos_theta = dot(view_dir, light_dir);
            let phase = phase_hg(cos_theta, 0.7); // 适中的各向异性

            let depth = -sample_z;
            var cascade_index: u32 = 2u;
            if (depth < cascade_uniform.splits.x) { cascade_index = 0u; }
            else if (depth < cascade_uniform.splits.y) { cascade_index = 1u; }

            let shadow_coords = cascade_uniform.view_proj[cascade_index] * vec4<f32>(world_pos, 1.0);
            let shadow_pos = shadow_coords.xyz / shadow_coords.w;
            let shadow_uv = shadow_pos.xy * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5, 0.5);

            var shadow = 1.0;
            if (shadow_pos.x >= -1.0 && shadow_pos.x <= 1.0 && shadow_pos.y >= -1.0 && shadow_pos.y <= 1.0 && shadow_pos.z >= 0.0 && shadow_pos.z <= 1.0) {
                // 显著减小阴影偏移 (Bias)。体积光对 Shadow Acne 不敏感，但对 Peter Panning (阴影断裂) 极其敏感。
                // 这里的偏移值应尽可能小。
                shadow = textureSampleCompareLevel(t_dir_shadow, s_shadow, shadow_uv, i32(cascade_index), shadow_pos.z - 0.0005);
            }
            local_scattering += lights.directional_light.color * lights.directional_light.strength * phase * shadow;
        }

        // 2. 点光源
        for (var i = 0u; i < min(config.num_lights, 4u); i++) {
            let light = all_point_lights[i];
            let light_vec = light.position - world_pos;
            let dist = length(light_vec);

            if (dist < light.radius) {
                let light_dir = light_vec / dist;
                let cos_theta = dot(view_dir, light_dir);
                let phase = phase_hg(cos_theta, 0.5);

                // 物理衰减
                let dist_ratio = dist / light.radius;
                let attenuation = pow(max(1.0 - pow(dist_ratio, 4.0), 0.0), 2.0) / (dist * dist + 1.0);

                let light_to_vox = (world_pos - light.position) * vec3<f32>(1.0, 1.0, -1.0);
                let dist_along_axis = max(abs(light_to_vox.x), max(abs(light_to_vox.y), abs(light_to_vox.z)));
                let shadow_z = (light.shadow_far / (light.shadow_far - light.shadow_near)) -
                               ((light.shadow_far * light.shadow_near) / (light.shadow_far - light.shadow_near)) / dist_along_axis;

                // 减小点光源偏移
                let shadow = textureSampleCompareLevel(t_point_shadow, s_shadow, light_to_vox, i32(i), shadow_z - 0.001);
                local_scattering += light.color * light.strength * attenuation * phase * shadow;
            }
        }

        // 3. 环境光贡献 (环境散射)
        let ambient_scattering = lights.fog_color * lights.ambient_strength * 0.1;
        local_scattering += ambient_scattering;

        // 4. 体积参数
        let scattering_coeff = lights.fog_scattering;
        let absorption_coeff = lights.fog_absorption;

        let current_scattering = local_scattering * scattering_coeff * density;
        let current_extinction = (scattering_coeff + absorption_coeff) * density;

        // 5. 能量守恒积分
        let step_transmittance = exp(-current_extinction * step_size);

        if (current_extinction > 0.0001) {
            let scattering_integral = (current_scattering / current_extinction) * (1.0 - step_transmittance);
            accumulated_scattering += scattering_integral * accumulated_transmittance;
        } else {
            accumulated_scattering += current_scattering * step_size * accumulated_transmittance;
        }

        accumulated_transmittance *= step_transmittance;

        textureStore(t_volumetric, vec3<u32>(global_id.xy, z), vec4<f32>(accumulated_scattering, accumulated_transmittance));
    }
}
