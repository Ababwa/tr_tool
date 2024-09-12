struct VertexOutput {
	@builtin(position) position: vec4<f32>,
	@location(0) tex_coord: vec2<f32>,
};

@group(0)
@binding(0)
var<uniform> perspective: mat4x4<f32>;

@group(0)
@binding(1)
var<uniform> camera: mat4x4<f32>;

@vertex
fn vs_main(
	@location(0) position: vec4<f32>,
	@location(1) tex_coord: vec2<f32>,
	@location(2) offset: vec2<f32>,
) -> VertexOutput {
	var pos = camera * position;
	pos.x += offset.x;
	pos.y += offset.y;
	pos = perspective * pos;
	return VertexOutput(pos, tex_coord);
}

@group(0)
@binding(2)
var r_color: texture_2d<f32>;

@fragment
fn fs_main(vertex: VertexOutput) -> @location(0) vec4<f32> {
	var color = textureLoad(r_color, vec2<i32>(vertex.tex_coord), 0);
	if color.a < 0.5 {
		discard;
	}
	return color;
}
