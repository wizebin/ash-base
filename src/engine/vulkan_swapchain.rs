use std::sync::{Mutex, Arc};

use ash::{
    ext::debug_utils,
    khr::{surface, swapchain},
    vk::{self, SurfaceFormatKHR, SurfaceKHR}, Device, Entry, Instance,
};

use winit::{
    raw_window_handle::{HasDisplayHandle, HasWindowHandle},
    window::Window,
};

use super::{vulkan_physical_device::get_mailbox_or_fifo_present_mode, vulkan_surface::{get_standard_surface_image_count, get_surface_capabilities, get_surface_capabilities_pre_transform, VulkanSurface}};


pub fn create_standard_swapchain(physical_device: &vk::PhysicalDevice, surface: &VulkanSurface, surface_format: SurfaceFormatKHR, dimensions: vk::Extent2D, swapchain_device: &swapchain::Device) -> vk::SwapchainKHR {

    let surface_capabilities = get_surface_capabilities(&physical_device, &surface.surface_loader, surface.surface);
    let desired_image_count = get_standard_surface_image_count(&surface_capabilities);

    let pre_transform = get_surface_capabilities_pre_transform(&surface_capabilities);

    let swapchain_create_info = vk::SwapchainCreateInfoKHR::default()
        .surface(surface.surface)
        .min_image_count(desired_image_count)
        .image_color_space(surface_format.color_space)
        .image_format(surface_format.format)
        .image_extent(dimensions)
        .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
        .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
        .pre_transform(pre_transform)
        .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
        .present_mode(get_mailbox_or_fifo_present_mode(physical_device, &surface.surface_loader, surface.surface))
        .clipped(true)
        .image_array_layers(1);

    unsafe {
        swapchain_device
            .create_swapchain(&swapchain_create_info, None)
            .expect("Failed to create swapchain.")
    }
}

pub fn get_swapchain_image_views(logical_device: Arc<Mutex<ash::Device>>, swapchain_device: &swapchain::Device, swapchain: vk::SwapchainKHR, surface_format: SurfaceFormatKHR) -> (Vec<vk::Image>, Vec<vk::ImageView>) {
    unsafe {
        let locked_device = logical_device.clone();
        let locked_device = locked_device.lock().unwrap();

        let present_images = swapchain_device.get_swapchain_images(swapchain).unwrap();

        let views: Vec<vk::ImageView> = present_images
                .iter()
                .map(|&image| {
                    let create_view_info = vk::ImageViewCreateInfo::default()
                        .view_type(vk::ImageViewType::TYPE_2D)
                        .format(surface_format.format)
                        .components(vk::ComponentMapping {
                            r: vk::ComponentSwizzle::R,
                            g: vk::ComponentSwizzle::G,
                            b: vk::ComponentSwizzle::B,
                            a: vk::ComponentSwizzle::A,
                        })
                        .subresource_range(vk::ImageSubresourceRange {
                            aspect_mask: vk::ImageAspectFlags::COLOR,
                            base_mip_level: 0,
                            level_count: 1,
                            base_array_layer: 0,
                            layer_count: 1,
                        })
                        .image(image);
                    locked_device.create_image_view(&create_view_info, None).unwrap()
                })
                .collect();

        (present_images, views)
    }
}
