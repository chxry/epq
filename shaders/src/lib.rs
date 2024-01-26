#![no_std]
use spirv_std::spirv;
use spirv_std::image::Image;
use spirv_std::glam::{UVec3, Vec4, Vec2};
use shared::Consts;

#[spirv(compute(threads(1)))]
pub unsafe fn rt_main(
  #[spirv(global_invocation_id)] id: UVec3,
  #[spirv(descriptor_set = 0, binding = 0)] tex: &Image!(2D, format = rgba32f, sampled = false),
  #[spirv(uniform, descriptor_set = 1, binding = 0)] consts: &Consts,
) {
  tex.write(
    id.truncate(),
    (id.as_vec3() / consts.size.extend(1.0)).extend(1.0),
  );
}

// replace
#[spirv(vertex)]
pub fn quad_v(#[spirv(vertex_index)] idx: u32, #[spirv(position)] out_pos: &mut Vec4) {
  let uv = Vec2::new(((idx << 1) & 2) as f32, (idx & 2) as f32);
  *out_pos = (2.0 * uv - Vec2::ONE).extend(0.0).extend(1.0);
}

#[spirv(fragment)]
pub fn test_f(
  #[spirv(frag_coord)] pos: Vec4,
  #[spirv(descriptor_set = 0, binding = 0)] tex: &Image!(2D, format = rgba32f),
  out_color: &mut Vec4,
) {
  *out_color = tex.read(pos.truncate().truncate().as_uvec2())
}
