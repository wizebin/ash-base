use std::sync::{Arc, Mutex};
use std::mem;

use ash::util::Align;
use ash::{vk, Device};

pub struct VulkanCommandPool {
    pub command_pool: vk::CommandPool,
    pub device: Arc<Mutex<Device>>,
}

impl VulkanCommandPool {
    pub unsafe fn new(device: Arc<Mutex<Device>>, queue_family_index: u32) -> Self {
        let command_pool_info = vk::CommandPoolCreateInfo::default()
            .queue_family_index(queue_family_index)
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);

        let locked_device = device.clone();
        let locked_device = locked_device.lock().unwrap();

        let command_pool = locked_device
            .create_command_pool(&command_pool_info, None)
            .expect("Failed to create command pool.");

        Self {
            command_pool,
            device,
        }
    }
}

impl Drop for VulkanCommandPool {
    fn drop(&mut self) {
        unsafe {
            let locked_device = self.device.clone();
            let locked_device = locked_device.lock().unwrap();
            locked_device.destroy_command_pool(self.command_pool, None);
        }
    }
}

pub fn create_command_buffers(command_pool: &VulkanCommandPool, device: Arc<Mutex<Device>>) -> (vk::CommandBuffer, vk::CommandBuffer) {
    let command_buffer_allocate_info = vk::CommandBufferAllocateInfo::default()
        .command_buffer_count(2)
        .command_pool(command_pool.command_pool)
        .level(vk::CommandBufferLevel::PRIMARY);

    let locked_device = device.clone();
    let locked_device = locked_device.lock().unwrap();
    let command_buffers = unsafe {locked_device
        .allocate_command_buffers(&command_buffer_allocate_info)
        .unwrap() };
    let setup_command_buffer = command_buffers[0];
    let draw_command_buffer = command_buffers[1];

    (setup_command_buffer, draw_command_buffer)
}

pub fn get_device_presentation_queue(device: Arc<Mutex<Device>>, queue_family_index: u32) -> vk::Queue {
    let locked_device = device.clone();
    let locked_device = locked_device.lock().unwrap();

    unsafe { locked_device.get_device_queue(queue_family_index, 0) }
}
