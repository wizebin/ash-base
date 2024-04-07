#![warn(
    clippy::use_self,
    deprecated_in_future,
    rust_2018_idioms,
    trivial_casts,
    trivial_numeric_casts,
    unused_qualifications
)]

mod engine;
use engine::{coherent_quads::CoherentQuads, commandbuffer::record_submit_commandbuffer, debugging::vulkan_debug_callback, memory::find_memorytype_index, vec3::Vector3, vertex::Vertex, vertex_generation::make_quad_vertices, vulkan_depth_image::VulkanDepthImage, vulkan_image::VulkanImage, vulkan_instance::make_vulkan_instance, vulkan_texture::VulkanTexture, vulkan_ubo::VulkanUniformBufferObject, winit_window::make_winit_window};

use std::{
    borrow::Cow, cell::RefCell, default::Default, error::Error, ffi, ops::Drop, os::raw::c_char, sync::{Arc, Mutex},
};

use ash::{
    ext::debug_utils,
    khr::{surface, swapchain},
    vk, Device, Entry, Instance,
};
use winit::{
    event::{ElementState, Event, KeyEvent, WindowEvent},
    event_loop::{ControlFlow, EventLoop, EventLoopBuilder},
    keyboard::{Key, NamedKey},
    platform::{macos::EventLoopBuilderExtMacOS, run_on_demand::EventLoopExtRunOnDemand},
    raw_window_handle::{HasDisplayHandle, HasWindowHandle},
    window::{Window, WindowBuilder},
};

use std::io::Cursor;
use std::mem;
use std::os::raw::c_void;

use ash::util::*;

pub fn render_loop<F: Fn()>(event_loop: RefCell<EventLoop<()>>, render: F) -> Result<(), impl Error> {
    event_loop.borrow_mut().run_on_demand(|event, elwp| {
        elwp.set_control_flow(ControlFlow::Poll);
        match event {
            Event::WindowEvent {
                event:
                    WindowEvent::CloseRequested
                    | WindowEvent::KeyboardInput {
                        event:
                            KeyEvent {
                                state: ElementState::Pressed,
                                logical_key: Key::Named(NamedKey::Escape),
                                ..
                            },
                        ..
                    },
                ..
            } => {
                elwp.exit();
            }
            Event::AboutToWait => render(),
            _ => (),
        }
    })
}

pub struct ExampleBase {
    pub entry: Entry,
    pub instance: Instance,
    pub device: Arc<Mutex<Device>>,
    pub surface_loader: surface::Instance,
    pub swapchain_loader: swapchain::Device,
    pub debug_utils_loader: debug_utils::Instance,
    pub window: Arc<Mutex<Window>>,
    pub debug_call_back: vk::DebugUtilsMessengerEXT,
    pub depth_image: Option<VulkanDepthImage>,

    pub pdevice: vk::PhysicalDevice,
    pub device_memory_properties: vk::PhysicalDeviceMemoryProperties,
    pub queue_family_index: u32,
    pub present_queue: vk::Queue,

    pub surface: vk::SurfaceKHR,
    pub surface_format: vk::SurfaceFormatKHR,
    pub surface_resolution: vk::Extent2D,

    pub swapchain: vk::SwapchainKHR,
    pub present_images: Vec<vk::Image>,
    pub present_image_views: Vec<vk::ImageView>,

    pub pool: vk::CommandPool,
    pub draw_command_buffer: vk::CommandBuffer,
    pub setup_command_buffer: vk::CommandBuffer,

    pub present_complete_semaphores: Vec<vk::Semaphore>,
    pub rendering_complete_semaphores: Vec<vk::Semaphore>,

    pub draw_commands_reuse_fence: vk::Fence,
    pub setup_commands_reuse_fence: vk::Fence,

    pub current_swapchain_image: RefCell<usize>,
    pub frame: RefCell<usize>,
}

impl ExampleBase {
    pub fn shared_device(&self) -> Arc<Mutex<Device>> {
        self.device.clone()
    }

    pub fn get_next_swapchain_image_index(&self) -> usize {
        let current_swapchain_image = *self.current_swapchain_image.borrow();
        *self.current_swapchain_image.borrow_mut() = (current_swapchain_image + 1)
            % self.present_images.len();
        current_swapchain_image
    }

    pub fn increment_frame(&self) -> usize {
        let frame = *self.frame.borrow();
        *self.frame.borrow_mut() = frame + 1;
        frame
    }

    pub fn new(window: Arc<Mutex<Window>>) -> Result<Self, Box<dyn Error>> {
        unsafe {

            let entry = Entry::linked();
            let title = {
                let locked_window = window.clone();
                let locked_window = locked_window.lock().unwrap();
                locked_window.title()
            };

            // temporary until we move instance creation out of example base, at which point we should set the name separate from the window title
            let instance = make_vulkan_instance(title.as_str(), &entry, window.clone());
            let instance = match instance {
                Ok(instance) => instance,
                Err(err) => {
                    eprintln!("Failed to create instance: {}", err);
                    return Err(err);
                }
            };

            let locked_window = window.clone();
            let locked_window = locked_window.lock().unwrap();

            let debug_info = vk::DebugUtilsMessengerCreateInfoEXT::default()
                .message_severity(
                    vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                        | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                        | vk::DebugUtilsMessageSeverityFlagsEXT::INFO,
                )
                .message_type(
                    vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                        | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                        | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
                )
                .pfn_user_callback(Some(vulkan_debug_callback));

            let debug_utils_loader = debug_utils::Instance::new(&entry, &instance);
            let debug_call_back = debug_utils_loader
                .create_debug_utils_messenger(&debug_info, None)
                .unwrap();
            let surface = ash_window::create_surface(
                &entry,
                &instance,
                locked_window.display_handle()?.as_raw(),
                locked_window.window_handle()?.as_raw(),
                None,
            )
            .unwrap();
            let pdevices = instance
                .enumerate_physical_devices()
                .expect("Physical device error");
            let surface_loader = surface::Instance::new(&entry, &instance);
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
            let queue_family_index = queue_family_index as u32;
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
            let device = Arc::new(Mutex::new(device));
            let locked_device = device.clone(); // temporary until we abstract everything else out, we need the device to be unlocked for the depth image creation
            let locked_device = locked_device.lock().unwrap(); // temporary until we abstract everything else out, we need the device to be unlocked for the depth image creation

            let present_queue = locked_device.get_device_queue(queue_family_index, 0);

            let surface_format = surface_loader
                .get_physical_device_surface_formats(pdevice, surface)
                .unwrap()[0];

            let surface_capabilities = surface_loader
                .get_physical_device_surface_capabilities(pdevice, surface)
                .unwrap();
            let mut desired_image_count = surface_capabilities.min_image_count + 1;
            if surface_capabilities.max_image_count > 0
                && desired_image_count > surface_capabilities.max_image_count
            {
                desired_image_count = surface_capabilities.max_image_count;
            }
            let surface_resolution = match surface_capabilities.current_extent.width {
                u32::MAX => vk::Extent2D {
                    width: locked_window.inner_size().width,
                    height: locked_window.inner_size().height,
                },
                _ => surface_capabilities.current_extent,
            };
            let pre_transform = if surface_capabilities
                .supported_transforms
                .contains(vk::SurfaceTransformFlagsKHR::IDENTITY)
            {
                vk::SurfaceTransformFlagsKHR::IDENTITY
            } else {
                surface_capabilities.current_transform
            };
            let present_modes = surface_loader
                .get_physical_device_surface_present_modes(pdevice, surface)
                .unwrap();
            let present_mode = present_modes
                .iter()
                .cloned()
                .find(|&mode| mode == vk::PresentModeKHR::MAILBOX)
                .unwrap_or(vk::PresentModeKHR::FIFO);
            let swapchain_loader = swapchain::Device::new(&instance, &locked_device);

            let swapchain_create_info = vk::SwapchainCreateInfoKHR::default()
                .surface(surface)
                .min_image_count(desired_image_count)
                .image_color_space(surface_format.color_space)
                .image_format(surface_format.format)
                .image_extent(surface_resolution)
                .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
                .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
                .pre_transform(pre_transform)
                .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
                .present_mode(present_mode)
                .clipped(true)
                .image_array_layers(1);

            let swapchain = swapchain_loader
                .create_swapchain(&swapchain_create_info, None)
                .unwrap();

            let pool_create_info = vk::CommandPoolCreateInfo::default()
                .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
                .queue_family_index(queue_family_index);

            let pool = locked_device.create_command_pool(&pool_create_info, None).unwrap();

            let command_buffer_allocate_info = vk::CommandBufferAllocateInfo::default()
                .command_buffer_count(2)
                .command_pool(pool)
                .level(vk::CommandBufferLevel::PRIMARY);

            let command_buffers = locked_device
                .allocate_command_buffers(&command_buffer_allocate_info)
                .unwrap();
            let setup_command_buffer = command_buffers[0];
            let draw_command_buffer = command_buffers[1];

            let present_images = swapchain_loader.get_swapchain_images(swapchain).unwrap();
            let present_image_views: Vec<vk::ImageView> = present_images
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
            let device_memory_properties = instance.get_physical_device_memory_properties(pdevice);
            drop(locked_device); // temporary until we abstract everything else out, we need the device to be unlocked for the depth image creation
            let depth_img = VulkanDepthImage::new(surface_resolution, device.clone(), device_memory_properties);
            let locked_device = device.clone(); // temporary until we abstract everything else out, we need the device to be unlocked for the depth image creation
            let locked_device = locked_device.lock().unwrap(); // temporary until we abstract everything else out, we need the device to be unlocked for the depth image creation

            let fence_create_info =
                vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED);

            let draw_commands_reuse_fence = locked_device
                .create_fence(&fence_create_info, None)
                .expect("Create fence failed.");
            let setup_commands_reuse_fence = locked_device
                .create_fence(&fence_create_info, None)
                .expect("Create fence failed.");

            record_submit_commandbuffer(
                &locked_device,
                setup_command_buffer,
                setup_commands_reuse_fence,
                present_queue,
                &[],
                &[],
                &[],
                |device, setup_command_buffer| {
                    let layout_transition_barriers = vk::ImageMemoryBarrier::default()
                        .image(depth_img.depth_image)
                        .dst_access_mask(
                            vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ
                                | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
                        )
                        .new_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
                        .old_layout(vk::ImageLayout::UNDEFINED)
                        .subresource_range(
                            vk::ImageSubresourceRange::default()
                                .aspect_mask(vk::ImageAspectFlags::DEPTH)
                                .layer_count(1)
                                .level_count(1),
                        );

                    device.cmd_pipeline_barrier(
                        setup_command_buffer,
                        vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                        vk::PipelineStageFlags::LATE_FRAGMENT_TESTS,
                        vk::DependencyFlags::empty(),
                        &[],
                        &[],
                        &[layout_transition_barriers],
                    );
                },
            );

            let semaphore_create_info = vk::SemaphoreCreateInfo::default();

            let present_complete_semaphores: Vec<vk::Semaphore> = (0..present_images.len())
                .map(|_| locked_device
                    .create_semaphore(&semaphore_create_info, None)
                    .unwrap()
                ).collect();

            let rendering_complete_semaphores = (0..present_images.len())
                .map(|_| locked_device
                    .create_semaphore(&semaphore_create_info, None)
                    .unwrap()
                ).collect();

            drop(locked_device);

            Ok(Self {
                entry,
                instance,
                device,
                queue_family_index,
                pdevice,
                device_memory_properties,
                window,
                surface_loader,
                surface_format,
                present_queue,
                surface_resolution,
                swapchain_loader,
                swapchain,
                present_images,
                present_image_views,
                pool,
                draw_command_buffer,
                setup_command_buffer,
                present_complete_semaphores,
                rendering_complete_semaphores,
                draw_commands_reuse_fence,
                setup_commands_reuse_fence,
                surface,
                debug_call_back,
                debug_utils_loader,
                current_swapchain_image: RefCell::new(0),
                frame: RefCell::new(0),
                depth_image: Some(depth_img),
            })
        }
    }
}

impl Drop for ExampleBase {
    fn drop(&mut self) {
        self.depth_image = None;

        let device = self.device.lock().unwrap();
        unsafe {
            device.device_wait_idle().unwrap();

            for semaphore in self.present_complete_semaphores.iter() {
                device.destroy_semaphore(*semaphore, None);
            }

            for semaphore in self.rendering_complete_semaphores.iter() {
                device.destroy_semaphore(*semaphore, None);
            }

            device
                .destroy_fence(self.draw_commands_reuse_fence, None);
            device
                .destroy_fence(self.setup_commands_reuse_fence, None);
            for &image_view in self.present_image_views.iter() {
                device.destroy_image_view(image_view, None);
            }
            device.destroy_command_pool(self.pool, None);
            self.swapchain_loader
                .destroy_swapchain(self.swapchain, None);
            device.destroy_device(None);
            self.surface_loader.destroy_surface(self.surface, None);
            self.debug_utils_loader
                .destroy_debug_utils_messenger(self.debug_call_back, None);
            self.instance.destroy_instance(None);
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    unsafe {
        let app_name = "Ash Grid";

        let (event_loop, window) = make_winit_window(app_name);

        let base = ExampleBase::new(window.clone())?;

        let renderpass_attachments = [
            vk::AttachmentDescription {
                format: base.surface_format.format,
                samples: vk::SampleCountFlags::TYPE_1,
                load_op: vk::AttachmentLoadOp::CLEAR,
                store_op: vk::AttachmentStoreOp::STORE,
                final_layout: vk::ImageLayout::PRESENT_SRC_KHR,
                ..Default::default()
            },
            vk::AttachmentDescription {
                format: vk::Format::D16_UNORM,
                samples: vk::SampleCountFlags::TYPE_1,
                load_op: vk::AttachmentLoadOp::CLEAR,
                initial_layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
                final_layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
                ..Default::default()
            },
        ];
        let color_attachment_refs = [vk::AttachmentReference {
            attachment: 0,
            layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        }];
        let depth_attachment_ref = vk::AttachmentReference {
            attachment: 1,
            layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
        };
        let dependencies = [vk::SubpassDependency {
            src_subpass: vk::SUBPASS_EXTERNAL,
            src_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            dst_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_READ
                | vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
            dst_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            ..Default::default()
        }];

        let subpass = vk::SubpassDescription::default()
            .color_attachments(&color_attachment_refs)
            .depth_stencil_attachment(&depth_attachment_ref)
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS);

        let renderpass_create_info = vk::RenderPassCreateInfo::default()
            .attachments(&renderpass_attachments)
            .subpasses(std::slice::from_ref(&subpass))
            .dependencies(&dependencies);

        let renderpass = base
            .shared_device().lock().unwrap()
            .create_render_pass(&renderpass_create_info, None)
            .unwrap();

        let framebuffers: Vec<vk::Framebuffer> = base
            .present_image_views
            .iter()
            .map(|&present_image_view| {
                let framebuffer_attachments = [present_image_view, base.depth_image.as_ref().unwrap().depth_image_view];
                let frame_buffer_create_info = vk::FramebufferCreateInfo::default()
                    .render_pass(renderpass)
                    .attachments(&framebuffer_attachments)
                    .width(base.surface_resolution.width)
                    .height(base.surface_resolution.height)
                    .layers(1);

                base.shared_device().lock().unwrap()
                    .create_framebuffer(&frame_buffer_create_info, None)
                    .unwrap()
            })
            .collect();

        let vertices = make_quad_vertices(0.0, 0.0, 0.5, 0.5, 0.0);

        let quad_quantity = 3;
        let mut quads = CoherentQuads::new(quad_quantity, base.shared_device(), base.device_memory_properties);
        for _ in 0..quad_quantity {
            quads.add_quad(vertices.clone());
        }
        quads.remap_data();

        let uniform_color_buffer_data = Vector3 {
            x: 1.0,
            y: 1.0,
            z: 1.0,
            _pad: 0.0,
        };

        let ubo = VulkanUniformBufferObject::new_from_vec3(uniform_color_buffer_data, base.shared_device(), base.device_memory_properties);

        let img = VulkanImage::new_from_bytes(include_bytes!("../assets/rust_2.png"), base.shared_device(), base.device_memory_properties);
        let tex = VulkanTexture::new_from_image(&img, base.shared_device(), base.device_memory_properties);
        let image_extent = img.extent();

        // let img2 = VulkanImage::new_from_bytes(include_bytes!("../assets/rust_2.png"), base.shared_device(), base.device_memory_properties);
        // let tex2 = VulkanTexture::new_from_image(&img2, base.shared_device(), base.device_memory_properties);

        record_submit_commandbuffer(
            &base.shared_device().lock().unwrap(),
            base.setup_command_buffer,
            base.setup_commands_reuse_fence,
            base.present_queue,
            &[],
            &[],
            &[],
            |device, texture_command_buffer| {
                let texture_barrier = vk::ImageMemoryBarrier {
                    dst_access_mask: vk::AccessFlags::TRANSFER_WRITE,
                    new_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    image: tex.texture_image,
                    subresource_range: vk::ImageSubresourceRange {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        level_count: 1,
                        layer_count: 1,
                        ..Default::default()
                    },
                    ..Default::default()
                };
                device.cmd_pipeline_barrier(
                    texture_command_buffer,
                    vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                    vk::PipelineStageFlags::TRANSFER,
                    vk::DependencyFlags::empty(),
                    &[],
                    &[],
                    &[texture_barrier],
                );
                let buffer_copy_regions = vk::BufferImageCopy::default()
                    .image_subresource(
                        vk::ImageSubresourceLayers::default()
                            .aspect_mask(vk::ImageAspectFlags::COLOR)
                            .layer_count(1),
                    )
                    .image_extent(image_extent.into());

                device.cmd_copy_buffer_to_image(
                    texture_command_buffer,
                    img.image_buffer,
                    tex.texture_image,
                    vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    &[buffer_copy_regions],
                );
                let texture_barrier_end = vk::ImageMemoryBarrier {
                    src_access_mask: vk::AccessFlags::TRANSFER_WRITE,
                    dst_access_mask: vk::AccessFlags::SHADER_READ,
                    old_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    new_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                    image: tex.texture_image,
                    subresource_range: vk::ImageSubresourceRange {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        level_count: 1,
                        layer_count: 1,
                        ..Default::default()
                    },
                    ..Default::default()
                };
                device.cmd_pipeline_barrier(
                    texture_command_buffer,
                    vk::PipelineStageFlags::TRANSFER,
                    vk::PipelineStageFlags::FRAGMENT_SHADER,
                    vk::DependencyFlags::empty(),
                    &[],
                    &[],
                    &[texture_barrier_end],
                );
            },
        );

        let sampler_info = vk::SamplerCreateInfo {
            mag_filter: vk::Filter::LINEAR,
            min_filter: vk::Filter::LINEAR,
            mipmap_mode: vk::SamplerMipmapMode::LINEAR,
            address_mode_u: vk::SamplerAddressMode::MIRRORED_REPEAT,
            address_mode_v: vk::SamplerAddressMode::MIRRORED_REPEAT,
            address_mode_w: vk::SamplerAddressMode::MIRRORED_REPEAT,
            max_anisotropy: 1.0,
            border_color: vk::BorderColor::FLOAT_OPAQUE_WHITE,
            compare_op: vk::CompareOp::NEVER,
            ..Default::default()
        };

        let sampler = base.shared_device().lock().unwrap().create_sampler(&sampler_info, None).unwrap();

        let tex_image_view_info = vk::ImageViewCreateInfo {
            view_type: vk::ImageViewType::TYPE_2D,
            format: tex.format,
            components: vk::ComponentMapping {
                r: vk::ComponentSwizzle::R,
                g: vk::ComponentSwizzle::G,
                b: vk::ComponentSwizzle::B,
                a: vk::ComponentSwizzle::A,
            },
            subresource_range: vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                level_count: 1,
                layer_count: 1,
                ..Default::default()
            },
            image: tex.texture_image,
            ..Default::default()
        };
        let tex_image_view = base
            .shared_device().lock().unwrap()
            .create_image_view(&tex_image_view_info, None)
            .unwrap();
        let descriptor_sizes = [
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::UNIFORM_BUFFER,
                descriptor_count: 1,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                descriptor_count: 1,
            },
        ];
        let descriptor_pool_info = vk::DescriptorPoolCreateInfo::default()
            .pool_sizes(&descriptor_sizes)
            .max_sets(1);

        let descriptor_pool = base
            .shared_device().lock().unwrap()
            .create_descriptor_pool(&descriptor_pool_info, None)
            .unwrap();
        let desc_layout_bindings = [
            vk::DescriptorSetLayoutBinding {
                descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                descriptor_count: 1,
                stage_flags: vk::ShaderStageFlags::FRAGMENT,
                ..Default::default()
            },
            vk::DescriptorSetLayoutBinding {
                binding: 1,
                descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                descriptor_count: 1,
                stage_flags: vk::ShaderStageFlags::FRAGMENT,
                ..Default::default()
            },
        ];
        let descriptor_info =
            vk::DescriptorSetLayoutCreateInfo::default().bindings(&desc_layout_bindings);

        let desc_set_layouts = [base
            .shared_device().lock().unwrap()
            .create_descriptor_set_layout(&descriptor_info, None)
            .unwrap()];

        let desc_alloc_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(descriptor_pool)
            .set_layouts(&desc_set_layouts);
        let descriptor_sets = base
            .shared_device().lock().unwrap()
            .allocate_descriptor_sets(&desc_alloc_info)
            .unwrap();

        let uniform_color_buffer_descriptor = ubo.get_descriptor_info(0);

        let tex_descriptor = vk::DescriptorImageInfo {
            image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            image_view: tex_image_view,
            sampler,
        };

        let write_desc_sets = [
            vk::WriteDescriptorSet {
                dst_set: descriptor_sets[0],
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                p_buffer_info: &uniform_color_buffer_descriptor,
                ..Default::default()
            },
            vk::WriteDescriptorSet {
                dst_set: descriptor_sets[0],
                dst_binding: 1,
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                p_image_info: &tex_descriptor,
                ..Default::default()
            },
        ];
        base.shared_device().lock().unwrap().update_descriptor_sets(&write_desc_sets, &[]);

        let mut vertex_spv_file = Cursor::new(&include_bytes!("../shader/texture/vert.spv")[..]);
        let mut frag_spv_file = Cursor::new(&include_bytes!("../shader/texture/frag.spv")[..]);

        let vertex_code =
            read_spv(&mut vertex_spv_file).expect("Failed to read vertex shader spv file");
        let vertex_shader_info = vk::ShaderModuleCreateInfo::default().code(&vertex_code);

        let frag_code =
            read_spv(&mut frag_spv_file).expect("Failed to read fragment shader spv file");
        let frag_shader_info = vk::ShaderModuleCreateInfo::default().code(&frag_code);

        let vertex_shader_module = base
            .shared_device().lock().unwrap()
            .create_shader_module(&vertex_shader_info, None)
            .expect("Vertex shader module error");

        let fragment_shader_module = base
            .shared_device().lock().unwrap()
            .create_shader_module(&frag_shader_info, None)
            .expect("Fragment shader module error");

        let layout_create_info =
            vk::PipelineLayoutCreateInfo::default().set_layouts(&desc_set_layouts);

        let pipeline_layout = base
            .shared_device().lock().unwrap()
            .create_pipeline_layout(&layout_create_info, None)
            .unwrap();

        let shader_entry_name = ffi::CStr::from_bytes_with_nul_unchecked(b"main\0");
        let shader_stage_create_infos = [
            vk::PipelineShaderStageCreateInfo {
                module: vertex_shader_module,
                p_name: shader_entry_name.as_ptr(),
                stage: vk::ShaderStageFlags::VERTEX,
                ..Default::default()
            },
            vk::PipelineShaderStageCreateInfo {
                module: fragment_shader_module,
                p_name: shader_entry_name.as_ptr(),
                stage: vk::ShaderStageFlags::FRAGMENT,
                ..Default::default()
            },
        ];
        // possibly relevant for coherent quads abstraction
        let vertex_input_binding_descriptions = [vk::VertexInputBindingDescription {
            binding: 0,
            stride: mem::size_of::<Vertex>() as u32,
            input_rate: vk::VertexInputRate::VERTEX,
        }];
        let vertex_input_attribute_descriptions = Vertex::get_attribute_descriptions();
        let vertex_input_state_info = vk::PipelineVertexInputStateCreateInfo::default()
            .vertex_attribute_descriptions(&vertex_input_attribute_descriptions)
            .vertex_binding_descriptions(&vertex_input_binding_descriptions);

        let vertex_input_assembly_state_info = vk::PipelineInputAssemblyStateCreateInfo {
            topology: vk::PrimitiveTopology::TRIANGLE_LIST,
            ..Default::default()
        };

        let viewports = [vk::Viewport {
            x: 0.0,
            y: 0.0,
            width: base.surface_resolution.width as f32,
            height: base.surface_resolution.height as f32,
            min_depth: 0.0,
            max_depth: 1.0,
        }];
        let scissors = [base.surface_resolution.into()];
        let viewport_state_info = vk::PipelineViewportStateCreateInfo::default()
            .scissors(&scissors)
            .viewports(&viewports);

        let rasterization_info = vk::PipelineRasterizationStateCreateInfo {
            front_face: vk::FrontFace::COUNTER_CLOCKWISE,
            line_width: 1.0,
            polygon_mode: vk::PolygonMode::FILL,
            ..Default::default()
        };

        let multisample_state_info = vk::PipelineMultisampleStateCreateInfo::default()
            .rasterization_samples(vk::SampleCountFlags::TYPE_1);

        let noop_stencil_state = vk::StencilOpState {
            fail_op: vk::StencilOp::KEEP,
            pass_op: vk::StencilOp::KEEP,
            depth_fail_op: vk::StencilOp::KEEP,
            compare_op: vk::CompareOp::ALWAYS,
            ..Default::default()
        };
        let depth_state_info = vk::PipelineDepthStencilStateCreateInfo {
            depth_test_enable: 1,
            depth_write_enable: 1,
            depth_compare_op: vk::CompareOp::LESS_OR_EQUAL,
            front: noop_stencil_state,
            back: noop_stencil_state,
            max_depth_bounds: 1.0,
            ..Default::default()
        };

        let color_blend_attachment_states = [vk::PipelineColorBlendAttachmentState {
            blend_enable: 1,
            src_color_blend_factor: vk::BlendFactor::SRC_ALPHA,
            dst_color_blend_factor: vk::BlendFactor::ONE_MINUS_SRC_ALPHA,
            color_blend_op: vk::BlendOp::ADD,
            src_alpha_blend_factor: vk::BlendFactor::ONE,
            dst_alpha_blend_factor: vk::BlendFactor::ONE,
            alpha_blend_op: vk::BlendOp::ADD,
            color_write_mask: vk::ColorComponentFlags::RGBA,
        }];
        let color_blend_state = vk::PipelineColorBlendStateCreateInfo::default()
            .logic_op(vk::LogicOp::CLEAR)
            .attachments(&color_blend_attachment_states);

        let dynamic_state = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
        let dynamic_state_info =
            vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dynamic_state);

        let graphic_pipeline_infos = vk::GraphicsPipelineCreateInfo::default()
            .stages(&shader_stage_create_infos)
            .vertex_input_state(&vertex_input_state_info)
            .input_assembly_state(&vertex_input_assembly_state_info)
            .viewport_state(&viewport_state_info)
            .rasterization_state(&rasterization_info)
            .multisample_state(&multisample_state_info)
            .depth_stencil_state(&depth_state_info)
            .color_blend_state(&color_blend_state)
            .dynamic_state(&dynamic_state_info)
            .layout(pipeline_layout)
            .render_pass(renderpass);

        let graphics_pipelines = base
            .shared_device().lock().unwrap()
            .create_graphics_pipelines(vk::PipelineCache::null(), &[graphic_pipeline_infos], None)
            .unwrap();

        let graphic_pipeline = graphics_pipelines[0];

        let _ = render_loop(event_loop, || {
            let current_swapchain_image = base.get_next_swapchain_image_index();
            let frame = base.increment_frame();

            for quad_id in 0..quads.quad_quantity() {
                let distance_from_zero = (frame as f32 / ((quad_id as f32 + 1.0) * 43.0)).sin() / 2.0 + 0.5;
                let y_position = (frame as f32 / 43.0).sin() / 2.0 - 1.0;
                let rotation = (frame as f32 / 43.0).cos() / 2.0;

                quads.modify_quad(quad_id, make_quad_vertices(quad_id as f32 / quads.quad_quantity() as f32 - 1.0, y_position, distance_from_zero * 2.0, distance_from_zero * 2.0, rotation));
            }

            quads.remap_data();

            let (present_index, _) = base
                .swapchain_loader
                .acquire_next_image(
                    base.swapchain,
                    u64::MAX,
                    base.present_complete_semaphores[current_swapchain_image],
                    vk::Fence::null(),
                )
                .unwrap();
            let clear_values = [
                vk::ClearValue {
                    color: vk::ClearColorValue {
                        float32: [0.0, 0.0, 0.0, 0.0],
                    },
                },
                vk::ClearValue {
                    depth_stencil: vk::ClearDepthStencilValue {
                        depth: 1.0,
                        stencil: 0,
                    },
                },
            ];

            let render_pass_begin_info = vk::RenderPassBeginInfo::default()
                .render_pass(renderpass)
                .framebuffer(framebuffers[present_index as usize])
                .render_area(base.surface_resolution.into())
                .clear_values(&clear_values);

            record_submit_commandbuffer(
                &base.shared_device().lock().unwrap(),
                base.draw_command_buffer,
                base.draw_commands_reuse_fence,
                base.present_queue,
                &[vk::PipelineStageFlags::BOTTOM_OF_PIPE],
                &[base.present_complete_semaphores[current_swapchain_image]],
                &[base.rendering_complete_semaphores[current_swapchain_image]],
                |device, draw_command_buffer| {
                    device.cmd_begin_render_pass(
                        draw_command_buffer,
                        &render_pass_begin_info,
                        vk::SubpassContents::INLINE,
                    );
                    device.cmd_bind_descriptor_sets(
                        draw_command_buffer,
                        vk::PipelineBindPoint::GRAPHICS,
                        pipeline_layout,
                        0,
                        &descriptor_sets[..],
                        &[],
                    );
                    device.cmd_bind_pipeline(
                        draw_command_buffer,
                        vk::PipelineBindPoint::GRAPHICS,
                        graphic_pipeline,
                    );
                    device.cmd_set_viewport(draw_command_buffer, 0, &viewports);
                    device.cmd_set_scissor(draw_command_buffer, 0, &scissors);
                    device.cmd_bind_vertex_buffers(
                        draw_command_buffer,
                        0,
                        &[quads.device_vertex_buffer],
                        &[0],
                    );
                    device.cmd_bind_index_buffer(
                        draw_command_buffer,
                        quads.device_index_buffer,
                        0,
                        vk::IndexType::UINT32,
                    );
                    device.cmd_draw_indexed(
                        draw_command_buffer,
                        quads.index_quantity() as u32,
                        quads.quad_quantity() as u32,
                        0,
                        0,
                        1,
                    );
                    // Or draw without the index buffer
                    // device.cmd_draw(draw_command_buffer, 3, 1, 0, 0);
                    device.cmd_end_render_pass(draw_command_buffer);
                },
            );
            let present_info = vk::PresentInfoKHR {
                wait_semaphore_count: 1,
                p_wait_semaphores: &base.rendering_complete_semaphores[current_swapchain_image],
                swapchain_count: 1,
                p_swapchains: &base.swapchain,
                p_image_indices: &present_index,
                ..Default::default()
            };
            base.swapchain_loader
                .queue_present(base.present_queue, &present_info)
                .unwrap();
        });
        base.shared_device().lock().unwrap().device_wait_idle().unwrap();

        for pipeline in graphics_pipelines {
            base.shared_device().lock().unwrap().destroy_pipeline(pipeline, None);
        }
        base.shared_device().lock().unwrap().destroy_pipeline_layout(pipeline_layout, None);
        base.shared_device().lock().unwrap()
            .destroy_shader_module(vertex_shader_module, None);
        base.shared_device().lock().unwrap()
            .destroy_shader_module(fragment_shader_module, None);
        base.shared_device().lock().unwrap().destroy_image_view(tex_image_view, None);

        // todo: consider dropping explicitly resources like the depth image here to avoid segfault, OR create the device and instance in an outer scope to guarantee proper destruction order
        drop(quads);

        for &descriptor_set_layout in desc_set_layouts.iter() {
            base.shared_device().lock().unwrap()
                .destroy_descriptor_set_layout(descriptor_set_layout, None);
        }
        base.shared_device().lock().unwrap().destroy_descriptor_pool(descriptor_pool, None);
        base.shared_device().lock().unwrap().destroy_sampler(sampler, None);
        for framebuffer in framebuffers {
            base.shared_device().lock().unwrap().destroy_framebuffer(framebuffer, None);
        }
        base.shared_device().lock().unwrap().destroy_render_pass(renderpass, None);

        Ok(())
    }
}
