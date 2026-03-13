struct VertexOutput {
	@builtin(position) position: vec4<f32>,
	@location(0) uv: vec2<f32>,
};

struct ScreenUniform {
	aspect_ratio: f32,
	max_count: f32,
	radius: f32,
	min_q: i32,
	min_r: i32,
};

@group(0) @binding(0) var<uniform> screen: ScreenUniform;
@group(0) @binding(1) var t_density: texture_2d<f32>;

@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> VertexOutput {
	var out: VertexOutput;
	let x = f32((i32(idx) & 1) << 2) - 1.0;
	let y = f32((i32(idx) & 2) << 1) - 1.0;
	out.position = vec4<f32>(x, y, 0.0, 1.0);
	out.uv = vec2<f32>(x * 0.5 + 0.5, 1.0 - (y * 0.5 + 0.5));
	return out;
}

fn hex_round(frac: vec2<f32>) -> vec2<f32> {
	var q = round(frac.x);
	var r = round(frac.y);
	var s = round(-frac.x - frac.y);
	let q_diff = abs(q - frac.x);
	let r_diff = abs(r - frac.y);
	let s_diff = abs(s - (-frac.x - frac.y));
	if q_diff > r_diff && q_diff > s_diff {
		q = -r - s;
	} else if r_diff > s_diff {
		r = -q - s;
	}
	return vec2<f32>(q, r);
}

fn lookup_density(q: i32, r: i32) -> f32 {
	let coords = vec2<i32>(q - screen.min_q, r - screen.min_r);
	let tex_size = textureDimensions(t_density);
	if coords.x < 0 || coords.y < 0 || coords.x >= i32(tex_size.x) || coords.y >= i32(tex_size.y) {
		return 0.0;
	}
	let count = textureLoad(t_density, coords, 0).r;
	return count / screen.max_count;
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
	let radius = screen.radius;
	let p = vec2<f32>(in.uv.x * screen.aspect_ratio, in.uv.y);
	let q_frac = (sqrt(3.0) / 3.0 * p.x - 1.0 / 3.0 * p.y) / radius;
	let r_frac = (2.0 / 3.0 * p.y) / radius;
	let axial  = hex_round(vec2<f32>(q_frac, r_frac));
	let qi = i32(axial.x);
	let ri = i32(axial.y);
	let density = lookup_density(qi, ri);
	let has_data = density >= 0.0;
	let d = max(density, 0.0);
	let color = viridis(d);
	let background_fill = vec3<f32>(0.02, 0.02, 0.05);
	let fill = select(background_fill, color, has_data);
	let center_x = radius * (sqrt(3.0) * axial.x + sqrt(3.0) / 2.0 * axial.y);
	let center_y = radius * (3.0 / 2.0 * axial.y);
	let center_pos = vec2<f32>(center_x, center_y);
	let local_p   = abs(p - center_pos);
	let h_dist	= max(local_p.x, local_p.x * 0.5 + local_p.y * (sqrt(3.0) / 2.0));
	let threshold = radius * (sqrt(3.0) / 2.0);
	let hex_mask  = smoothstep(threshold, threshold * 0.95, h_dist);
	let border_color = vec3<f32>(0.01, 0.01, 0.08);
	let final_rgb = mix(border_color, fill, hex_mask);
	return vec4<f32>(final_rgb, 1.0);
}
