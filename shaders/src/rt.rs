use core::ops::Range;
use core::f32::consts::PI;
use spirv_std::spirv;
use spirv_std::num_traits::Float;
use spirv_std::glam::{UVec3, Vec4, Vec3, Vec2};
use shared::Consts;
use crate::StorageImage;

const MAX_BOUNCES: u32 = 16;

#[spirv(compute(threads(1)))]
pub fn main(
  #[spirv(global_invocation_id)] id: UVec3,
  #[spirv(descriptor_set = 0, binding = 0)] tex: &StorageImage,
  #[spirv(uniform, descriptor_set = 1, binding = 0)] consts: &Consts,
  #[spirv(storage_buffer, descriptor_set = 2, binding = 0)] materials: &[Vec4],
  #[spirv(storage_buffer, descriptor_set = 2, binding = 1)] spheres: &[Vec4],
) {
  let mut rng = Rng(id.x * consts.samples + id.y * consts.size.x);
  let uv = ((id.as_vec3().truncate() + rng.vec2()) / consts.size.as_vec2()) * 2.0 - Vec2::ONE;
  let mut ray = Ray {
    origin: Vec3::ZERO,
    dir: -(uv * Vec2::new(consts.size.x as f32 / consts.size.y as f32, 1.0) * 0.7.tan())
      .extend(1.0),
  };

  let mut color = Vec3::ONE;
  for _ in 0..MAX_BOUNCES {
    let mut closest = Hit {
      distance: f32::MAX,
      ..Default::default()
    };
    for i in 0..spheres.len() / 2 {
      let hit = Sphere {
        pos: spheres[2 * i].truncate(),
        radius: spheres[2 * i].w,
        mat: spheres[2 * i + 1].x as _,
      }
      .hit(&ray, 0.001..closest.distance);
      if hit.distance > 0.0 {
        closest = hit;
      }
    }

    if closest.distance != f32::MAX {
      let material = materials[closest.mat as usize];
      color *= material.truncate();
      ray.origin = closest.pos;
      ray.dir = match material.w as u32 {
        0 => closest.normal + rng.vec3_sphere().normalize(),
        1 => reflect(ray.dir, closest.normal),
        _ => break,
      };
    } else {
      color *= Vec3::new(0.5, 0.7, 1.0);
      break;
    };
  }

  // safety: this is our texel
  unsafe { tex.write(id.truncate(), tex.read(id.truncate()) + color.extend(1.0)) };
}

fn reflect(v: Vec3, n: Vec3) -> Vec3 {
  v - 2.0 * v.dot(n) * n
}

struct Ray {
  origin: Vec3,
  dir: Vec3,
}

impl Ray {
  fn at(&self, t: f32) -> Vec3 {
    self.origin + t * self.dir
  }
}

/// if distance=0.0 then no hit
#[derive(Default)]
struct Hit {
  distance: f32,
  pos: Vec3,
  normal: Vec3,
  mat: u32,
}

struct Sphere {
  pos: Vec3,
  radius: f32,
  mat: u32,
}

impl Sphere {
  fn hit(&self, ray: &Ray, range: Range<f32>) -> Hit {
    let oc = ray.origin - self.pos;
    let a = ray.dir.length_squared();
    let b = oc.dot(ray.dir);
    let c = oc.length_squared() - self.radius * self.radius;
    let disc = b * b - a * c;
    if disc < 0.0 {
      Hit::default()
    } else {
      let sqrtd = disc.sqrt();
      let mut distance = (-b - sqrtd) / a;
      if !range.contains(&distance) {
        distance = (-b + sqrtd) / a;
        if !range.contains(&distance) {
          return Hit::default();
        }
      }
      let pos = ray.at(distance);
      let normal = (pos - self.pos) / self.radius;
      Hit {
        distance,
        pos,
        normal,
        mat: self.mat,
      }
    }
  }
}

/// pcg random number generator
struct Rng(u32);

impl Rng {
  fn u32(&mut self) -> u32 {
    self.0 = self.0 * 747796405 + 2891336453;
    let w = ((self.0 >> ((self.0 >> 28) + 4)) ^ self.0) * 277803737;
    (w >> 22) ^ w
  }

  fn f32_pos(&mut self) -> f32 {
    self.u32() as f32 / u32::MAX as f32
  }

  fn f32(&mut self) -> f32 {
    self.f32_pos() * 2.0 - 1.0
  }

  fn vec2(&mut self) -> Vec2 {
    Vec2::new(self.f32(), self.f32())
  }

  fn vec3(&mut self) -> Vec3 {
    Vec3::new(self.f32(), self.f32(), self.f32())
  }

  fn vec3_sphere(&mut self) -> Vec3 {
    let u = self.f32_pos();
    let v = self.f32();
    let theta = u * 2.0 * PI;
    let phi = v.acos();
    self.f32_pos().cbrt() * Vec3::new(phi.sin() * theta.cos(), phi.sin() * theta.sin(), phi.cos())
  }
}
