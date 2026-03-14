struct VertexOutput {
	@builtin(position) clip_position: vec4<f32>,
	@location(0) uv: vec2<f32>,
};

struct ScreenUniform {
	aspect_ratio: f32,
	num_sectors: u32,
};

@group(0) @binding(0) var<uniform> uniforms: ScreenUniform;
@group(0) @binding(1) var<storage, read> sector_angles: array<f32>;

@vertex
fn vs_main(@builtin(vertex_index) vertex_idx: u32) -> VertexOutput {
	var out: VertexOutput;
	let uv = vec2<f32>(f32((vertex_idx << 1u) & 2u), f32(vertex_idx & 2u));
	let pos = uv * 2.0 - 1.0;
	out.clip_position = vec4<f32>(pos, 0.0, 1.0);
	out.uv = pos;
	return out;
}

fn viridis(t: f32) -> vec3<f32> {
	let r = 0.184455 + t*(0.107708 + t*(-0.327241 + t*(-4.599932 + t*(6.203736 + t*(4.751787 + t*-5.432077)))));
	let g = 0.005768 + t*(1.396470 + t*(0.214814 + t*(-5.758238 + t*(14.153965 + t*(-13.749439 + t*4.641571)))));
	let b = 0.267511 + t*(0.073383 + t*(15.657724 + t*(-90.257825 + t*(202.560788 + t*(-202.603108 + t*74.394908)))));
	return vec3<f32>(r, g, b);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
	let uv = vec2<f32>(in.uv.x * uniforms.aspect_ratio, in.uv.y);
	let dist = length(uv);
	if (dist > 0.8 || dist < 0.3) {
		discard;
	}
	let pi = 3.14159265359;
	var angle = atan2(uv.x, uv.y);
	if (angle < 0.0) {
		angle += 2.0 * pi;
	}
	let normalized_angle = angle / (2.0 * pi);
	var sector_idx = 0u;
	for (var i = 0u; i < uniforms.num_sectors; i++) {
		if (normalized_angle < sector_angles[i]) {
			sector_idx = i;
			break;
		}
	}
	let t = f32(sector_idx) / f32(uniforms.num_sectors);
	let color = viridis(t);
	let alpha = smoothstep(0.8, 0.79, dist) * smoothstep(0.3, 0.31, dist);
	return vec4<f32>(color, alpha);
}
