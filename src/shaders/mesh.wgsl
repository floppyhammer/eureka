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
    ssr_enabled: u32,
    frame_count: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
}

@group(0) @binding(0)
var<uniform> camera: Camera;

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

struct Cluster {
    offset: u32,
    count: u32,
}

struct ClusterConfig {
    screen_size: vec2<f32>,
    _pad0: vec2<f32>,
    grid_size: vec3<u32>,
    _pad1: u32,
    z_near: f32,
    z_far: f32,
}

@group(1) @binding(0)
var<uniform> lights: Lights;

@group(1) @binding(1)
var t_shadow_cascade: texture_depth_2d_array;
@group(1) @binding(2)
var s_shadow: sampler_comparison;

struct CascadeUniform {
    view_proj: array<mat4x4<f32>, 3>,
    splits: vec4<f32>,
}
@group(1) @binding(3)
var<uniform> cascade_uniform: CascadeUniform;

@group(1) @binding(4)
var t_point_shadow: texture_depth_cube_array;
@group(1) @binding(5)
var t_ssao: texture_2d<f32>;
@group(1) @binding(6)
var t_skybox: texture_cube<f32>;
@group(1) @binding(7)
var s_skybox: sampler;

@group(1) @binding(8)
var<storage, read> all_point_lights: array<PointLight>;
@group(1) @binding(9)
var<storage, read> light_grid: array<Cluster>;
@group(1) @binding(10)
var<storage, read> light_index_list: array<u32>;
@group(1) @binding(11)
var<uniform> cluster_config: ClusterConfig;

@group(1) @binding(12)
var t_volumetric: texture_3d<f32>;

@group(1) @binding(13)
var t_irradiance: texture_cube<f32>;
@group(1) @binding(14)
var t_brdf_lut: texture_2d<f32>;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) normal: vec3<f32>,
    @location(3) tangent: vec3<f32>,
    @location(4) bitangent: vec3<f32>,
}

struct InstanceInput {
    // Model matrix.
    @location(5) model_matrix_0: vec4<f32>,
    @location(6) model_matrix_1: vec4<f32>,
    @location(7) model_matrix_2: vec4<f32>,
    @location(8) model_matrix_3: vec4<f32>,

    // Normal matrix.
    @location(9) normal_matrix_0: vec4<f32>,
    @location(10) normal_matrix_1: vec4<f32>,
    @location(11) normal_matrix_2: vec4<f32>,

    @location(12) material_idx: u32,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) world_position: vec4<f32>,
    @location(2) world_tangent: vec3<f32>,
    @location(3) world_bitangent: vec3<f32>,
    @location(4) world_normal: vec3<f32>,
    @location(5) @interpolate(flat) material_idx: u32,
}

@vertex
fn vs_main(vertex: VertexInput, instance: InstanceInput) -> VertexOutput {
    let model_matrix = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3);

    let normal_matrix = mat3x3<f32>(
        instance.normal_matrix_0.xyz,
        instance.normal_matrix_1.xyz,
        instance.normal_matrix_2.xyz);

    let world_normal = normalize(normal_matrix * vertex.normal);
    let world_tangent = normalize(normal_matrix * vertex.tangent);
    let world_bitangent = normalize(normal_matrix * vertex.bitangent);

    // Vertex's world position.
    let vertex_world_position = model_matrix * vec4<f32>(vertex.position, 1.0);

    var out: VertexOutput;
    out.clip_position = camera.view_proj * vertex_world_position;
    out.tex_coords = vertex.tex_coords;
    out.world_position = vertex_world_position;
    out.world_tangent = world_tangent;
    out.world_bitangent = world_bitangent;
    out.world_normal = world_normal;
    out.material_idx = instance.material_idx;

    return out;
}

// Fragment shader //

// Texture bind group (Bindless).
// -------------------------
struct Material {
    base_color: vec4<f32>,
    emissive: vec3<f32>,
    emissive_strength: f32,
    metallic: f32,
    roughness: f32,
    alpha_cutoff: f32,
    color_texture_idx: i32,
    normal_texture_idx: i32,
    metallic_roughness_texture_idx: i32,
    occlusion_texture_idx: i32,
    emissive_texture_idx: i32,
    alpha_mode: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
}

const ALPHA_MODE_OPAQUE: u32 = 0u;
const ALPHA_MODE_MASK: u32 = 1u;
const ALPHA_MODE_BLEND: u32 = 2u;

@group(2) @binding(0)
var<storage, read> materials: array<Material>;

@group(2) @binding(1)
var t_textures: binding_array<texture_2d<f32>>;
@group(2) @binding(2)
var s_sampler: sampler;
// -------------------------

// -------------------------
// PBR Core Functions
// -------------------------

const PI: f32 = 3.14159265359;

// D: Trowbridge-Reitz GGX (Normal Distribution Function)
fn distribution_ggx(N: vec3<f32>, H: vec3<f32>, roughness: f32) -> f32 {
    let a = roughness * roughness;
    let a2 = a * a;
    let n_dot_h = max(dot(N, H), 0.0);
    let n_dot_h2 = n_dot_h * n_dot_h;

    let num = a2;
    var denom = (n_dot_h2 * (a2 - 1.0) + 1.0);
    denom = PI * denom * denom;

    // Use a slightly larger epsilon for the denominator
    return num / max(denom, 0.00001);
}

// G: Smith's method with Schlick-GGX
fn geometry_schlick_ggx(n_dot_v: f32, roughness: f32) -> f32 {
    let r = (roughness + 1.0);
    let k = (r * r) / 8.0;

    let num = n_dot_v;
    let denom = n_dot_v * (1.0 - k) + k;

    return num / denom;
}

fn geometry_smith(N: vec3<f32>, V: vec3<f32>, L: vec3<f32>, roughness: f32) -> f32 {
    let n_dot_v = max(dot(N, V), 0.0);
    let n_dot_l = max(dot(N, L), 0.0);
    let ggx2 = geometry_schlick_ggx(n_dot_v, roughness);
    let ggx1 = geometry_schlick_ggx(n_dot_l, roughness);

    return ggx1 * ggx2;
}

// F: Fresnel-Schlick Equation
fn fresnel_schlick(cos_theta: f32, F0: vec3<f32>) -> vec3<f32> {
    return F0 + (1.0 - F0) * pow(clamp(1.0 - cos_theta, 0.0, 1.0), 5.0);
}

// Fresnel Schlick with roughness (Lagarde's modification for indirect lighting)
fn fresnel_schlick_roughness(cos_theta: f32, F0: vec3<f32>, roughness: f32) -> vec3<f32> {
    return F0 + (max(vec3<f32>(1.0 - roughness), F0) - F0) * pow(clamp(1.0 - cos_theta, 0.0, 1.0), 5.0);
}

fn hash_2d(p: vec2<f32>) -> f32 {
    let p3 = fract(vec3<f32>(p.xyx) * 0.1031);
    let p3_2 = p3 + dot(p3, p3.yzx + 33.33);
    return fract((p3_2.x + p3_2.y) * p3_2.z);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let material = materials[in.material_idx];

    // Sample diffuse texture.
    var sampled_color: vec4<f32> = vec4<f32>(1.0, 1.0, 1.0, 1.0);
    if (material.color_texture_idx >= 0) {
        sampled_color = textureSample(t_textures[u32(material.color_texture_idx)], s_sampler, in.tex_coords);
    }
    let object_color = sampled_color * material.base_color;

    // Alpha test for MASK mode
    if (material.alpha_mode == ALPHA_MODE_MASK) {
        if (object_color.a < material.alpha_cutoff) {
            discard;
        }
    }

    // Reconstruct TBN matrix (Tangent to World)
    let world_normal_basis = normalize(in.world_normal);
    let world_tangent = normalize(in.world_tangent);
    let world_bitangent = normalize(in.world_bitangent);
    let tbn_to_world = mat3x3<f32>(
        world_tangent,
        world_bitangent,
        world_normal_basis
    );

    var world_normal = world_normal_basis;
    if (material.normal_texture_idx >= 0) {
        let normal_map = textureSample(t_textures[u32(material.normal_texture_idx)], s_sampler, in.tex_coords).xyz * 2.0 - 1.0;
        world_normal = normalize(tbn_to_world * normal_map);
    }

    // PBR Parameters (From Material Uniforms)
    var metallic: f32 = material.metallic;
    var roughness: f32 = material.roughness;

    if (material.metallic_roughness_texture_idx >= 0) {
        let mr_sample = textureSample(t_textures[u32(material.metallic_roughness_texture_idx)], s_sampler, in.tex_coords);
        // glTF standard: Metallic is B channel, Roughness is G channel
        metallic = mr_sample.b;
        roughness = mr_sample.g;
    }

    // --- DEBUG: Uncomment to SEE the metallic and roughness factor ---
    // return vec4<f32>(vec3<f32>(roughness), 1.0);
    // return vec4<f32>(vec3<f32>(metallic), 1.0);

    // Clamp roughness to a safe minimum to prevent specular highlight disappearing
    // and avoid division by zero in BRDF equations.
    roughness = max(roughness, 0.045);

    var material_ao = 1.0;
    if (material.occlusion_texture_idx >= 0) {
        material_ao = textureSample(t_textures[u32(material.occlusion_texture_idx)], s_sampler, in.tex_coords).r;
    }

    var ambient_ao = 1.0;
    if (camera.ssao_enabled == 1u) {
        // Ensure we stay within bounds and handle potential fractional pixel offsets
        let ssao_coords = vec2<i32>(in.clip_position.xy);
        ambient_ao = textureLoad(t_ssao, ssao_coords, 0).r;
    }

    // Uncomment to debug ambient_ao factor
    // return vec4<f32>(vec3<f32>(ambient_ao), 1.0);

    let view_to_frag = camera.view_pos.xyz - in.world_position.xyz;
    let view_dir = normalize(view_to_frag + vec3<f32>(0.00001)); // Add epsilon to prevent NaN

    // F0: Surface reflection at zero incidence
    // For non-metals, we use 0.04. For metals, we use the object color.
    var F0 = vec3<f32>(0.04);
    F0 = mix(F0, object_color.xyz, metallic);

    // --- Clustered Point Lights ---
    let view_pos_for_cluster = camera.view * in.world_position;
    let cluster_depth = -view_pos_for_cluster.z;

    // Calculate Cluster Index
    let x_slice = min(u32(in.clip_position.x / (cluster_config.screen_size.x / f32(cluster_config.grid_size.x))), cluster_config.grid_size.x - 1u);
    let y_slice = min(u32(in.clip_position.y / (cluster_config.screen_size.y / f32(cluster_config.grid_size.y))), cluster_config.grid_size.y - 1u);
    let z_slice = min(u32(max(log2(max(cluster_depth, cluster_config.z_near) / cluster_config.z_near) * f32(cluster_config.grid_size.z) / log2(cluster_config.z_far / cluster_config.z_near), 0.0)), cluster_config.grid_size.z - 1u);

    let cluster_idx = x_slice + y_slice * cluster_config.grid_size.x + z_slice * cluster_config.grid_size.x * cluster_config.grid_size.y;
    let cluster = light_grid[cluster_idx];

    var point_lights_result = vec3<f32>(0.0, 0.0, 0.0);
    for (var i: u32 = 0u; i < cluster.count; i++) {
        let light_idx = light_index_list[cluster.offset + i];
        let light = all_point_lights[light_idx];

        let light_vec = light.position - in.world_position.xyz;
        let distance = length(light_vec);
        let light_dir = normalize(light_vec);
        let half_dir = normalize(view_dir + light_dir);
        let n_dot_l = max(dot(world_normal, light_dir), 0.0);

        // Adaptive Bias
        let bias = max(0.005 * (1.0 - n_dot_l), 0.0005);

        let dist_vec = abs(light_vec);
        let dist_along_axis = max(dist_vec.x, max(dist_vec.y, dist_vec.z));
        let near = light.shadow_near;
        let far = light.shadow_far;
        let shadow_z = (far / (far - near)) - ((far * near) / (far - near)) / dist_along_axis;
        let final_shadow_z = clamp(shadow_z, 0.0, 1.0);
        let light_to_frag = (in.world_position.xyz - light.position) * vec3<f32>(1.0, 1.0, -1.0);

        // Only calculate shadows for the first N lights that have shadow slots
        var shadow_factor = 1.0;
        if (light_idx < 4u) {
            shadow_factor = textureSampleCompare(t_point_shadow, s_shadow, light_to_frag, i32(light_idx), final_shadow_z - bias);
        }

        // Cook-Torrance BRDF
        let NDF = distribution_ggx(world_normal, half_dir, roughness);
        let G = geometry_smith(world_normal, view_dir, light_dir, roughness);
        let F = fresnel_schlick(max(dot(half_dir, view_dir), 0.0), F0);

        let numerator = NDF * G * F;
        let denominator = 4.0 * max(dot(world_normal, view_dir), 0.0) * max(dot(world_normal, light_dir), 0.0) + 0.0001;
        let specular = numerator / denominator;

        let kS = F;
        var kD = vec3<f32>(1.0) - kS;
        kD *= 1.0 - metallic;

        // 现代基于半径的衰减公式 (Physically Based)
        let dist_ratio = distance / light.radius;
        let attenuation = pow(saturate(1.0 - pow(dist_ratio, 4.0)), 2.0) / (distance * distance + 1.0);
        let radiance = light.color * light.strength * attenuation;

        point_lights_result += (kD * object_color.xyz / PI + specular) * radiance * n_dot_l * shadow_factor;
    }

    var directional_light_result = vec3<f32>(0.0, 0.0, 0.0);
    {
        let light_dir = normalize(-lights.directional_light.direction);
        let half_dir = normalize(view_dir + light_dir);

        // CSM Shadow mapping
        let view_pos = camera.view * in.world_position;
        let depth = -view_pos.z;
        var cascade_index: u32 = 2u;
        if (depth < cascade_uniform.splits.x) {
            cascade_index = 0u;
        } else if (depth < cascade_uniform.splits.y) {
            cascade_index = 1u;
        }
        let shadow_coords = cascade_uniform.view_proj[cascade_index] * in.world_position;
        let shadow_pos = shadow_coords.xyz / shadow_coords.w;
        let shadow_uv = shadow_pos.xy * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5, 0.5);

        var shadow_factor = 1.0;
        let n_dot_l_geo = dot(world_normal_basis, light_dir);

        if (shadow_pos.x >= -1.0 && shadow_pos.x <= 1.0 && shadow_pos.y >= -1.0 && shadow_pos.y <= 1.0 && shadow_pos.z >= 0.0 && shadow_pos.z <= 1.0) {
            // Adaptive bias to prevent shadow acne and moving stripes
            let bias = max(0.002 * (1.0 - n_dot_l_geo), 0.0005);

            // 3x3 PCF (Percentage Closer Filtering)
            var shadow_sum = 0.0;
            let texel_size = 1.0 / vec2<f32>(textureDimensions(t_shadow_cascade).xy);
            for (var y: f32 = -1.0; y <= 1.0; y += 1.0) {
                for (var x: f32 = -1.0; x <= 1.0; x += 1.0) {
                    let offset = vec2<f32>(x, y) * texel_size;
                    shadow_sum += textureSampleCompare(t_shadow_cascade, s_shadow, shadow_uv + offset, i32(cascade_index), shadow_pos.z - bias);
                }
            }
            shadow_factor = shadow_sum / 9.0;

            // Smoothly fade out shadows at the terminator to prevent hard black stripes on spheres
            shadow_factor *= saturate(n_dot_l_geo * 5.0);
        }

        // Cook-Torrance BRDF for Directional Light
        let NDF = distribution_ggx(world_normal, half_dir, roughness);
        let G = geometry_smith(world_normal, view_dir, light_dir, roughness);
        let F = fresnel_schlick(max(dot(half_dir, view_dir), 0.0), F0);

        let numerator = NDF * G * F;
        let denominator = 4.0 * max(dot(world_normal, view_dir), 0.0) * max(dot(world_normal, light_dir), 0.0) + 0.0001;
        let specular = numerator / denominator;

        let kS = F;
        var kD = vec3<f32>(1.0) - kS;
        kD *= 1.0 - metallic;

        let n_dot_l = max(dot(world_normal, light_dir), 0.0);
        let radiance = lights.directional_light.color * lights.directional_light.strength;

        directional_light_result = (kD * object_color.xyz / PI + specular) * radiance * n_dot_l * shadow_factor;
    }

    // --- IBL (Image Based Lighting) ---
    let reflect_dir = reflect(-view_dir, world_normal);
    let n_dot_v = max(dot(world_normal, view_dir), 0.0001);

    // 1. Specular part (Prefiltered Environment Map)
    // Using 9.0 as MAX_REFLECTION_LOD as it matches the previous skybox blur logic
    let prefiltered_color = textureSampleLevel(t_skybox, s_skybox, reflect_dir, roughness * 9.0).rgb;
    let env_brdf = textureSample(t_brdf_lut, s_skybox, vec2<f32>(n_dot_v, roughness)).rg;

    // Use the roughness-aware Fresnel for indirect specular
    let F_env = fresnel_schlick_roughness(n_dot_v, F0, roughness);
    let indirect_specular_base = prefiltered_color * (F_env * env_brdf.x + env_brdf.y);

    // --- Improved Specular Occlusion ---
    // Physically based specular occlusion (Frostbite/UE4 style)
    var spec_occlusion = saturate(pow(ambient_ao + n_dot_v, ambient_ao) - 1.0 + ambient_ao);
    spec_occlusion = pow(spec_occlusion, 1.0 + roughness * 4.0);
    spec_occlusion = smoothstep(0.0, 0.8, spec_occlusion);

    let combined_ao = material_ao * ambient_ao;
    let indirect_specular = indirect_specular_base * spec_occlusion * combined_ao;

    // 2. Diffuse part (Irradiance Map)
    let kS_env = F_env;
    var kD_env = vec3<f32>(1.0) - kS_env;
    kD_env *= 1.0 - metallic;

    let irradiance = textureSample(t_irradiance, s_skybox, world_normal).rgb;
    let indirect_diffuse = kD_env * irradiance * object_color.xyz * combined_ao;
    // --- End IBL ---

    // --- Emissive ---
    var emissive = material.emissive * material.emissive_strength;
    if (material.emissive_texture_idx >= 0) {
        emissive *= textureSample(t_textures[u32(material.emissive_texture_idx)], s_sampler, in.tex_coords).rgb;
    }

    let result = indirect_diffuse + indirect_specular + point_lights_result + directional_light_result + emissive;

    // --- Volumetric Lighting Application ---
    var final_rgb = result;

    // 关键逻辑：只有透明物体（AlphaMode::Blend = 2）才在 Shader 中采样体积光
    // 不透明物体由后续的 VolumetricApply 节点统一处理
    if (material.alpha_mode == 2u) {
        // 透明物体采样体积光时，必须使用不带抖动的 UV 和坐标
        let screen_uv = in.clip_position.xy / cluster_config.screen_size;
        let stable_uv = screen_uv - camera.jitter.xy * 0.5;

        let volumetric_uvw = vec3<f32>(
            stable_uv,
            log2(max(cluster_depth, cluster_config.z_near) / cluster_config.z_near) / log2(cluster_config.z_far / cluster_config.z_near)
        );
        let volumetric_data = textureSampleLevel(t_volumetric, s_skybox, volumetric_uvw, 0.0);

        // 物理混合修复：
        // 由于背景已经由 VolumetricApply 渲染了雾效，我们需要让透明物体输出
        // (物体颜色 * 透射率 + 散射光) * Alpha。
        // 这样在 SrcFactor = One, DstFactor = OneMinusSrcAlpha 的混合下：
        // 总散射光 = (散射光 * Alpha) + (背景中已有的散射光 * (1 - Alpha)) = 散射光 (保持一致)
        final_rgb = (result * volumetric_data.a + volumetric_data.rgb) * object_color.a;
    }

    return vec4<f32>(final_rgb, object_color.a);
}
