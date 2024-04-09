use std::sync::{Arc, Mutex};
use ash::vk;

use super::{vulkan_depth_image::VulkanDepthImage, vulkan_render_pass::VulkanColorDepthRenderPass};

pub struct VulkanFramebuffers {
    pub device: Arc<Mutex<ash::Device>>,
    pub framebuffers: Vec<vk::Framebuffer>,
}

impl VulkanFramebuffers {
    pub fn new(
        device: Arc<Mutex<ash::Device>>,
        surface_resolution: vk::Extent2D,
        renderpass: &VulkanColorDepthRenderPass,
        depth_img: &VulkanDepthImage,
        present_image_views: &Vec<vk::ImageView>,
    ) -> Self {
        let framebuffers: Vec<vk::Framebuffer> = present_image_views
            .iter()
            .map(|&present_image_view| {
                let locked_device = device.clone();
                let locked_device = locked_device.lock().unwrap();

                let framebuffer_attachments = [present_image_view, depth_img.depth_image_view];
                let frame_buffer_create_info = vk::FramebufferCreateInfo::default()
                    .render_pass(renderpass.render_pass)
                    .attachments(&framebuffer_attachments)
                    .width(surface_resolution.width)
                    .height(surface_resolution.height)
                    .layers(1);

                unsafe { locked_device
                    .create_framebuffer(&frame_buffer_create_info, None)
                    .unwrap() }
            })
            .collect();

        Self { framebuffers, device }
    }
}

impl Drop for VulkanFramebuffers {
    fn drop(&mut self) {
        let locked_device = self.device.lock().unwrap();
        for framebuffer in self.framebuffers.iter() {
            unsafe {
                locked_device.destroy_framebuffer(*framebuffer, None);
            }
        }
    }
}
