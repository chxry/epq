use core::ops::Range;
use core::f32::consts::{PI, FRAC_1_PI};
use spirv_std::{spirv, Sampler};
use spirv_std::image::Image2d;
use spirv_std::num_traits::Float;
use spirv_std::glam::{UVec3, Vec4, Vec3, Vec2};
use shared::Consts;
use crate::StorageImage;

const MAX_BOUNCES: u32 = 16;

#[spirv(compute(threads(16, 16)))]
pub fn main(
  #[spirv(global_invocation_id)] id: UVec3,
  #[spirv(descriptor_set = 0, binding = 0)] tex: &StorageImage,
  #[spirv(uniform, descriptor_set = 1, binding = 0)] consts: &Consts,
  #[spirv(storage_buffer, descriptor_set = 2, binding = 0)] materials: &[Vec4],
  #[spirv(storage_buffer, descriptor_set = 2, binding = 1)] spheres: &[Vec4],
  #[spirv(descriptor_set = 2, binding = 2)] sky: &Image2d,
  #[spirv(descriptor_set = 3, binding = 0)] sampler: &Sampler,
) {
  let mut rng = Rng(id.x * consts.samples + id.y * consts.size.x);
  let uv = ((id.as_vec3().truncate() + rng.vec2()) / consts.size.as_vec2()) * 2.0 - Vec2::ONE;
  let cam_pos = Vec3::ZERO;
  let mut ray = Ray {
    origin: cam_pos,
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
      // lambertian bsdf
      let light = uniform_sample(closest.normal, &mut rng);
      color *= lambert_brdf(material.truncate()) * closest.normal.dot(light) / uniform_pdf();

      ray.origin = closest.pos;
      ray.dir = light;
      // break;

      // ray.dir = match material.w as u32 {
      //   0 => closest.normal + rng.vec3_sphere(),
      //   1 => reflect(ray.dir, closest.normal),
      //   _ => break,
      // };
    } else {
      color *= sky
        .sample_by_lod(*sampler, sample_equirect(ray.dir), 1.0)
        .truncate();
      break;
    };
  }

  let weight = 1.0 / consts.samples as f32;
  // safety: this is our texel
  unsafe {
    tex.write(
      id.truncate(),
      tex.read(id.truncate()) * (1.0 - weight) + color.extend(1.0) * weight,
    )
  };
}

fn reflect(v: Vec3, n: Vec3) -> Vec3 {
  v - 2.0 * v.dot(n) * n
}

fn lambert_brdf(color: Vec3) -> Vec3 {
  color * FRAC_1_PI
}

fn uniform_pdf() -> f32 {
  FRAC_1_PI / 2.0
}

fn uniform_sample(n: Vec3, rng: &mut Rng) -> Vec3 {
  let z = rng.f32();
  let r = (1.0 - z * z).max(0.0).sqrt();
  let phi = 2.0 * PI * rng.f32();
  (n + Vec3::new(r * phi.cos(), r * phi.sin(), z)).normalize()
}

/// schlick approximation for dielectric fresnel
fn schlick(cos: f32, ir: f32) -> f32 {
  let r0 = ((1.0 - ir) / (1.0 + ir)).powf(2.0);
  r0 + (1.0 - r0) * (1.0 - cos).powf(5.0)
}

fn ggx_distribution(nh: f32, a: f32) -> f32 {
  let a2 = a * a;
  let d = nh * nh * (a2 - 1.0) + 1.0;
  a2 / (PI * d * d)
}

fn smith_vis(nv: f32, nl: f32, a: f32) -> f32 {
  let v = nl * (nv * (1.0 - a) + a);
  let l = nv * (nl * (1.0 - a) + a);

  return 0.5 / (v + l);
}

pub fn orthonormal_basis(v: &Vec3) -> (Vec3, Vec3) {
  // From https://graphics.pixar.com/library/OrthonormalB/paper.pdf
  let sign = if v.z > 0.0 { 1.0 } else { 0.0 };
  let a = -1.0 / (sign + v.z);
  let b = v.x * v.y * a;
  (
    Vec3::new(1.0 + sign * v.x * v.x * a, sign * b, -sign * v.x),
    Vec3::new(b, sign + v.y * v.y * a, -v.y),
  )
}

/// given spherical coordinates returns equirectangular coordinates
fn sample_equirect(dir: Vec3) -> Vec2 {
  Vec2::new(dir.z.atan2(dir.x) + PI, dir.y.acos()) / Vec2::new(2.0 * PI, PI)
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
