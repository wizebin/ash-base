use std::sync::{Arc, Mutex};
use std::mem;

use ash::util::Align;
use ash::{vk, Device};

use super::vec3::Vector3;
use super::vulkan_image::VulkanImage;
use super::{dimensions::Dimensions, memory::find_memorytype_index};

pub struct VulkanUniformBufferObject {
  pub device: Arc<Mutex<Device>>,
  pub uniform_color_buffer: vk::Buffer,
  pub uniform_color_buffer_memory: vk::DeviceMemory,
  pub color_vector: Vector3,
}

impl VulkanUniformBufferObject {
  pub unsafe fn new_from_vec3(uniform_color_buffer_data: Vector3, device: Arc<Mutex<Device>>, device_memory_properties: vk::PhysicalDeviceMemoryProperties) -> Self {
    let uniform_color_buffer_info = vk::BufferCreateInfo {
        size: mem::size_of_val(&uniform_color_buffer_data) as u64,
        usage: vk::BufferUsageFlags::UNIFORM_BUFFER,
        sharing_mode: vk::SharingMode::EXCLUSIVE,
        ..Default::default()
    };

    let locked_device = device.clone();
    let locked_device = locked_device.lock().unwrap();

    let uniform_color_buffer = locked_device
        .create_buffer(&uniform_color_buffer_info, None)
        .unwrap();
    let uniform_color_buffer_memory_req = locked_device
        .get_buffer_memory_requirements(uniform_color_buffer);
    let uniform_color_buffer_memory_index = find_memorytype_index(
        &uniform_color_buffer_memory_req,
        &device_memory_properties,
        vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
    )
    .expect("Unable to find suitable memorytype for the uniform color buffer.");

    let uniform_color_buffer_allocate_info = vk::MemoryAllocateInfo {
        allocation_size: uniform_color_buffer_memory_req.size,
        memory_type_index: uniform_color_buffer_memory_index,
        ..Default::default()
    };
    let uniform_color_buffer_memory = locked_device
        .allocate_memory(&uniform_color_buffer_allocate_info, None)
        .unwrap();
    let uniform_ptr = locked_device
        .map_memory(
            uniform_color_buffer_memory,
            0,
            uniform_color_buffer_memory_req.size,
            vk::MemoryMapFlags::empty(),
        )
        .unwrap();
    let mut uniform_aligned_slice = Align::new(
        uniform_ptr,
        mem::align_of::<Vector3>() as u64,
        uniform_color_buffer_memory_req.size,
    );
    uniform_aligned_slice.copy_from_slice(&[uniform_color_buffer_data]);
    locked_device.unmap_memory(uniform_color_buffer_memory);
    locked_device
        .bind_buffer_memory(uniform_color_buffer, uniform_color_buffer_memory, 0)
        .unwrap();

    Self {
        color_vector: uniform_color_buffer_data,
        device: device.clone(),
        uniform_color_buffer,
        uniform_color_buffer_memory,
    }
  }

  pub fn get_descriptor_info(&self, offset: u64) -> vk::DescriptorBufferInfo {
    vk::DescriptorBufferInfo {
        buffer: self.uniform_color_buffer,
        offset,
        range: mem::size_of_val(&self.color_vector) as u64,
    }

  }
}

impl Drop for VulkanUniformBufferObject {
  fn drop(&mut self) {
    let locked_device = self.device.lock().unwrap();
    unsafe {
        locked_device.free_memory(self.uniform_color_buffer_memory, None);
        locked_device.destroy_buffer(self.uniform_color_buffer, None);
    }
  }
}
