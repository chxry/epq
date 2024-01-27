#![no_std]
pub mod rt;

use spirv_std::spirv;
use spirv_std::image::Image;
use spirv_std::glam::{Vec4, Vec2};
use shared::Consts;

pub type StorageImage = Image!(2D, format = rgba32f, sampled = false);

// replace
#[spirv(vertex)]
pub fn quad_v(#[spirv(vertex_index)] idx: u32, #[spirv(position)] out_pos: &mut Vec4) {
  let uv = Vec2::new(((idx << 1) & 2) as f32, (idx & 2) as f32);
  *out_pos = (2.0 * uv - Vec2::ONE).extend(0.0).extend(1.0);
}

#[spirv(fragment)]
pub fn test_f(
  #[spirv(frag_coord)] pos: Vec4,
  #[spirv(descriptor_set = 0, binding = 0)] tex: &StorageImage,
  #[spirv(uniform, descriptor_set = 1, binding = 0)] consts: &Consts,
  out_color: &mut Vec4,
) {
  *out_color = tex.read(pos.truncate().truncate().as_uvec2()) / consts.samples as f32;
}
