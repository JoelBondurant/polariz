struct VertexInput {
	@location(0) position: vec2<f32>,
	@location(1) color: vec3<f32>,
};

struct VertexOutput {
	@builtin(position) clip_position: vec4<f32>,
	@location(0) color: vec3<f32>,
};

struct Uniforms {
	y_min: f32,
	y_max: f32,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
	var out: VertexOutput;
	let x_clip = input.position.x;
	let y_norm = (input.position.y - uniforms.y_min) / (uniforms.y_max - uniforms.y_min);
	let y_clip = y_norm * 2.0 - 1.0;
	out.clip_position = vec4<f32>(x_clip, y_clip, 0.0, 1.0);
	out.color = input.color;
	return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
	return vec4<f32>(in.color, 1.0);
}
