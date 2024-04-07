use std::sync::{Arc, Mutex};

use ash::{Device, vk::{self, Extent2D}};

use super::memory::find_memorytype_index;

pub struct VulkanDepthImage {
    pub device: Arc<Mutex<Device>>,
    pub depth_image: vk::Image,
    pub depth_image_view: vk::ImageView,
    pub depth_image_memory: vk::DeviceMemory,
    pub dropped: bool,
}

impl VulkanDepthImage {
    pub unsafe fn new(surface_resolution: Extent2D, device: Arc<Mutex<Device>>, device_memory_properties: vk::PhysicalDeviceMemoryProperties) -> Self {
        let locked_device = device.clone();
        let locked_device = locked_device.lock().unwrap();

        let depth_image_create_info = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .format(vk::Format::D16_UNORM)
            .extent(surface_resolution.into())
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let depth_image = locked_device.create_image(&depth_image_create_info, None).unwrap();
        let depth_image_memory_req = locked_device.get_image_memory_requirements(depth_image);
        let depth_image_memory_index = find_memorytype_index(
            &depth_image_memory_req,
            &device_memory_properties,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )
        .expect("Unable to find suitable memory index for depth image.");

        let depth_image_allocate_info = vk::MemoryAllocateInfo::default()
            .allocation_size(depth_image_memory_req.size)
            .memory_type_index(depth_image_memory_index);

        let depth_image_memory = locked_device
            .allocate_memory(&depth_image_allocate_info, None)
            .unwrap();

        locked_device
            .bind_image_memory(depth_image, depth_image_memory, 0)
            .expect("Unable to bind depth image memory");

        let depth_image_view_info = vk::ImageViewCreateInfo::default()
            .subresource_range(
                vk::ImageSubresourceRange::default()
                    .aspect_mask(vk::ImageAspectFlags::DEPTH)
                    .level_count(1)
                    .layer_count(1),
            )
            .image(depth_image)
            .format(depth_image_create_info.format)
            .view_type(vk::ImageViewType::TYPE_2D);

        let depth_image_view = locked_device
            .create_image_view(&depth_image_view_info, None)
            .unwrap();

        Self {
            device,
            depth_image,
            depth_image_memory,
            depth_image_view,
            dropped: false,
        }
    }

    pub fn intentionally_free(&mut self) {
        if self.dropped {
            return;
        }

        unsafe {
            let locked_device = self.device.clone();
            let locked_device = locked_device.lock().unwrap();

            locked_device.free_memory(self.depth_image_memory, None);
            locked_device.destroy_image_view(self.depth_image_view, None);
            locked_device.destroy_image(self.depth_image, None);
            self.dropped = true;
        }
    }
}

impl Drop for VulkanDepthImage {
    fn drop(&mut self) {
        self.intentionally_free();
    }
}
