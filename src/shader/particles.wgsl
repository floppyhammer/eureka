struct ParticleData {
	transform: mat4x4<f32>,
	velocity: vec3<f32>,
	color: vec4<f32>,
	custom: vec4<f32>,
}

struct FrameParams {
	emitting: bool,
	system_phase: f32,
	prev_system_phase: f32,
	cycle: u32,

    explosiveness: f32,
    randomness: f32,
    time: f32,
    delta: f32,

	frame: u32,
	random_seed: u32,
	particle_size: f32,
	pad0: u32,

	emission_transform: mat4x4<f32>, // The transform of the emitter node.
}

@group(0)
@binding(0)
var<storage, read_write> particles: array<ParticleData>;

@group(0)
@binding(1)
var<storage, read_write> frame_params: FrameParams;

@compute
@workgroup_size(1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let particle_id = global_id.x;

    particles[global_id.x] = ;
}
