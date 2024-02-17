struct VertexOutput {
	@location(0) tex_coord: vec2<f32>,
	@builtin(position) position: vec4<f32>,
};

@group(0)
@binding(0)
var<uniform> transform: mat4x4<f32>;

@vertex
fn vs_main(
	@location(0) position: vec4<f32>,
	@location(1) tex_coord: vec2<f32>,
) -> VertexOutput {
	var result: VertexOutput;
	result.tex_coord = tex_coord;
	result.position = transform * position;
	return result;
}

@group(0)
@binding(1)
var r_color: texture_2d<f32>;

@fragment
fn fs_main(vertex: VertexOutput) -> @location(0) vec4<f32> {
	var color = textureLoad(r_color, vec2<i32>(vertex.tex_coord), 0);
	if color.a < 0.5 {
		discard;
	}
	return color;
}
