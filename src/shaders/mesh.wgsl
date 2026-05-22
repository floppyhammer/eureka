// Vertex shader //

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
    _pad0: f32,
    _pad1: f32,
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
    @location(9) normal_matrix_0: vec3<f32>,
    @location(10) normal_matrix_1: vec3<f32>,
    @location(11) normal_matrix_2: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) world_position: vec4<f32>,
    @location(2) world_tangent: vec3<f32>,
    @location(3) world_bitangent: vec3<f32>,
    @location(4) world_normal: vec3<f32>,
}

@vertex
fn vs_main(vertex: VertexInput, instance: InstanceInput) -> VertexOutput {
    let model_matrix = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3);

    let normal_matrix = mat3x3<f32>(
        instance.normal_matrix_0,
        instance.normal_matrix_1,
        instance.normal_matrix_2);

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

    return out;
}

// Fragment shader //

// Texture bind group.
// -------------------------
struct Material {
    base_color: vec4<f32>,
    metallic: f32,
    roughness: f32,
    _pad0: f32,
    _pad1: f32,
}

@group(2) @binding(0)
var<uniform> material: Material;

#ifdef COLOR_MAP
@group(2) @binding(1)
var t_diffuse: texture_2d<f32>;
@group(2) @binding(2)
var s_diffuse: sampler;
#endif

#ifdef NORMAP_MAP
@group(2) @binding(3)
var t_normal: texture_2d<f32>;
@group(2) @binding(4)
var s_normal: sampler;
#endif
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

    return num / max(denom, 0.000001);
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

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Sample diffuse texture.
#ifdef COLOR_MAP
    let sampled_color: vec4<f32> = textureSample(t_diffuse, s_diffuse, in.tex_coords);
#else
    let sampled_color: vec4<f32> = vec4<f32>(1.0, 1.0, 1.0, 1.0);
#endif
    let object_color = sampled_color * material.base_color;

    // Reconstruct TBN matrix (Tangent to World)
    let world_normal_basis = normalize(in.world_normal);
    let world_tangent = normalize(in.world_tangent);
    let world_bitangent = normalize(in.world_bitangent);
    let tbn_to_world = mat3x3<f32>(
        world_tangent,
        world_bitangent,
        world_normal_basis
    );

#ifdef NORMAP_MAP
    let normal_map = textureSample(t_normal, s_normal, in.tex_coords).xyz * 2.0 - 1.0;
    let world_normal = normalize(tbn_to_world * normal_map);
#else
    let world_normal = world_normal_basis;
#endif

    // PBR Parameters (From Material Uniforms)
    let metallic: f32 = material.metallic;
    let roughness: f32 = material.roughness;

    var ambient_ao = 1.0;
    if (camera.ssao_enabled == 1u) {
        ambient_ao = textureLoad(t_ssao, vec2<i32>(in.clip_position.xy), 0).r;
    }
    let view_dir = normalize(camera.view_pos.xyz - in.world_position.xyz);

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

        // Point Shadow
        let frag_to_light = light.position - in.world_position.xyz;
        let dist_vec = abs(frag_to_light);
        let dist_along_axis = max(dist_vec.x, max(dist_vec.y, dist_vec.z));
        let near = 0.1;
        let far = 100.0;
        let shadow_z = (far / (far - near)) - ((far * near) / (far - near)) / dist_along_axis;
        let final_shadow_z = clamp(shadow_z, 0.0, 1.0);
        let light_to_frag = in.world_position.xyz - light.position;
        let shadow_factor = textureSampleCompare(t_point_shadow, s_shadow, light_to_frag, i32(i), final_shadow_z - 0.002);

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

        let n_dot_l = max(dot(world_normal, light_dir), 0.0);
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
        // Geometric back-face check: if the surface faces away from the light, it's in shadow.
        let n_dot_l_geo = dot(world_normal_basis, light_dir);
        if (n_dot_l_geo <= 0.0) {
            shadow_factor = 0.0;
        } else if (shadow_pos.x >= -1.0 && shadow_pos.x <= 1.0 && shadow_pos.y >= -1.0 && shadow_pos.y <= 1.0 && shadow_pos.z >= 0.0 && shadow_pos.z <= 1.0) {
            // Slope-scaled Bias
            let bias = max(0.0005 * (1.0 - n_dot_l_geo), 0.00005);
            // 3x3 PCF (Percentage Closer Filtering) for array textures
            var shadow_sum = 0.0;
            let texel_size = 1.0 / vec2<f32>(textureDimensions(t_shadow).xy);
            for (var y: f32 = -1.0; y <= 1.0; y += 1.0) {
                for (var x: f32 = -1.0; x <= 1.0; x += 1.0) {
                    let offset = vec2<f32>(x, y) * texel_size;
                    shadow_sum += textureSampleCompare(t_shadow, s_shadow, shadow_uv + offset, i32(cascade_index), shadow_pos.z - bias);
                }
            }
            shadow_factor = shadow_sum / 9.0;
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

    let ambient_color = lights.ambient_color * lights.ambient_strength * object_color.xyz * ambient_ao;
    let result = ambient_color + point_lights_result + directional_light_result;

    // Reinhard Tone Mapping
    let mapped = result / (result + vec3<f32>(1.0));

    return vec4<f32>(mapped, object_color.a);
}
