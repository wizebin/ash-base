use std::sync::{Arc, Mutex};
use std::mem;

use ash::util::Align;
use ash::{vk, Device};

pub struct VulkanSampler {
  pub device: Arc<Mutex<Device>>,
  pub sampler: vk::Sampler,
}

impl VulkanSampler {
  pub fn new(device: Arc<Mutex<Device>>) -> Self {
    let sampler_info = vk::SamplerCreateInfo {
      mag_filter: vk::Filter::LINEAR,
      min_filter: vk::Filter::LINEAR,
      mipmap_mode: vk::SamplerMipmapMode::LINEAR,
      address_mode_u: vk::SamplerAddressMode::MIRRORED_REPEAT,
      address_mode_v: vk::SamplerAddressMode::MIRRORED_REPEAT,
      address_mode_w: vk::SamplerAddressMode::MIRRORED_REPEAT,
      max_anisotropy: 1.0,
      border_color: vk::BorderColor::FLOAT_OPAQUE_WHITE,
      compare_op: vk::CompareOp::NEVER,
      ..Default::default()
    };

    let locked_device = device.clone();
    let locked_device = locked_device.lock().unwrap();

    let sampler = unsafe { locked_device.create_sampler(&sampler_info, None).unwrap() };

    Self { device, sampler }
  }
}

impl Drop for VulkanSampler {
  fn drop(&mut self) {
    let locked_device = self.device.lock().unwrap();
    unsafe {
      locked_device.destroy_sampler(self.sampler, None);
    }
  }
}
