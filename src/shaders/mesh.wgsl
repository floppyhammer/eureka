// Vertex shader //

struct Camera {
    view_pos: vec4<f32>,
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    view_proj: mat4x4<f32>,
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

const MAX_POINT_LIGHTS = 10;

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
var t_shadow: texture_depth_2d;
@group(1) @binding(2)
var s_shadow: sampler_comparison;
@group(1) @binding(3)
var<uniform> light_camera: Camera;

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
#ifdef COLOR_MAP
@group(2) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(2) @binding(1)
var s_diffuse: sampler;
#endif

#ifdef NORMAP_MAP
@group(2) @binding(2)
var t_normal: texture_2d<f32>;
@group(2) @binding(3)
var s_normal: sampler;
#endif
// -------------------------

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Sample diffuse texture.
#ifdef COLOR_MAP
    let object_color: vec4<f32> = textureSample(t_diffuse, s_diffuse, in.tex_coords);
#else
    let object_color: vec4<f32> = vec4<f32>(1.0, 1.0, 1.0, 1.0);
#endif

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

    let ambient_color = lights.ambient_color * lights.ambient_strength;
    let view_dir = normalize(camera.view_pos.xyz - in.world_position.xyz);

    var point_lights_result = vec3<f32>(0.0, 0.0, 0.0);
    for (var i: u32 = 0; i < lights.point_light_count; i++) {
        let light = lights.point_lights[i];
        let light_vec = light.position - in.world_position.xyz;
        let distance = length(light_vec);
        let light_dir = normalize(light_vec);
        let half_dir = normalize(view_dir + light_dir);

        // Diffuse
        let diffuse_strength = max(dot(world_normal, light_dir), 0.0);
        let diffuse_color = light.color * diffuse_strength * light.strength;

        // Specular
        let specular_strength = pow(max(dot(world_normal, half_dir), 0.0), 32.0);
        let specular_color = light.color * specular_strength * light.strength;

        // Attenuation
        let attenuation = 1.0 / (light.constant + light.linear0 * distance + light.quadratic * (distance * distance));

        point_lights_result += (diffuse_color + specular_color) * attenuation;
    }

    var directional_light_result = vec3<f32>(0.0, 0.0, 0.0);
    {
        // 1. Calculate direction vectors
        let light_dir = normalize(-lights.directional_light.direction);
        let half_dir = normalize(view_dir + light_dir);

        // 2. Shadow mapping
        let shadow_coords = light_camera.view_proj * in.world_position;
        let shadow_pos = shadow_coords.xyz / shadow_coords.w;
        let shadow_uv = shadow_pos.xy * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5, 0.5);

        var shadow_factor = 1.0;
        // Geometric back-face check: if the surface faces away from the light, it's in shadow.
        let n_dot_l = dot(world_normal_basis, light_dir);
        if (n_dot_l <= 0.0) {
            shadow_factor = 0.0;
        } else if (shadow_pos.x >= -1.0 && shadow_pos.x <= 1.0 &&
            shadow_pos.y >= -1.0 && shadow_pos.y <= 1.0 &&
            shadow_pos.z >= 0.0 && shadow_pos.z <= 1.0) {

            // Slope-scaled Bias: more bias when the light is at a steep angle.
            // This prevents "Shadow Acne" while minimizing "Peter Panning".
            let bias = max(0.0015 * (1.0 - n_dot_l), 0.0002);
            shadow_factor = textureSampleCompare(t_shadow, s_shadow, shadow_uv, shadow_pos.z - bias);
        }

        // 3. Lighting calculation
        let diffuse_strength = max(dot(world_normal, light_dir), 0.0);
        let diffuse_color = lights.directional_light.color * diffuse_strength;

        let specular_strength = pow(max(dot(world_normal, half_dir), 0.0), 32.0);
        let specular_color = lights.directional_light.color * specular_strength;

        directional_light_result = (diffuse_color + specular_color) * lights.directional_light.strength * shadow_factor;
    }

    let result = (ambient_color + point_lights_result + directional_light_result) * object_color.xyz;
    return vec4<f32>(result, object_color.a);
}
