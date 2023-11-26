#import bevy_render::view::View

@group(0) @binding(0) var<uniform> view: View;
@group(0) @binding(1) var output_texture: texture_storage_2d<rgba16float, read_write>;

@compute @workgroup_size(8, 8, 1)
fn path_trace(@builtin(global_invocation_id) id: vec3<u32>) {
    let pixel_uv = (vec2<f32>(id.xy) + 0.5) / view.viewport.zw;
    // Clip position goes from -1 to 1.
    let pixel_clip_pos = (pixel_uv * 2.0) - 1.0;
    let ray_target = view.inverse_view_proj * vec4<f32>(pixel_clip_pos, 1.0, 1.0);
    let ray_origin = view.world_position;

    var c: vec4<f32>;
    if ray_sphere_intersect(ray_origin, ray_target.xyz - ray_origin, vec3<f32>(0.0, 0.0, -3.0), 1.0) != -1.0 {
        c = vec4<f32>(3.0, 4.0, 4.0, 1.0);
    }
    else {
        c = vec4<f32>(0.0, 1.0, 1.0, 1.0);
    }

    // var c: vec4<f32>;
    // if length(ray_target.xy) < 0.3 {
    
    textureStore(output_texture, id.xy, c);
}

// fn raySphereIntersect(vec3 r0, vec3 rd, vec3 s0, float sr) {
//     // - r0: ray origin
//     // - rd: normalized ray direction
//     // - s0: sphere center
//     // - sr: sphere radius
//     // - Returns distance from r0 to first intersecion with sphere,
//     //   or -1.0 if no intersection.
//     float a = dot(rd, rd);
//     vec3 s0_r0 = r0 - s0;
//     float b = 2.0 * dot(rd, s0_r0);
//     float c = dot(s0_r0, s0_r0) - (sr * sr);
//     if (b*b - 4.0*a*c < 0.0) {
//         return -1.0;
//     }
//     return (-b - sqrt((b*b) - 4.0*a*c))/(2.0*a);
// }

fn ray_sphere_intersect(r0: vec3<f32>, rd: vec3<f32>, s0: vec3<f32>, sr: f32) -> f32 {
    // - r0: ray origin
    // - rd: normalized ray direction
    // - s0: sphere center
    // - sr: sphere radius
    // - Returns distance from r0 to first intersecion with sphere,
    //   or -1.0 if no intersection.
    let a = dot(rd, rd);
    let s0_r0 = r0 - s0;
    let b = 2.0 * dot(rd, s0_r0);
    let c = dot(s0_r0, s0_r0) - (sr * sr);
    if (b*b - 4.0*a*c < 0.0) {
        return -1.0;
    }
    return (-b - sqrt((b*b) - 4.0*a*c))/(2.0*a);
    
}
