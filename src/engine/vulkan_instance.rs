#![warn(
    clippy::use_self,
    deprecated_in_future,
    trivial_casts,
    trivial_numeric_casts,
    unused_qualifications
)]

use std::{
    default::Default, error::Error, ffi, os::raw::c_char, sync::{Arc, Mutex},
};

use ash::{
    ext::debug_utils,
    vk, Entry, Instance,
};
use winit::{
    raw_window_handle::HasDisplayHandle,
    window::Window,
};

pub unsafe fn make_vulkan_instance(app_name: &str, entry: &Entry, window: Arc<Mutex<Window>>) -> Result<Instance, Box<dyn Error>> {
    let app_name = ffi::CString::new(app_name).unwrap();

    let layer_names = [ffi::CStr::from_bytes_with_nul_unchecked(
        b"VK_LAYER_KHRONOS_validation\0",
    )];
    let layers_names_raw: Vec<*const c_char> = layer_names
        .iter()
        .map(|raw_name| raw_name.as_ptr())
        .collect();

    let mut extension_names =
        ash_window::enumerate_required_extensions(window.lock().unwrap().display_handle()?.as_raw())
            .unwrap()
            .to_vec();
    extension_names.push(debug_utils::NAME.as_ptr());

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    {
        extension_names.push(ash::khr::portability_enumeration::NAME.as_ptr());
        extension_names.push(ash::khr::get_physical_device_properties2::NAME.as_ptr());
    }

    let appinfo = vk::ApplicationInfo::default()
        .application_name(app_name.as_c_str())
        .application_version(0)
        .engine_name(app_name.as_c_str())
        .engine_version(0)
        .api_version(vk::make_api_version(0, 1, 0, 0));

    let create_flags = if cfg!(any(target_os = "macos", target_os = "ios")) {
        vk::InstanceCreateFlags::ENUMERATE_PORTABILITY_KHR
    } else {
        vk::InstanceCreateFlags::default()
    };

    let create_info = vk::InstanceCreateInfo::default()
        .application_info(&appinfo)
        .enabled_layer_names(&layers_names_raw)
        .enabled_extension_names(&extension_names)
        .flags(create_flags);

    let instance: Instance = entry
        .create_instance(&create_info, None)
        .expect("Instance creation error");

    Ok(instance)
}
