#![warn(
    clippy::use_self,
    deprecated_in_future,
    rust_2018_idioms,
    trivial_casts,
    trivial_numeric_casts,
    unused_qualifications
)]

mod engine;
use engine::{coherent_quads::CoherentQuads, commandbuffer::{record_submit_commandbuffer, submit_commandbuffer_to_ensure_depth_image_format, submit_commandbuffer_to_load_image}, debugging::VulkanDebugger, image_manager::ImageManager, vec3::Vector3, vertex::Vertex, vertex_generation::make_quad_vertices, vulkan_bindings::{make_image_sampler_fragment_layout_binding, make_ubo_fragment_layout_binding}, vulkan_commands::{create_command_buffers, get_device_presentation_queue, VulkanCommandPool}, vulkan_depth_image::VulkanDepthImage, vulkan_descriptor::{make_image_sampler_pool_size, make_ubo_pool_size, update_device_descriptor_sets, VulkanDescriptorPool, VulkanDescriptorSetLayouts}, vulkan_fences::create_standard_fences, vulkan_framebuffer::VulkanFramebuffers, vulkan_image::VulkanImage, vulkan_instance::make_vulkan_instance, vulkan_logical_device::{make_logical_device, make_swapchain_device}, vulkan_physical_device::get_physical_device_and_family_that_support, vulkan_pipeline::{VulkanPipeline, VulkanPipelineLayout}, vulkan_render_pass::VulkanColorDepthRenderPass, vulkan_sampler::VulkanSampler, vulkan_semaphores::create_semaphores, vulkan_shaders::VulkanShader, vulkan_surface::VulkanSurface, vulkan_swapchain::{create_standard_swapchain, get_swapchain_image_views}, vulkan_texture::{VulkanTexture, VulkanTextureView}, vulkan_ubo::VulkanUniformBufferObject, winit_window::{get_window_resolution, make_winit_window}};

use std::{
    cell::RefCell, default::Default, error::Error, ffi, ops::Drop, sync::{Arc, Mutex}
};

use ash::{
    khr::swapchain,
    vk, Device, Entry, Instance,
};
use winit::{
    event::{ElementState, Event, KeyEvent, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::{Key, NamedKey},
    platform::run_on_demand::EventLoopExtRunOnDemand,
    window::Window,
};

use std::io::Cursor;
use std::mem;

pub fn render_loop<F: FnMut(bool)>(event_loop: RefCell<EventLoop<()>>, mut render: F) -> Result<(), impl Error> {
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
            Event::WindowEvent {
                event: WindowEvent::Resized(_),
                ..
            } => {
                render(true);
            }
            Event::AboutToWait => render(false),
            _ => (),
        }
    })
}

pub struct PipelineData {
    pub texture: VulkanTexture,
    pub sampler: VulkanSampler,
    pub texture_view: VulkanTextureView,
    pub descriptor_pool: VulkanDescriptorPool,
    pub descriptor_set_layouts: VulkanDescriptorSetLayouts,
    pub vertex_shader: VulkanShader,
    pub fragment_shader: VulkanShader,
    pub pipeline_layout: VulkanPipelineLayout,
    pub viewports: [vk::Viewport; 1],
    pub scissors: [vk::Rect2D; 1],
    pub graphics_pipelines: VulkanPipeline, // must be last for automatic drop to be last, https://github.com/rust-lang/rfcs/blob/246ff86b320a72f98ed2df92805e8e3d48b402d6/text/1857-stabilize-drop-order.md
}

pub struct PipelineExtras {
    vertex_bytes: Vec<u8>,
    frag_bytes: Vec<u8>,
    raw_ubo_data: Vec<Vector3>,
}

pub struct ExampleBase {
    pub entry: Entry,
    pub instance: Instance,
    pub device: Arc<Mutex<Device>>,
    pub swapchain_device: swapchain::Device,
    pub window: Arc<Mutex<Window>>,
    pub depth_image: Option<VulkanDepthImage>,
    pub debugger: Option<VulkanDebugger>,
    pub command_pool: Option<VulkanCommandPool>,
    pub surface: Option<VulkanSurface>,
    pub renderpass: Option<VulkanColorDepthRenderPass>,
    pub framebuffers: Option<VulkanFramebuffers>,

    pub pdevice: vk::PhysicalDevice,
    pub device_memory_properties: vk::PhysicalDeviceMemoryProperties,
    pub queue_family_index: u32,
    pub present_queue: vk::Queue,

    pub surface_format: vk::SurfaceFormatKHR,
    pub surface_resolution: vk::Extent2D,

    pub swapchain: vk::SwapchainKHR,
    pub present_images: Vec<vk::Image>,
    pub present_image_views: Vec<vk::ImageView>,

    pub draw_command_buffer: vk::CommandBuffer,
    pub setup_command_buffer: vk::CommandBuffer,

    pub present_complete_semaphores: Vec<vk::Semaphore>,
    pub rendering_complete_semaphores: Vec<vk::Semaphore>,

    pub draw_commands_reuse_fence: vk::Fence,
    pub setup_commands_reuse_fence: vk::Fence,

    pub pipeline_data: Option<PipelineData>,
    pub pipeline_extras: Option<PipelineExtras>,

    pub current_swapchain_image: RefCell<usize>,
    pub frame: RefCell<usize>,
    pub image_manager: ImageManager,
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

            let instance = make_vulkan_instance(title.as_str(), &entry, window.clone())?;

            let debugger = VulkanDebugger::new(&entry, &instance);
            let surf = VulkanSurface::new(&entry, &instance, window.clone());
            let (pdevice, queue_family_index) = get_physical_device_and_family_that_support(&instance, &surf.surface_loader, surf.surface);
            let device = make_logical_device(&instance, pdevice, queue_family_index);
            let present_queue = get_device_presentation_queue(device.clone(), queue_family_index);
            let surface_format = surf.get_format(&pdevice);
            let surface_resolution = surf.get_resolution(get_window_resolution(window.clone()), &pdevice);
            let swapchain_device = make_swapchain_device(&instance, device.clone());
            let swapchain = create_standard_swapchain(&pdevice, &surf, surface_format, surface_resolution, &swapchain_device);
            let (present_images, present_image_views) = get_swapchain_image_views(device.clone(), &swapchain_device, swapchain, surface_format);

            let command_pool = VulkanCommandPool::new(device.clone(), queue_family_index);
            let (setup_command_buffer, draw_command_buffer) = create_command_buffers(&command_pool, device.clone());
            let device_memory_properties = instance.get_physical_device_memory_properties(pdevice);
            let depth_img = VulkanDepthImage::new(surface_resolution, device.clone(), device_memory_properties);

            let fences = create_standard_fences(device.clone(), 2);
            let (draw_commands_reuse_fence, setup_commands_reuse_fence) = (fences[0], fences[1]);


            submit_commandbuffer_to_ensure_depth_image_format(
                device.clone(),
                setup_command_buffer,
                setup_commands_reuse_fence,
                present_queue,
                &depth_img,
            );

            let present_complete_semaphores = create_semaphores(device.clone(), present_images.len());
            let rendering_complete_semaphores = create_semaphores(device.clone(), present_images.len());

            let renderpass = VulkanColorDepthRenderPass::new(device.clone(), surface_format.format);

            let framebuffers = VulkanFramebuffers::new(
                device.clone(),
                surface_resolution,
                &renderpass,
                &depth_img,
                &present_image_views,
            );

            Ok(Self {
                entry,
                instance,
                device,
                queue_family_index,
                pdevice,
                device_memory_properties,
                window,
                surface_format,
                present_queue,
                surface_resolution,
                swapchain_device,
                swapchain,
                present_images,
                present_image_views,
                draw_command_buffer,
                setup_command_buffer,
                present_complete_semaphores,
                rendering_complete_semaphores,
                draw_commands_reuse_fence,
                setup_commands_reuse_fence,
                current_swapchain_image: RefCell::new(0),
                frame: RefCell::new(0),
                depth_image: Some(depth_img),
                debugger: Some(debugger),
                command_pool: Some(command_pool),
                surface: Some(surf),
                renderpass: Some(renderpass),
                framebuffers: Some(framebuffers),
                pipeline_data: None,
                pipeline_extras: None,
                image_manager: ImageManager::new(),
            })
        }
    }

    pub fn add_image(&mut self, name: &'static str, image: VulkanImage) {
        self.image_manager.add_image(name, image);
    }

    pub unsafe fn create_pipeline(&mut self, vertex_bytes: Vec<u8>, frag_bytes: Vec<u8>, ubo: Vec<Vector3>) {
        self.pipeline_extras = Some(PipelineExtras {
            vertex_bytes: vertex_bytes.clone(),
            frag_bytes: frag_bytes.clone(),
            raw_ubo_data: ubo.clone(),
        });

        let ubo = VulkanUniformBufferObject::new_from_vec3(ubo[0], self.shared_device(), self.device_memory_properties);
        let img = self.image_manager.get_image("rust.png");
        let images = vec![img];

        let tex = VulkanTexture::new_from_image(&images[0], self.device.clone(), self.device_memory_properties);
        submit_commandbuffer_to_load_image(self.device.clone(), self.setup_command_buffer, self.setup_commands_reuse_fence, self.present_queue, &tex, &images[0]);

        let samplr = VulkanSampler::new(self.device.clone());
        let texview = VulkanTextureView::new(self.device.clone(), &tex);

        let descriptor_sizes = vec![make_ubo_pool_size(1), make_image_sampler_pool_size(1)];
        let desc_layout_bindings = vec![
            make_ubo_fragment_layout_binding(1, 1),
            make_image_sampler_fragment_layout_binding(1, 1),
        ];

        let mut descriptor_pool = VulkanDescriptorPool::new(self.device.clone(), descriptor_sizes);

        let descriptor_set_layouts = VulkanDescriptorSetLayouts::new(self.device.clone(), desc_layout_bindings);
        descriptor_pool.create_source_descriptor_sets_releasing_old(&descriptor_set_layouts);

        let uniform_color_buffer_descriptor = ubo.get_descriptor_info(0);

        let tex_descriptor = texview.get_descriptor_info(&samplr);

        let write_desc_sets = vec![
            vk::WriteDescriptorSet {
                dst_set: descriptor_pool.source_descriptor_sets[0],
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                p_buffer_info: &uniform_color_buffer_descriptor,
                ..Default::default()
            },
            vk::WriteDescriptorSet {
                dst_set: descriptor_pool.source_descriptor_sets[0],
                dst_binding: 1,
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                p_image_info: &tex_descriptor,
                ..Default::default()
            },
        ];

        update_device_descriptor_sets(self.device.clone(), &write_desc_sets);

        let vertex_spv_file = Cursor::new(vertex_bytes.as_slice());
        let frag_spv_file = Cursor::new(frag_bytes.as_slice());
        let vertex_shader = VulkanShader::new(self.device.clone(), vertex_spv_file);
        let fragment_shader = VulkanShader::new(self.device.clone(), frag_spv_file);

        let pipeline_layout = VulkanPipelineLayout::new(self.device.clone(), &descriptor_set_layouts);

        let shader_entry_name = ffi::CString::new("main").unwrap();
        let shader_entry_name = shader_entry_name.as_c_str();
        let shader_stage_create_infos = [
            vk::PipelineShaderStageCreateInfo {
                module: vertex_shader.shader_module,
                p_name: shader_entry_name.as_ptr(),
                stage: vk::ShaderStageFlags::VERTEX,
                ..Default::default()
            },
            vk::PipelineShaderStageCreateInfo {
                module: fragment_shader.shader_module,
                p_name: shader_entry_name.as_ptr(),
                stage: vk::ShaderStageFlags::FRAGMENT,
                ..Default::default()
            },
        ];

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
            width: self.surface_resolution.width as f32,
            height: self.surface_resolution.height as f32,
            min_depth: 0.0,
            max_depth: 1.0,
        }];
        let scissors = [self.surface_resolution.into()];
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
            .layout(pipeline_layout.pipeline_layout)
            .render_pass(self.renderpass.as_ref().unwrap().render_pass);

        let graphics_pipelines = VulkanPipeline::new(self.device.clone(), graphic_pipeline_infos);

        self.pipeline_data = Some(PipelineData {
            graphics_pipelines,
            pipeline_layout,
            vertex_shader,
            fragment_shader,
            descriptor_pool,
            descriptor_set_layouts,
            sampler: samplr,
            texture: tex,
            texture_view: texview,
            viewports,
            scissors,
        });
    }

    pub fn recreate_pipeline(&mut self, ) {
        if self.pipeline_data.is_none() {
            return;
        }

        self.pipeline_data = None;
        unsafe {
            self.create_pipeline(
                self.pipeline_extras.as_ref().unwrap().vertex_bytes.clone(),
                self.pipeline_extras.as_ref().unwrap().frag_bytes.clone(),
                self.pipeline_extras.as_ref().unwrap().raw_ubo_data.clone(),
            )
        };
    }

    pub fn recreate_swapchain(&mut self, resolution: vk::Extent2D) {
        unsafe {
            self.renderpass = None;
            self.framebuffers = None;
            self.depth_image = None;
            let device = self.device.lock().unwrap();
            device.device_wait_idle().unwrap();
            for &image_view in self.present_image_views.iter() {
                device.destroy_image_view(image_view, None);
            }
            self.swapchain_device
                    .destroy_swapchain(self.swapchain, None);
        }

        self.surface_resolution = resolution;

        let swapchain = create_standard_swapchain(&self.pdevice, &self.surface.as_ref().unwrap(), self.surface_format, self.surface_resolution, &self.swapchain_device);
        let (present_images, present_image_views) = get_swapchain_image_views(self.device.clone(), &self.swapchain_device, swapchain, self.surface_format);

        self.swapchain = swapchain;
        self.present_images = present_images;
        self.present_image_views = present_image_views;

        self.renderpass = Some(VulkanColorDepthRenderPass::new(self.device.clone(), self.surface_format.format));
        self.depth_image = Some(VulkanDepthImage::new(self.surface_resolution, self.device.clone(), self.device_memory_properties));

        submit_commandbuffer_to_ensure_depth_image_format(
                self.device.clone(),
                self.setup_command_buffer,
                self.setup_commands_reuse_fence,
                self.present_queue,
                &self.depth_image.as_ref().unwrap(),
            );

        println!("Recreating framebuffers with size {:?}", self.surface_resolution);
        self.framebuffers = Some(VulkanFramebuffers::new(
            self.device.clone(),
            self.surface_resolution,
            &self.renderpass.as_ref().unwrap(),
            &self.depth_image.as_ref().unwrap(),
            &self.present_image_views,
        ));
    }
}

impl Drop for ExampleBase {
    fn drop(&mut self) {
        self.depth_image = None;
        self.framebuffers = None;
        self.renderpass = None;

        unsafe {
            {
                let device = self.device.lock().unwrap();
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
                self.swapchain_device
                    .destroy_swapchain(self.swapchain, None);
            }
            self.pipeline_data = None;
            self.command_pool = None;
            self.surface = None;
            self.image_manager.clear();
            {
                let device = self.device.lock().unwrap();
                device.destroy_device(None);
                self.debugger = None;
                self.instance.destroy_instance(None);
            }
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    unsafe {
        let app_name = "Ash Grid";

        let (event_loop, window) = make_winit_window(app_name);

        let mut base = ExampleBase::new(window.clone())?;

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

        let raw_ubo_data = vec![uniform_color_buffer_data];

        let vertex_bytes = Vec::from(include_bytes!("../shader/texture/vert.spv"));
        let frag_bytes = Vec::from(include_bytes!("../shader/texture/frag.spv"));

        base.add_image("rust.png", VulkanImage::new_from_bytes(include_bytes!("../assets/rust.png"), base.shared_device(), base.device_memory_properties));
        base.add_image("rust_2.png", VulkanImage::new_from_bytes(include_bytes!("../assets/rust_2.png"), base.shared_device(), base.device_memory_properties));
        base.create_pipeline(vertex_bytes, frag_bytes, raw_ubo_data);

        println!("finished pipeline creation");

        let _ = render_loop(event_loop, |recreate_swapchain| {
            let frame = base.increment_frame();
            if recreate_swapchain && frame > 10 {
                println!("Should recreate swapchain");
                base.shared_device().lock().unwrap().device_wait_idle().unwrap();
                let resolution = get_window_resolution(window.clone());
                println!("Recreating swapchain with resolution {:?}", resolution);
                base.recreate_swapchain(resolution);
                base.recreate_pipeline();
                return;
            }
            let current_swapchain_image = base.get_next_swapchain_image_index();

            for quad_id in 0..quads.quad_quantity() {
                let distance_from_zero = (frame as f32 / ((quad_id as f32 + 1.0) * 43.0)).sin() / 2.0 + 0.5;
                let y_position = (frame as f32 / 43.0).sin() / 2.0 - 1.0;
                let rotation = (frame as f32 / 43.0).cos() / 2.0;

                quads.modify_quad(quad_id, make_quad_vertices(quad_id as f32 / quads.quad_quantity() as f32 - 1.0, y_position, distance_from_zero * 2.0, distance_from_zero * 2.0, rotation));
            }

            quads.remap_data();

            let acquisition_result = base
                .swapchain_device
                .acquire_next_image(
                    base.swapchain,
                    u64::MAX,
                    base.present_complete_semaphores[current_swapchain_image],
                    vk::Fence::null(),
                );
            let present_index = match acquisition_result {
                Ok((present_index, _)) => {
                    present_index
                }
                Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                    println!("Swapchain out of date");
                    return;
                }
                Err(e) => {
                    panic!("Failed to acquire next image: {:?}", e);
                }
            };
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
                .render_pass(base.renderpass.as_ref().unwrap().render_pass)
                .framebuffer(base.framebuffers.as_ref().unwrap().framebuffers[present_index as usize])
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
                        base.pipeline_data.as_ref().unwrap().pipeline_layout.pipeline_layout,
                        0,
                        &&base.pipeline_data.as_ref().unwrap().descriptor_pool.source_descriptor_sets[..],
                        &[],
                    );
                    device.cmd_bind_pipeline(
                        draw_command_buffer,
                        vk::PipelineBindPoint::GRAPHICS,
                        base.pipeline_data.as_ref().unwrap().graphics_pipelines.pipeline,
                    );
                    device.cmd_set_viewport(draw_command_buffer, 0, &base.pipeline_data.as_ref().unwrap().viewports);
                    device.cmd_set_scissor(draw_command_buffer, 0, &base.pipeline_data.as_ref().unwrap().scissors);
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
            let presentation_result = base.swapchain_device
                .queue_present(base.present_queue, &present_info);
            match presentation_result {
                Ok(_) => {}
                Err(vk::Result::SUBOPTIMAL_KHR) => {
                    println!("Swapchain suboptimal");
                    return;
                }
                Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                    println!("Swapchain out of date");
                    return;
                }
                Err(e) => {
                    panic!("Failed to present: {:?}", e);
                }
            }
        });
        base.shared_device().lock().unwrap().device_wait_idle().unwrap();

        Ok(())
    }
}
