struct Size {
    width: u32,
    height: u32,
}

@group(0) @binding(0) var<uniform> target_size: Size;
@group(0) @binding(1) var source_texture: texture_2d<f32>;
@group(0) @binding(2) var source_texture_sampler: sampler;

struct UpscalingVertexOutput {
    @builtin(position)
    position: vec4<f32>,
    @location(0)
    uv: vec2<f32>,
    // @location(1)
    // texture_coordinates: vec2<f32>,
};


// Taken from https://github.com/bevyengine/bevy/blob/main/crates/bevy_core_pipeline/src/fullscreen_vertex_shader/fullscreen.wgsl

// This vertex shader produces the following, when drawn using indices 0..3:
//
//  1 |  0-----x.....2
//  0 |  |  s  |  . ´
// -1 |  x_____x´
// -2 |  :  .´
// -3 |  1´
//    +---------------
//      -1  0  1  2  3
//
// The axes are clip-space x and y. The region marked s is the visible region.
// The digits in the corners of the right-angled triangle are the vertex
// indices.
//
// The top-left has UV 0,0, the bottom-left has 0,2, and the top-right has 2,0.
// This means that the UV gets interpolated to 1,1 at the bottom-right corner
// of the clip-space rectangle that is at 1,-1 in clip space.
@vertex
fn pt_upscaling_vertex_shader(@builtin(vertex_index) vertex_index: u32) -> UpscalingVertexOutput {
    var uv = vec2<f32>(f32(vertex_index >> 1u), f32(vertex_index & 1u)) * 2.0;
    var clip_position: vec4<f32> = vec4<f32>(uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0), 0.0, 1.0);

    return UpscalingVertexOutput(clip_position, uv);
}

@fragment
fn pt_upscaling_fragment_shader(vertex_output: UpscalingVertexOutput) -> @location(0) vec4<f32> {
    return textureSample(source_texture, source_texture_sampler, vertex_output.uv);
}
