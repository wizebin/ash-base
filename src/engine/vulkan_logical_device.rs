use std::sync::{Arc, Mutex};

use ash::{
    ext::debug_utils,
    khr::{surface, swapchain},
    vk, Device, Entry, Instance,
};

pub unsafe fn make_logical_device(instance: &Instance, pdevice: vk::PhysicalDevice, queue_family_index: u32) -> Arc<Mutex<Device>> {
  let device_extension_names_raw = [
      swapchain::NAME.as_ptr(),
      #[cfg(any(target_os = "macos", target_os = "ios"))]
      ash::khr::portability_subset::NAME.as_ptr(),
  ];
  let features = vk::PhysicalDeviceFeatures {
      shader_clip_distance: 1,
      ..Default::default()
  };
  let priorities = [1.0];

  let queue_info = vk::DeviceQueueCreateInfo::default()
      .queue_family_index(queue_family_index)
      .queue_priorities(&priorities);

  let device_create_info = vk::DeviceCreateInfo::default()
      .queue_create_infos(std::slice::from_ref(&queue_info))
      .enabled_extension_names(&device_extension_names_raw)
      .enabled_features(&features);

  let device: Device = instance
      .create_device(pdevice, &device_create_info, None)
      .unwrap();

    Arc::new(Mutex::new(device))
}

pub fn make_swapchain_device(instance: &Instance, logical_device: Arc<Mutex<Device>>) -> swapchain::Device {
    let locked_device = logical_device.clone();
    let locked_device = locked_device.lock().unwrap();

    swapchain::Device::new(instance, &locked_device)
}
