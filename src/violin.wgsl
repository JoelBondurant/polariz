struct VertexOutput {
	@builtin(position) position: vec4<f32>,
	@location(0) uv: vec2<f32>,
};

struct ScreenUniform {
	num_groups: u32,
	y_min: f32,
	y_max: f32,
	width_scale: f32,
};

@group(0) @binding(0) var<uniform> screen: ScreenUniform;
@group(0) @binding(1) var t_kde: texture_2d<f32>;
@group(0) @binding(2) var<storage, read> medians: array<f32>;

@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> VertexOutput {
	var out: VertexOutput;
	let x = f32((i32(idx) & 1) << 2) - 1.0;
	let y = f32((i32(idx) & 2) << 1) - 1.0;
	out.position = vec4<f32>(x, y, 0.0, 1.0);
	out.uv = vec2<f32>(x * 0.5 + 0.5, 1.0 - (y * 0.5 + 0.5));
	return out;
}

fn viridis(t: f32) -> vec3<f32> {
	return vec3<f32>(
		0.184455 + t*(0.107708 + t*(-0.327241 + t*(-4.599932 + t*(6.203736 + t*(4.751787 + t*-5.432077))))),
		0.005768 + t*(1.396470 + t*(0.214814 + t*(-5.758238 + t*(14.153965 + t*(-13.749439 + t*4.641571))))),
		0.267511 + t*(0.073383 + t*(15.657724 + t*(-90.257825 + t*(202.560788 + t*(-202.603108 + t*74.394908))))),
	);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
	let num_groups = f32(screen.num_groups);
	let x_norm = in.uv.x * num_groups;
	let group_idx = i32(floor(x_norm));
	let local_x = fract(x_norm) - 0.5;
	let tex_size = textureDimensions(t_kde);
	let bin_idx = i32((1.0 - in.uv.y) * f32(tex_size.y));
	let density = textureLoad(t_kde, vec2<i32>(group_idx, bin_idx), 0).r;
	let t = f32(group_idx) / max(num_groups - 1.0, 1.0);
	let violin_color = viridis(t);
	let border_color = viridis(1.0 - t);
	let half_width = density * screen.width_scale;
	let dist = abs(local_x);
	let trunc_limit = 0.0001;
	let is_visible = step(trunc_limit, half_width);
	let inside = (1.0 - smoothstep(half_width - 0.002, half_width, dist)) * is_visible;
	let border_thickness = 0.025;
	let outer_edge = half_width + border_thickness;
	let border_mask = (1.0 - smoothstep(outer_edge - 0.002, outer_edge, dist)) * (1.0 - inside) * is_visible;
	let med_val = medians[group_idx];
	let med_y_uv = 1.0 - (med_val - screen.y_min) / (screen.y_max - screen.y_min);
	let line_mask = (1.0 - smoothstep(0.001, 0.003, abs(in.uv.y - med_y_uv))) * inside;
	let background = vec3<f32>(0.01, 0.01, 0.02);
	var rgb = background;
	rgb = mix(rgb, border_color, border_mask);
	rgb = mix(rgb, violin_color, inside);
	rgb = mix(rgb, vec3<f32>(1.0), line_mask);
	return vec4<f32>(rgb, 1.0);
}
