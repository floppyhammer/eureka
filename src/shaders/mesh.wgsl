
struct Camera {
    view_pos: vec4<f32>,
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    view_proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    ssao_enabled: u32,
}

@group(0) @binding(0)
var<uniform> camera: Camera;

struct PointLight {
    position: vec3<f32>,
    strength: f32,
    color: vec3<f32>,
    constant: f32,
    linear0: f32,
    quadratic: f32,
    shadow_near: f32,
    shadow_far: f32,
}

struct DirectionalLight {
    direction: vec3<f32>,
    strength: f32,
    color: vec3<f32>,
    _pad: f32,
}

const MAX_POINT_LIGHTS = 4;

struct Lights {
    ambient_color: vec3<f32>,
    ambient_strength: f32,
    directional_light: DirectionalLight,
    point_lights: array<PointLight, MAX_POINT_LIGHTS>,
    point_light_count: u32,
}

@group(1) @binding(0)
var<uniform> lights: Lights;

@group(1) @binding(1)
var t_shadow: texture_depth_2d_array;
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
    metallic: f32,
    roughness: f32,
    alpha_cutoff: f32,
    color_texture_idx: i32,
    normal_texture_idx: i32,
    metallic_roughness_texture_idx: i32,
    alpha_mode: u32,
    _pad0: u32,
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
        metallic *= mr_sample.b;
        roughness *= mr_sample.g;
    }

    // Clamp roughness to a safe minimum to prevent specular highlight disappearing
    // and avoid division by zero in BRDF equations.
    roughness = max(roughness, 0.045);

    var ambient_ao = 1.0;
    if (camera.ssao_enabled == 1u) {
        // Ensure we stay within bounds and handle potential fractional pixel offsets
        let ssao_coords = vec2<i32>(in.clip_position.xy);
        ambient_ao = textureLoad(t_ssao, ssao_coords, 0).r;
    }

    let view_to_frag = camera.view_pos.xyz - in.world_position.xyz;
    let view_dir = normalize(view_to_frag + vec3<f32>(0.00001)); // Add epsilon to prevent NaN

    // Ensure normal is always facing towards the camera (fixes back-facing artifacts on spheres)
    if (dot(world_normal, view_dir) < 0.0) {
        world_normal = world_normal - 2.0 * dot(world_normal, view_dir) * view_dir;
        world_normal = normalize(world_normal);
    }

    // F0: Surface reflection at zero incidence
    // For non-metals, we use 0.04. For metals, we use the object color.
    var F0 = vec3<f32>(0.04);
    F0 = mix(F0, object_color.xyz, metallic);

    var point_lights_result = vec3<f32>(0.0, 0.0, 0.0);
    for (var i: u32 = 0; i < lights.point_light_count; i++) {
        let light = lights.point_lights[i];
        let light_vec = light.position - in.world_position.xyz;
        let distance = length(light_vec);
        let light_dir = normalize(light_vec);
        let half_dir = normalize(view_dir + light_dir);
        let n_dot_l = max(dot(world_normal, light_dir), 0.0);

        // Adaptive Bias: Using dot product to increase bias at grazing angles
        let bias = max(0.005 * (1.0 - n_dot_l), 0.0005);

        let dist_vec = abs(light_vec);
        let dist_along_axis = max(dist_vec.x, max(dist_vec.y, dist_vec.z));
        let near = light.shadow_near;
        let far = light.shadow_far;
        let shadow_z = (far / (far - near)) - ((far * near) / (far - near)) / dist_along_axis;
        let final_shadow_z = clamp(shadow_z, 0.0, 1.0);
        let light_to_frag = in.world_position.xyz - light.position;
        let shadow_factor = textureSampleCompare(t_point_shadow, s_shadow, light_to_frag, i32(i), final_shadow_z - bias);

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

        let attenuation = 1.0 / (light.constant + light.linear0 * distance + light.quadratic * (distance * distance));
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
            let texel_size = 1.0 / vec2<f32>(textureDimensions(t_shadow).xy);
            for (var y: f32 = -1.0; y <= 1.0; y += 1.0) {
                for (var x: f32 = -1.0; x <= 1.0; x += 1.0) {
                    let offset = vec2<f32>(x, y) * texel_size;
                    shadow_sum += textureSampleCompare(t_shadow, s_shadow, shadow_uv + offset, i32(cascade_index), shadow_pos.z - bias);
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

    // --- Simple IBL (Environmental Reflection) ---
    let reflect_dir = reflect(-view_dir, world_normal);

    // Sample from the skybox.
    // We use roughness * 9.0 to simulate blurring (requires mipmaps)
    let env_reflection = textureSampleLevel(t_skybox, s_skybox, reflect_dir, roughness * 9.0).rgb;

    // Use the roughness-aware Fresnel for indirect specular
    let F_env = fresnel_schlick_roughness(max(dot(world_normal, view_dir), 0.0), F0, roughness);

    // Specular part of indirect lighting
    // Faking the "blurring" effect by reducing intensity if mips are not available
    let env_intensity = 1.0 - (roughness * 0.7);
    let indirect_specular = env_reflection * F_env * ambient_ao * env_intensity;

    // Diffuse part of indirect lighting (Ambient)
    let kS_env = F_env;
    var kD_env = vec3<f32>(1.0) - kS_env;
    kD_env *= 1.0 - metallic;

    let indirect_diffuse = kD_env * lights.ambient_color * lights.ambient_strength * object_color.xyz * ambient_ao;
    // --- End Simple IBL ---

    let result = indirect_diffuse + indirect_specular + point_lights_result + directional_light_result;

    // Reinhard Tone Mapping
    let mapped = result / (result + vec3<f32>(1.0));

    return vec4<f32>(mapped, object_color.a);
}
