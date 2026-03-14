struct VertexOutput {
	@builtin(position) clip_position: vec4<f32>,
	@location(0) color: vec4<f32>,
};

struct Uniforms {
	x_min: f32,
	x_max: f32,
	y_min: f32,
	y_max: f32,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

@vertex
fn vs_main(
	@location(0) pos: vec4<f32>,
	@location(1) color: vec4<f32>,
) -> VertexOutput {
	var out: VertexOutput;
	let x_range = max(uniforms.x_max - uniforms.x_min, 0.0001);
	let y_range = max(uniforms.y_max - uniforms.y_min, 0.0001);
	let x_norm = (pos.x - uniforms.x_min) / x_range;
	let y_norm = (pos.y - uniforms.y_min) / y_range;
	out.clip_position = vec4<f32>(x_norm * 2.0 - 1.0, y_norm * 2.0 - 1.0, 0.0, 1.0);
	out.color = color;
	return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
	return in.color;
}
