use spirv_std::spirv;
use spirv_std::num_traits::Float;
use spirv_std::glam::{UVec3, Vec3, Vec2};
use shared::Consts;
use crate::StorageImage;

#[spirv(compute(threads(1)))]
pub fn main(
  #[spirv(global_invocation_id)] id: UVec3,
  #[spirv(descriptor_set = 0, binding = 0)] tex: &StorageImage,
  #[spirv(uniform, descriptor_set = 1, binding = 0)] consts: &Consts,
) {
  let mut rng = Rng(id.x * consts.samples + id.y * consts.size.x);
  let uv = ((id.as_vec3().truncate() + rng.next_vec2()) / consts.size.as_vec2()) * 2.0 - Vec2::ONE;
  let ray = Ray {
    origin: Vec3::ZERO,
    dir: -(uv * Vec2::new(consts.size.x as f32 / consts.size.y as f32, 1.0) * 1.25.tan())
      .extend(1.0),
  };

  let sphere = Sphere {
    pos: Vec3::new(0.0, 0.0, -2.0),
    radius: 1.0,
  };
  let color = if sphere.hit(&ray) {
    Vec3::X
  } else {
    Vec3::ZERO
  };

  // safety: this is our texel
  unsafe { tex.write(id.truncate(), tex.read(id.truncate()) + color.extend(1.0)) };
}

struct Ray {
  origin: Vec3,
  dir: Vec3,
}

struct Sphere {
  pos: Vec3,
  radius: f32,
}

impl Sphere {
  fn hit(&self, ray: &Ray) -> bool {
    let oc = ray.origin - self.pos;
    let a = ray.dir.length_squared();
    let b = oc.dot(ray.dir);
    let c = oc.length_squared() - self.radius * self.radius;
    let disc = b * b - a * c;
    disc >= 0.0
  }
}

/// pcg random number generator
struct Rng(u32);

impl Rng {
  /// [0, u32::MAX]
  fn next(&mut self) -> u32 {
    self.0 = self.0 * 747796405 + 2891336453;
    let w = ((self.0 >> ((self.0 >> 28) + 4)) ^ self.0) * 277803737;
    (w >> 22) ^ w
  }

  /// [-1.0, 1.0]
  fn next_f32(&mut self) -> f32 {
    self.next() as f32 / (u32::MAX as f32 / 2.0) - 1.0
  }

  /// [-1.0, 1.0]
  fn next_vec2(&mut self) -> Vec2 {
    Vec2::new(self.next_f32(), self.next_f32())
  }

  /// [-1.0, 1.0]
  fn next_vec3(&mut self) -> Vec3 {
    Vec3::new(self.next_f32(), self.next_f32(), self.next_f32())
  }
}
