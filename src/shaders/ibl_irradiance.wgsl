@group(0) @binding(0)
var t_skybox: texture_cube<f32>;
@group(0) @binding(1)
var s_skybox: sampler;
@group(0) @binding(2)
var t_irradiance: texture_storage_2d_array<rgba16float, write>;

const PI: f32 = 3.14159265359;

fn get_sampling_vector(uv: vec2<f32>, face: u32) -> vec3<f32> {
    let st = uv * 2.0 - 1.0;
    var ret: vec3<f32>;
    switch (face) {
        case 0u: { ret = vec3<f32>(1.0, -st.y, -st.x); } // +X
        case 1u: { ret = vec3<f32>(-1.0, -st.y, st.x); } // -X
        case 2u: { ret = vec3<f32>(st.x, 1.0, st.y); }  // +Y
        case 3u: { ret = vec3<f32>(st.x, -1.0, -st.y); } // -Y
        case 4u: { ret = vec3<f32>(st.x, -st.y, 1.0); }  // +Z
        case 5u: { ret = vec3<f32>(-st.x, -st.y, -1.0); } // -Z
        default: { ret = vec3<f32>(0.0); }
    }
    return normalize(ret);
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let output_size = textureDimensions(t_irradiance);
    if (id.x >= output_size.x || id.y >= output_size.y) {
        return;
    }

    let normal = get_sampling_vector((vec2<f32>(id.xy) + 0.5) / vec2<f32>(output_size), id.z);

    var irradiance = vec3<f32>(0.0);

    var up = vec3<f32>(0.0, 1.0, 0.0);
    if (abs(normal.y) > 0.999) {
        up = vec3<f32>(1.0, 0.0, 0.0);
    }
    let right = normalize(cross(up, normal));
    up = cross(normal, right);

    let sample_delta = 0.05;
    var nr_samples = 0.0;

    for (var phi = 0.0; phi < 2.0 * PI; phi += sample_delta) {
        for (var theta = 0.0; theta < 0.5 * PI; theta += sample_delta) {
            // Spherical to cartesian (in tangent space)
            let tangent_sample = vec3<f32>(sin(theta) * cos(phi), sin(theta) * sin(phi), cos(theta));
            // Tangent space to world
            let sample_vec = tangent_sample.x * right + tangent_sample.y * up + tangent_sample.z * normal;

            irradiance += textureSampleLevel(t_skybox, s_skybox, sample_vec, 0.0).rgb * cos(theta) * sin(theta);
            nr_samples += 1.0;
        }
    }

    irradiance = PI * irradiance * (1.0 / nr_samples);

    textureStore(t_irradiance, id.xy, i32(id.z), vec4<f32>(irradiance, 1.0));
}
