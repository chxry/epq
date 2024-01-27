#![no_std]
use glam::UVec2;

#[repr(C, align(16))]
#[derive(Default)]
pub struct Consts {
  pub size: UVec2,
  pub samples: u32,
}
