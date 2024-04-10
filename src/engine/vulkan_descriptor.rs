use std::sync::{Arc, Mutex};
use std::mem;

use ash::util::Align;
use ash::{vk, Device};

pub fn make_ubo_pool_size(size: u32) -> vk::DescriptorPoolSize {
    vk::DescriptorPoolSize {
        ty: vk::DescriptorType::UNIFORM_BUFFER,
        descriptor_count: size,
    }
}
pub fn make_image_sampler_pool_size(size: u32) -> vk::DescriptorPoolSize {
    vk::DescriptorPoolSize {
        ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
        descriptor_count: size,
    }
}

pub struct VulkanDescriptorPool {
    pub device: Arc<Mutex<Device>>,
    pub descriptor_pool: vk::DescriptorPool,
    pub source_descriptor_sets: Vec<vk::DescriptorSet>,
}

impl VulkanDescriptorPool {
    pub fn new(device: Arc<Mutex<Device>>, descriptor_sizes: Vec<vk::DescriptorPoolSize>) -> Self {
        let descriptor_pool_info = vk::DescriptorPoolCreateInfo::default()
            .pool_sizes(&descriptor_sizes)
            .max_sets(1);

        let locked_device = device.clone();
        let locked_device = locked_device.lock().unwrap();

        let descriptor_pool = unsafe { locked_device
            .create_descriptor_pool(&descriptor_pool_info, None)
            .unwrap() };

        Self { device, descriptor_pool, source_descriptor_sets: Vec::new() }
    }

    pub fn create_source_descriptor_sets_releasing_old(&mut self, descriptor_set_layouts: &VulkanDescriptorSetLayouts) {
        let descriptor_set_allocate_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(self.descriptor_pool)
            .set_layouts(&descriptor_set_layouts.descriptor_set_layouts);

        let locked_device = self.device.clone();
        let locked_device = locked_device.lock().unwrap();

        if self.source_descriptor_sets.len() > 0 {
            unsafe {
                // locked_device.free_descriptor_sets(self.descriptor_pool, &self.source_descriptor_sets);
            }
            self.source_descriptor_sets.clear();
        }

        let descriptor_sets = unsafe { locked_device
            .allocate_descriptor_sets(&descriptor_set_allocate_info)
            .unwrap() };

        self.source_descriptor_sets = descriptor_sets;
    }
}

impl Drop for VulkanDescriptorPool {
    fn drop(&mut self) {
        let locked_device = self.device.lock().unwrap();
        unsafe {
            for layout in self.source_descriptor_sets.iter() {
                unsafe {
                    // locked_device.free_descriptor_sets(self.descriptor_pool, &[*layout]);
                }
            }
            locked_device.destroy_descriptor_pool(self.descriptor_pool, None);
        }
    }
}

pub struct VulkanDescriptorSetLayouts {
    pub device: Arc<Mutex<Device>>,
    pub descriptor_set_layouts: Vec<vk::DescriptorSetLayout>,
}

impl VulkanDescriptorSetLayouts {
    pub fn new(device: Arc<Mutex<Device>>, bindings: Vec<vk::DescriptorSetLayoutBinding>) -> Self {
        let descriptor_set_layout_info = vk::DescriptorSetLayoutCreateInfo::default()
            .bindings(&bindings);

        let locked_device = device.clone();
        let locked_device = locked_device.lock().unwrap();

        let descriptor_set_layout = unsafe { locked_device
            .create_descriptor_set_layout(&descriptor_set_layout_info, None)
            .unwrap() };

        Self { device, descriptor_set_layouts: vec![descriptor_set_layout] }
    }
}

impl Drop for VulkanDescriptorSetLayouts {
    fn drop(&mut self) {
        let locked_device = self.device.lock().unwrap();
        for layout in self.descriptor_set_layouts.iter() {
            unsafe {
                locked_device.destroy_descriptor_set_layout(*layout, None);
            }
        }
    }
}

pub fn update_device_descriptor_sets(device: Arc<Mutex<Device>>, descriptor_sets: &Vec<vk::WriteDescriptorSet>) {
    let locked_device = device.clone();
    let locked_device = locked_device.lock().unwrap();

    unsafe {
        locked_device.update_descriptor_sets(&descriptor_sets, &[]);
    }
}
