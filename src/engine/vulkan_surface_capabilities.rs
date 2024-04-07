use ash::{
    ext::debug_utils,
    khr::{surface, swapchain},
    vk::{self, SurfaceKHR}, Device, Entry, Instance,
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
