struct VertexOutput {
	@location(0) color_index: u32,
	@builtin(position) position: vec4<f32>,
};

@group(0)
@binding(0)
var<uniform> transform: mat4x4<f32>;

@vertex
fn vs_main(
	@location(0) position: vec4<f32>,
	@location(1) color_index: u32,
) -> VertexOutput {
	var result: VertexOutput;
	result.color_index = color_index;
	result.position = transform * position;
	return result;
}

@group(0)
@binding(1)
var r_color: texture_1d<f32>;

@fragment
fn fs_main(vertex: VertexOutput) -> @location(0) vec4<f32> {
	return textureLoad(r_color, vertex.color_index, 0);
}
