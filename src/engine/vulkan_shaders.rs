use std::sync::{Arc, Mutex};
use std::mem;

use ash::util::{read_spv, Align};
use ash::{vk, Device};

pub struct VulkanShader {
    pub device: Arc<Mutex<Device>>,
    pub shader_module: vk::ShaderModule,
}

impl VulkanShader {
    pub fn new(device: Arc<Mutex<Device>>, mut code: std::io::Cursor<&[u8]>) -> Self {
        let code = read_spv(&mut code).expect("Failed to read shader data");
        let shader_info = vk::ShaderModuleCreateInfo::default().code(&code);

        let locked_device = device.clone();
        let locked_device = locked_device.lock().unwrap();

        let shader_module = unsafe { locked_device
            .create_shader_module(&shader_info, None)
            .expect("Failed to load shader") };

        Self { device, shader_module }
    }
}

impl Drop for VulkanShader {
    fn drop(&mut self) {
        let locked_device = self.device.clone();
        let locked_device = locked_device.lock().unwrap();

        unsafe {
            locked_device.destroy_shader_module(self.shader_module, None);
        }
    }
}
