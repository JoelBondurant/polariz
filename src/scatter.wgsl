struct VertexInput {
	@location(0) position: vec2<f32>,
	@location(1) color: vec3<f32>,
	@builtin(vertex_index) vertex_idx: u32,
};

struct VertexOutput {
	@builtin(position) clip_position: vec4<f32>,
	@location(0) color: vec3<f32>,
	@location(1) uv: vec2<f32>,
};

struct Uniforms {
	x_min: f32,
	x_max: f32,
	y_min: f32,
	y_max: f32,
	point_size: f32,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

@vertex
fn vs_main(
	@location(0) pos: vec2<f32>,
	@location(1) color: vec3<f32>,
	@builtin(vertex_index) vertex_idx: u32,
) -> VertexOutput {
	var out: VertexOutput;
	let quad_idx = vertex_idx % 6;
	var uv = vec2<f32>(0.0, 0.0);
	if (quad_idx == 0u) { uv = vec2<f32>(-1.0, -1.0); }
	else if (quad_idx == 1u) { uv = vec2<f32>(1.0, -1.0); }
	else if (quad_idx == 2u) { uv = vec2<f32>(1.0, 1.0); }
	else if (quad_idx == 3u) { uv = vec2<f32>(-1.0, -1.0); }
	else if (quad_idx == 4u) { uv = vec2<f32>(1.0, 1.0); }
	else if (quad_idx == 5u) { uv = vec2<f32>(-1.0, 1.0); }
	let x_norm = (pos.x - uniforms.x_min) / (uniforms.x_max - uniforms.x_min);
	let y_norm = (pos.y - uniforms.y_min) / (uniforms.y_max - uniforms.y_min);
	let base_clip = vec2<f32>(x_norm * 2.0 - 1.0, y_norm * 2.0 - 1.0);
	let offset = uv * uniforms.point_size;
	out.clip_position = vec4<f32>(base_clip + offset, 0.0, 1.0);
	out.color = color;
	out.uv = uv;
	return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
	let dist = length(in.uv);
	if (dist > 1.0) {
		discard;
	}
	let alpha = smoothstep(1.0, 0.8, dist);
	return vec4<f32>(in.color, alpha);
}
