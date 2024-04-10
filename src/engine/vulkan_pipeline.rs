use std::sync::{Arc, Mutex};
use std::mem;

use ash::util::Align;
use ash::{vk, Device};

use super::vulkan_descriptor::VulkanDescriptorSetLayouts;

// let layout_create_info =
//                 vk::PipelineLayoutCreateInfo::default().set_layouts(&descriptor_set_layouts.descriptor_set_layouts);

//             let pipeline_layout = base
//                 .shared_device().lock().unwrap()
//                 .create_pipeline_layout(&layout_create_info, None)
//                 .unwrap();

pub struct VulkanPipelineLayout {
    pub device: Arc<Mutex<Device>>,
    pub pipeline_layout: vk::PipelineLayout,
}

impl VulkanPipelineLayout {
    pub fn new(device: Arc<Mutex<Device>>, descriptor_set_layouts: &VulkanDescriptorSetLayouts) -> Self {
        let layout_create_info =
            vk::PipelineLayoutCreateInfo::default().set_layouts(&descriptor_set_layouts.descriptor_set_layouts);

        let locked_device = device.clone();
        let locked_device = locked_device.lock().unwrap();

        let pipeline_layout = unsafe {
            locked_device
                .create_pipeline_layout(&layout_create_info, None)
                .unwrap()
        };

        Self { device, pipeline_layout }
    }
}

impl Drop for VulkanPipelineLayout {
    fn drop(&mut self) {
        let locked_device = self.device.clone();
        let locked_device = locked_device.lock().unwrap();

        unsafe {
            locked_device.destroy_pipeline_layout(self.pipeline_layout, None);
        }
    }
}


pub struct VulkanPipeline {
    pub device: Arc<Mutex<Device>>,
    pub pipeline: vk::Pipeline,
}

impl VulkanPipeline {
    pub fn new(device: Arc<Mutex<Device>>, pipeline_create_info: vk::GraphicsPipelineCreateInfo) -> Self {
        let locked_device = device.clone();
        let locked_device = locked_device.lock().unwrap();

        let pipeline = unsafe {
            locked_device
                .create_graphics_pipelines(vk::PipelineCache::null(), &[pipeline_create_info], None)
                .expect("Failed to create graphics pipeline")[0]
        };

        Self { device, pipeline }
    }
}

impl Drop for VulkanPipeline {
    fn drop(&mut self) {
        let locked_device = self.device.clone();
        let locked_device = locked_device.lock().unwrap();

        unsafe {
            locked_device.destroy_pipeline(self.pipeline, None);
        }
    }
}
