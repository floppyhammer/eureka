#import eureka::camera::Camera

@group(0) @binding(0) var<uniform> camera: Camera;

@group(0) @binding(1) var t_color: texture_2d<f32>;
@group(0) @binding(2) var t_normal_roughness: texture_2d<f32>;
@group(0) @binding(3) var t_depth: texture_depth_2d;
@group(0) @binding(4) var s_nearest: sampler;
@group(0) @binding(5) var s_linear: sampler;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    let x = f32(i32(in_vertex_index & 1u) * 4 - 1);
    let y = f32(i32(in_vertex_index & 2u) * 2 - 1);
    out.position = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = vec2<f32>((x + 1.0) * 0.5, (1.0 - y) * 0.5);
    return out;
}

// 将 NDC 深度转换为观察空间 Z (负值)
fn ndc_depth_to_view_z(depth: f32) -> f32 {
    let view_pos = camera.inv_proj * vec4<f32>(0.0, 0.0, depth, 1.0);
    return view_pos.z / view_pos.w;
}

// 从 UV 和深度重建观察空间位置
fn reconstruct_view_pos(uv: vec2<f32>, depth: f32) -> vec3<f32> {
    let ndc = vec3<f32>(uv.x * 2.0 - 1.0, (1.0 - uv.y) * 2.0 - 1.0, depth);
    let view_pos_h = camera.inv_proj * vec4<f32>(ndc, 1.0);
    return view_pos_h.xyz / view_pos_h.w;
}

// Fresnel-Schlick 近似
fn fresnel_schlick(cos_theta: f32, F0: f32) -> f32 {
    return F0 + (1.0 - F0) * pow(clamp(1.0 - cos_theta, 0.0, 1.0), 5.0);
}

// 边缘衰减函数
fn edge_fade(uv: vec2<f32>) -> f32 {
    let fade_x = smoothstep(0.0, 0.1, uv.x) * smoothstep(1.0, 0.9, uv.x);
    let fade_y = smoothstep(0.0, 0.1, uv.y) * smoothstep(1.0, 0.9, uv.y);
    return fade_x * fade_y;
}

fn interleaved_gradient_noise(uv: vec2<f32>, frame: u32) -> f32 {
    let magic = vec3<f32>(0.06711056, 0.00583715, 52.9829189);
    let frame_offset = fract(f32(frame % 16u) * 0.61803398875);
    return fract(magic.z * fract(dot(uv, magic.xy) + frame_offset));
}

fn ray_march(
    origin: vec3<f32>,
    direction: vec3<f32>,
    max_steps: u32,
    step_size: f32,
    thickness: f32,
    jitter: f32,
) -> vec2<f32> {
    // 使用抖动来打破采样步长的规律性，减少摩尔纹和切片感
    var current_pos = origin + direction * step_size * (jitter + 0.1);
    var prev_pos = origin;

    for (var i = 0u; i < max_steps; i++) {
        prev_pos = current_pos;
        current_pos += direction * step_size;
        
        // 检查是否在相机前方 (RH: Z < 0)
        if (current_pos.z >= -0.01) {
            return vec2<f32>(-1.0, 0.0);
        }
        
        // 投影到屏幕空间
        let clip_pos = camera.proj * vec4<f32>(current_pos, 1.0);
        if (clip_pos.w < 0.01) {
            return vec2<f32>(-1.0, 0.0);
        }
        
        let screen_pos = clip_pos.xyz / clip_pos.w;
        let uv = vec2<f32>(screen_pos.x * 0.5 + 0.5, 0.5 - screen_pos.y * 0.5);
        
        // 检查是否超出屏幕边界
        if (uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0) {
            return vec2<f32>(-1.0, 0.0);
        }
        
        // 采样场景深度
        let scene_depth = textureSampleLevel(t_depth, s_nearest, uv, 0);
        if (scene_depth >= 0.9999) {
            continue;
        }

        let scene_view_z = ndc_depth_to_view_z(scene_depth);
        
        // 检查是否相交
        let ray_z = current_pos.z;
        let delta = scene_view_z - ray_z;
        
        // delta > 0 表示射线在场景深度之后
        if (delta > 0.0 && delta < thickness) {
            // 二分搜索 refinement
            var refine_origin = prev_pos;
            var refine_dir = direction * step_size;
            var refine_uv = uv;

            for (var j = 0u; j < 8u; j++) {
                refine_dir *= 0.5;
                let mid_pos = refine_origin + refine_dir;

                let mid_clip = camera.proj * vec4<f32>(mid_pos, 1.0);
                if (mid_clip.w < 0.01) {
                    break;
                }

                let mid_screen = mid_clip.xyz / mid_clip.w;
                refine_uv = vec2<f32>(mid_screen.x * 0.5 + 0.5, 0.5 - mid_screen.y * 0.5);

                if (refine_uv.x < 0.0 || refine_uv.x > 1.0 || refine_uv.y < 0.0 || refine_uv.y > 1.0) {
                    break;
                }

                let mid_depth = textureSampleLevel(t_depth, s_nearest, refine_uv, 0);
                let mid_scene_z = ndc_depth_to_view_z(mid_depth);

                if (mid_scene_z - mid_pos.z < 0.0) {
                    refine_origin = mid_pos;
                }
            }

            return refine_uv;
        }
    }

    return vec2<f32>(-1.0, 0.0);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    if (camera.ssr_enabled == 0u) {
        return vec4<f32>(0.0);
    }

    let tex_size = vec2<f32>(textureDimensions(t_color));
    
    let nr_sample = textureSampleLevel(t_normal_roughness, s_nearest, in.uv, 0);
    let view_normal = normalize(nr_sample.xyz * 2.0 - 1.0);
    let roughness = nr_sample.a;

    let depth = textureSampleLevel(t_depth, s_nearest, in.uv, 0);

    if (depth >= 1.0 || depth <= 0.0) {
        return vec4<f32>(0.0);
    }
    
    if (length(view_normal) < 0.01) {
        return vec4<f32>(0.0);
    }
    
    if (roughness > 0.8) {
        return vec4<f32>(0.0);
    }
    
    let view_pos = reconstruct_view_pos(in.uv, depth);
    let view_dir = normalize(-view_pos);

    let reflect_dir = reflect(-view_dir, view_normal);
    
    // Ray Marching 参数
    let view_distance = length(view_pos);
    let max_steps = 80u;

    // 基础步长限制，防止在近处太碎，在远处太粗
    let step_size = max(0.04, view_distance * 0.01 + roughness * 0.05);

    // 厚度随距离增加，但在掠射角处削减以防止背面穿透
    let n_dot_r = max(dot(view_normal, reflect_dir), 0.001);
    let thickness = max(0.1, view_distance * 0.03) * n_dot_r;
    
    // 抖动处理
    let jitter = interleaved_gradient_noise(in.position.xy, camera.frame_count);

    let hit_uv = ray_march(view_pos, reflect_dir, max_steps, step_size, thickness, jitter);

    if (hit_uv.x < 0.0) {
        return vec4<f32>(0.0);
    }
    
    let fade = edge_fade(hit_uv);
    if (fade < 0.01) {
        return vec4<f32>(0.0);
    }

    var reflection_color = vec3<f32>(0.0);
    if (roughness < 0.1) {
        reflection_color = textureSampleLevel(t_color, s_linear, hit_uv, 0.0).rgb;
    } else {
        let blur_radius = roughness * 4.0;
        let inv_tex_size = 1.0 / tex_size;
        var sample_count = 0.0;
        
        for (var y = -1; y <= 1; y++) {
            for (var x = -1; x <= 1; x++) {
                let offset = vec2<f32>(f32(x), f32(y)) * blur_radius * inv_tex_size;
                let sample_uv = hit_uv + offset;
                if (sample_uv.x >= 0.0 && sample_uv.x <= 1.0 && sample_uv.y >= 0.0 && sample_uv.y <= 1.0) {
                    let c = textureSampleLevel(t_color, s_linear, sample_uv, 0.0).rgb;
                    // --- Firefly clamping ---
                    // 限制单个采样的亮度，防止 HDR 亮块在粗糙表面产生剧烈闪烁
                    let luminance = dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
                    let max_lum = 2.0;
                    let weight = 1.0 / (1.0 + luminance / max_lum);

                    reflection_color += c * weight;
                    sample_count += weight;
                }
            }
        }
        
        if (sample_count > 0.0) {
            reflection_color /= sample_count;
        }
    }
    
    let n_dot_v = max(dot(-view_dir, view_normal), 0.0);

    // 绝缘体 F0 = 0.04
    let F0 = 0.04;
    let fresnel = fresnel_schlick(n_dot_v, F0);
    
    // 增强粗糙度衰减：织物等粗糙表面反射应该极弱
    let roughness_fade = pow(saturate(1.0 - roughness), 4.0);

    let intensity = roughness_fade * fresnel * fade;

    // 最后的安全限制：如果反射颜色过亮，也进行一定的抑制
    reflection_color = min(reflection_color, vec3<f32>(1.5));

    return vec4<f32>(reflection_color, intensity);
}
