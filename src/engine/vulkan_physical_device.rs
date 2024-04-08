use ash::{
    ext::debug_utils,
    khr::{surface, swapchain},
    vk::{self, SurfaceKHR}, Device, Entry, Instance,
};

pub unsafe fn get_physical_device_and_family_that_support(instance: &Instance, surface_loader: &surface::Instance, surface: SurfaceKHR) -> (vk::PhysicalDevice, u32) {
  let pdevices = instance
      .enumerate_physical_devices()
      .expect("Physical device error");
  let (pdevice, queue_family_index) = pdevices
    .iter()
    .find_map(|pdevice| {
        instance
            .get_physical_device_queue_family_properties(*pdevice)
            .iter()
            .enumerate()
            .find_map(|(index, info)| {
                let supports_graphic_and_surface =
                    info.queue_flags.contains(vk::QueueFlags::GRAPHICS)
                        && surface_loader
                            .get_physical_device_surface_support(
                                *pdevice,
                                index as u32,
                                surface,
                            )
                            .unwrap();
                if supports_graphic_and_surface {
                    Some((*pdevice, index))
                } else {
                    None
                }
            })
    })
    .expect("Couldn't find suitable device.");

    (pdevice, queue_family_index as u32)
}

pub fn get_mailbox_or_fifo_present_mode(physical_device: &vk::PhysicalDevice, surface_loader: &surface::Instance, surface: SurfaceKHR) -> vk::PresentModeKHR {
    let present_modes = unsafe { surface_loader
        .get_physical_device_surface_present_modes(*physical_device, surface)
        .unwrap() };

    present_modes
        .iter()
        .cloned()
        .find(|&mode| mode == vk::PresentModeKHR::MAILBOX)
        .unwrap_or(vk::PresentModeKHR::FIFO)
}
