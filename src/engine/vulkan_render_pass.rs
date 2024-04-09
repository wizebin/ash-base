use std::sync::{Arc, Mutex};

use ash::vk;

use super::vulkan_attachments::{make_color_attachment, make_color_subpass_dependency, make_depth_attachment, make_standard_depth_color_attachments};

pub struct VulkanColorDepthRenderPass {
    pub device: Arc<Mutex<ash::Device>>,
    pub render_pass: vk::RenderPass,
}

impl VulkanColorDepthRenderPass {
    pub fn new(device: Arc<Mutex<ash::Device>>, surface_format: vk::Format) -> Self {
        let locked_device = device.clone();
        let locked_device = locked_device.lock().unwrap();

        let renderpass_attachments = make_standard_depth_color_attachments(surface_format);

        let color_attachment_refs = [make_color_attachment(0)];
        let depth_attachment_ref = make_depth_attachment(1);
        let dependencies = [make_color_subpass_dependency()];

        let subpass = vk::SubpassDescription::default()
            .color_attachments(&color_attachment_refs)
            .depth_stencil_attachment(&depth_attachment_ref)
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS);

        let renderpass_create_info = vk::RenderPassCreateInfo::default()
            .attachments(&renderpass_attachments)
            .subpasses(std::slice::from_ref(&subpass))
            .dependencies(&dependencies);

        let render_pass = unsafe { locked_device
            .create_render_pass(&renderpass_create_info, None)
            .unwrap() };

        Self { render_pass, device }
    }
}

impl Drop for VulkanColorDepthRenderPass {
    fn drop(&mut self) {
        let locked_device = self.device.lock().unwrap();
        unsafe {
            locked_device.destroy_render_pass(self.render_pass, None);
        }
    }
}
