use std::sync::{Arc, Mutex};
use std::mem;

use ash::util::Align;
use ash::{vk, Device};

use super::{dimensions::Dimensions, memory::find_memorytype_index};


pub struct VulkanImage {
  pub dimensions: Dimensions,
  pub device: Arc<Mutex<Device>>,
  pub image_buffer: vk::Buffer,
  pub image_buffer_memory: vk::DeviceMemory,
}

impl VulkanImage {
  pub unsafe fn new_from_bytes(bytes: &'static [u8], device: Arc<Mutex<Device>>, device_memory_properties: vk::PhysicalDeviceMemoryProperties) -> Self {
    let loaded_image = image::load_from_memory(bytes).unwrap().to_rgba8();
    let (width, height) = loaded_image.dimensions();
    let image_dimensions = Dimensions::new(width, height, 0);

    let image_data = loaded_image.into_raw();

    let image_buffer_info = vk::BufferCreateInfo {
        size: (mem::size_of::<u8>() * image_data.len()) as u64,
        usage: vk::BufferUsageFlags::TRANSFER_SRC,
        sharing_mode: vk::SharingMode::EXCLUSIVE,
        ..Default::default()
    };


    let locked_device = device.clone();
    let locked_device = locked_device.lock().unwrap();

    let image_buffer = locked_device.create_buffer(&image_buffer_info, None).unwrap();
    let image_buffer_memory_req = locked_device.get_buffer_memory_requirements(image_buffer);

    let image_buffer_memory_index = find_memorytype_index(
        &image_buffer_memory_req,
        &device_memory_properties,
        vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
    )
    .expect("Unable to find suitable memorytype for the image buffer.");

    let image_buffer_allocate_info = vk::MemoryAllocateInfo {
        allocation_size: image_buffer_memory_req.size,
        memory_type_index: image_buffer_memory_index,
        ..Default::default()
    };

    let image_buffer_memory = locked_device
        .allocate_memory(&image_buffer_allocate_info, None)
        .unwrap();
    let image_ptr = locked_device
        .map_memory(
            image_buffer_memory,
            0,
            image_buffer_memory_req.size,
            vk::MemoryMapFlags::empty(),
        )
        .unwrap();
    let mut image_slice = Align::new(
        image_ptr,
        mem::align_of::<u8>() as u64,
        image_buffer_memory_req.size,
    );
    image_slice.copy_from_slice(&image_data);
    locked_device.unmap_memory(image_buffer_memory);
    locked_device
        .bind_buffer_memory(image_buffer, image_buffer_memory, 0)
        .unwrap();

    Self {
      dimensions: image_dimensions,
      device: device.clone(),
      image_buffer,
      image_buffer_memory,
    }
  }

  pub fn extent(&self) -> vk::Extent2D {
    self.dimensions.extent2d()
  }
}

impl Drop for VulkanImage {
  fn drop(&mut self) {
    let locked_device = self.device.lock().unwrap();
    unsafe {
      locked_device.free_memory(self.image_buffer_memory, None);
      locked_device.destroy_buffer(self.image_buffer, None);
    }
  }
}
