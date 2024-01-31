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
    distance: f32,
}

struct Lights {
    ambient_color: vec3<f32>,
    ambient_strength: f32,
    point_light: PointLight,
    directional_light: DirectionalLight,
}

@group(1) @binding(0)
var<uniform> lights: Lights;

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
    // Analogous to GLSL's gl_Position.
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    // Positions below are in TBN space.
    @location(1) tbn_position: vec3<f32>,
    @location(2) tbn_light_position: vec3<f32>,
    @location(3) tbn_view_position: vec3<f32>,
}

@vertex
fn vs_main(vertex: VertexInput, instance: InstanceInput) -> VertexOutput {
    let model_matrix = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3);

    // Model matrix for normal.
    let normal_matrix = mat3x3<f32>(
        instance.normal_matrix_0,
        instance.normal_matrix_1,
        instance.normal_matrix_2);

    // Construct the tangent matrix.
    let world_normal = normalize(normal_matrix * vertex.normal);
    let world_tangent = normalize(normal_matrix * vertex.tangent);
    let world_bitangent = normalize(normal_matrix * vertex.bitangent);
    let tbn_matrix = transpose(mat3x3<f32>(
        world_tangent,
        world_bitangent,
        world_normal));

    // Vertex's world position.
    let vertex_world_position = model_matrix * vec4<f32>(vertex.position, 1.0);

    var out: VertexOutput;
    out.clip_position = camera.view_proj * vertex_world_position;
    out.tex_coords = vertex.tex_coords;

    /*
    So instead of sending the inverse of the TBN matrix to the fragment shader,
    we send a tangent-space light position, view position, and vertex position
    to the fragment shader. This saves us from having to do matrix
    multiplications in the fragment shader.
    */

    // Convert world positions to TBN space.
    out.tbn_position = tbn_matrix * vertex_world_position.xyz;
    out.tbn_view_position = tbn_matrix * camera.view_pos.xyz;
    out.tbn_light_position = tbn_matrix * lights.point_light.position;

    return out;
}

// Fragment shader //

// Texture bind group.
// -------------------------
#ifdef COLOR_MAP
// Diffuse.
@group(2) @binding(0)
var t_diffuse: texture_2d<f32>;

@group(2) @binding(1)
var s_diffuse: sampler;
#endif

#ifdef NORMAP_MAP
// Normal map.
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

#ifdef NORMAP_MAP
    // The normal map is defined in TBN space.
    let tbn_normal = textureSample(t_normal, s_normal, in.tex_coords).xyz * 2.0 - 1.0;
#else
    // Use the unit normal in TBN space.
    let tbn_normal = vec3<f32>(0.0, 0.0, 1.0);
#endif

    let ambient_color = lights.ambient_color * lights.ambient_strength;

    // Create the lighting vectors. (Do calculations in TBN space.)
    let light_dir = normalize(in.tbn_light_position - in.tbn_position);
    let view_dir = normalize(in.tbn_view_position - in.tbn_position);
    // The Blinn part of Blinn-Phong.
    let half_dir = normalize(view_dir + light_dir);

    // Calculate diffuse lighting.
    let diffuse_strength = max(dot(tbn_normal, light_dir), 0.0);
    let diffuse_color = lights.point_light.color * diffuse_strength;

    // Calculate specular lighting.
    let specular_strength = pow(max(dot(tbn_normal, half_dir), 0.0), 4.0);
    let specular_color = specular_strength * lights.point_light.color;

    // Compute attenuation.
    let distance = length(in.tbn_light_position - in.tbn_position);
    let attenuation = 1.0 / (lights.point_light.constant + lights.point_light.linear0 * distance +
    		    lights.point_light.quadratic * (distance * distance));

    let result = (ambient_color + diffuse_color + specular_color) * attenuation * object_color.xyz;

    return vec4<f32>(result, object_color.a);
}
