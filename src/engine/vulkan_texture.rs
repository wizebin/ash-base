use std::sync::{Arc, Mutex};
use std::mem;

use ash::util::Align;
use ash::{vk, Device};

use super::vulkan_image::VulkanImage;
use super::vulkan_sampler::VulkanSampler;
use super::{dimensions::Dimensions, memory::find_memorytype_index};


pub struct VulkanTexture {
  pub device: Arc<Mutex<Device>>,
  pub texture_image: vk::Image,
  pub format: vk::Format,
  pub texture_memory: vk::DeviceMemory,
}

impl VulkanTexture {
  pub unsafe fn new_from_image(vulkan_image: &VulkanImage, device: Arc<Mutex<Device>>, device_memory_properties: vk::PhysicalDeviceMemoryProperties) -> Self {
    let texture_create_info = vk::ImageCreateInfo {
        image_type: vk::ImageType::TYPE_2D,
        format: vk::Format::R8G8B8A8_UNORM,
        extent: vulkan_image.extent().into(),
        mip_levels: 1,
        array_layers: 1,
        samples: vk::SampleCountFlags::TYPE_1,
        tiling: vk::ImageTiling::OPTIMAL,
        usage: vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED,
        sharing_mode: vk::SharingMode::EXCLUSIVE,
        ..Default::default()
    };
    let locked_device = device.clone();
    let locked_device = locked_device.lock().unwrap();

    let texture_image = locked_device
        .create_image(&texture_create_info, None)
        .unwrap();
    let texture_memory_req = locked_device.get_image_memory_requirements(texture_image);
    let texture_memory_index = find_memorytype_index(
        &texture_memory_req,
        &device_memory_properties,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
    )
    .expect("Unable to find suitable memory index for depth image.");

    let texture_allocate_info = vk::MemoryAllocateInfo {
        allocation_size: texture_memory_req.size,
        memory_type_index: texture_memory_index,
        ..Default::default()
    };
    let texture_memory = locked_device
        .allocate_memory(&texture_allocate_info, None)
        .unwrap();
    locked_device
        .bind_image_memory(texture_image, texture_memory, 0)
        .expect("Unable to bind depth image memory");

    Self {
      device: device.clone(),
      texture_image,
      format: texture_create_info.format,
      texture_memory,
    }
  }
}

impl Drop for VulkanTexture {
  fn drop(&mut self) {
    let locked_device = self.device.lock().unwrap();
    unsafe {
        locked_device.free_memory(self.texture_memory, None);
        locked_device.destroy_image(self.texture_image, None);
    }
  }
}

// let tex_image_view_info = vk::ImageViewCreateInfo {
//     view_type: vk::ImageViewType::TYPE_2D,
//     format: tex.format,
//     components: vk::ComponentMapping {
//         r: vk::ComponentSwizzle::R,
//         g: vk::ComponentSwizzle::G,
//         b: vk::ComponentSwizzle::B,
//         a: vk::ComponentSwizzle::A,
//     },
//     subresource_range: vk::ImageSubresourceRange {
//         aspect_mask: vk::ImageAspectFlags::COLOR,
//         level_count: 1,
//         layer_count: 1,
//         ..Default::default()
//     },
//     image: tex.texture_image,
//     ..Default::default()
// };
// let tex_image_view = base
//     .shared_device().lock().unwrap()
//     .create_image_view(&tex_image_view_info, None)
//     .unwrap();

pub struct VulkanTextureView {
  pub device: Arc<Mutex<Device>>,
  pub texture_image_view: vk::ImageView,
}

impl VulkanTextureView {
  pub fn new(device: Arc<Mutex<Device>>, texture: &VulkanTexture) -> Self {
    let tex_image_view_info = vk::ImageViewCreateInfo {
        view_type: vk::ImageViewType::TYPE_2D,
        format: texture.format,
        components: vk::ComponentMapping {
            r: vk::ComponentSwizzle::R,
            g: vk::ComponentSwizzle::G,
            b: vk::ComponentSwizzle::B,
            a: vk::ComponentSwizzle::A,
        },
        subresource_range: vk::ImageSubresourceRange {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            level_count: 1,
            layer_count: 1,
            ..Default::default()
        },
        image: texture.texture_image,
        ..Default::default()
    };
    let locked_device = device.clone();
    let locked_device = locked_device.lock().unwrap();

    let texture_image_view = unsafe { locked_device
        .create_image_view(&tex_image_view_info, None)
        .unwrap() };

    Self {
      device,
      texture_image_view,
    }
  }

  pub fn get_descriptor_info(&self, sampler: &VulkanSampler) -> vk::DescriptorImageInfo {
    vk::DescriptorImageInfo {
        image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        image_view: self.texture_image_view,
        sampler: sampler.sampler,
    }
  }
}

impl Drop for VulkanTextureView {
  fn drop(&mut self) {
    let locked_device = self.device.lock().unwrap();
    unsafe {
        locked_device.destroy_image_view(self.texture_image_view, None);
    }
  }
}
