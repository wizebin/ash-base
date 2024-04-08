use std::sync::{Mutex, Arc};

use ash::{
    ext::debug_utils,
    khr::{surface, swapchain},
    vk::{self, SurfaceKHR}, Device, Entry, Instance,
};

use winit::{
    raw_window_handle::{HasDisplayHandle, HasWindowHandle},
    window::Window,
};

pub fn get_surface_capabilities(physical_device: &vk::PhysicalDevice, surface_loader: &surface::Instance, surface: SurfaceKHR) -> vk::SurfaceCapabilitiesKHR {
    unsafe {
        surface_loader
            .get_physical_device_surface_capabilities(*physical_device, surface)
            .expect("Failed to get surface capabilities.")
    }
}

pub fn get_standard_surface_image_count(surface_capabilities: &vk::SurfaceCapabilitiesKHR) -> u32 {
    let mut desired_image_count = surface_capabilities.min_image_count + 1;
    if surface_capabilities.max_image_count > 0 && desired_image_count > surface_capabilities.max_image_count {
        desired_image_count = surface_capabilities.max_image_count;
    }
    desired_image_count
}

pub fn get_surface_capabilities_pre_transform(surface_capabilities: &vk::SurfaceCapabilitiesKHR) -> vk::SurfaceTransformFlagsKHR {
    if surface_capabilities
        .supported_transforms
        .contains(vk::SurfaceTransformFlagsKHR::IDENTITY)
    {
        vk::SurfaceTransformFlagsKHR::IDENTITY
    } else {
        surface_capabilities.current_transform
    }
}

pub struct VulkanSurface {
    pub surface_loader: surface::Instance,
    pub surface: SurfaceKHR,
}

impl VulkanSurface {
    pub fn new(entry: &Entry, instance: &Instance, window: Arc<Mutex<Window>>) -> Self {
        let locked_window = window.clone();
        let locked_window = locked_window.lock().unwrap();

        let surface = unsafe { ash_window::create_surface(
            entry,
            instance,
            locked_window.display_handle().unwrap().as_raw(),
            locked_window.window_handle().unwrap().as_raw(),
            None,
        )
        .unwrap() };

        let surface_loader = surface::Instance::new(&entry, &instance);

        Self {
            surface_loader,
            surface,
        }
    }
}

impl Drop for VulkanSurface {
    fn drop(&mut self) {
        unsafe {
            self.surface_loader.destroy_surface(self.surface, None);
        }
    }
}
