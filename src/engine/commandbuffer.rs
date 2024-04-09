#![warn(
    clippy::use_self,
    deprecated_in_future,
    trivial_casts,
    trivial_numeric_casts,
    unused_qualifications
)]

use std::{default::Default, sync::{Arc, Mutex}};
use ash::{vk, Device};

use super::vulkan_depth_image::VulkanDepthImage;

/// Helper function for submitting command buffers. Immediately waits for the fence before the command buffer
/// is executed. That way we can delay the waiting for the fences by 1 frame which is good for performance.
/// Make sure to create the fence in a signaled state on the first use.
#[allow(clippy::too_many_arguments)]
pub fn record_submit_commandbuffer<FunctionPointerType: FnOnce(&Device, vk::CommandBuffer)>(
    device: &Device,
    command_buffer: vk::CommandBuffer,
    command_buffer_reuse_fence: vk::Fence,
    submit_queue: vk::Queue,
    wait_mask: &[vk::PipelineStageFlags],
    wait_semaphores: &[vk::Semaphore],
    signal_semaphores: &[vk::Semaphore],
    func: FunctionPointerType,
) {
    unsafe {
        device
            .wait_for_fences(&[command_buffer_reuse_fence], true, u64::MAX)
            .expect("Wait for fence failed.");

        device
            .reset_fences(&[command_buffer_reuse_fence])
            .expect("Reset fences failed.");

        device
            .reset_command_buffer(
                command_buffer,
                vk::CommandBufferResetFlags::RELEASE_RESOURCES,
            )
            .expect("Reset command buffer failed.");

        let command_buffer_begin_info = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        device
            .begin_command_buffer(command_buffer, &command_buffer_begin_info)
            .expect("Begin commandbuffer");
        func(device, command_buffer);
        device
            .end_command_buffer(command_buffer)
            .expect("End commandbuffer");

        let command_buffers = vec![command_buffer];

        let submit_info = vk::SubmitInfo::default()
            .wait_semaphores(wait_semaphores)
            .wait_dst_stage_mask(wait_mask)
            .command_buffers(&command_buffers)
            .signal_semaphores(signal_semaphores);

        device
            .queue_submit(submit_queue, &[submit_info], command_buffer_reuse_fence)
            .expect("queue submit failed.");
    }
}

pub fn submit_commandbuffer_to_ensure_depth_image_format(device: Arc<Mutex<ash::Device>>, setup_command_buffer: vk::CommandBuffer, setup_commands_reuse_fence: vk::Fence, setup_command_buffer_submit_queue: vk::Queue, depth_image: &VulkanDepthImage) {
    let locked_device = device.clone();
    let locked_device = locked_device.lock().unwrap();

    record_submit_commandbuffer(
        &locked_device,
        setup_command_buffer,
        setup_commands_reuse_fence,
        setup_command_buffer_submit_queue,
        &[],
        &[],
        &[],
        |device, setup_command_buffer| {
            let layout_transition_barriers = vk::ImageMemoryBarrier::default()
                .image(depth_image.depth_image)
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

            unsafe { device.cmd_pipeline_barrier(
                setup_command_buffer,
                vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                vk::PipelineStageFlags::LATE_FRAGMENT_TESTS,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[layout_transition_barriers],
            ) };
        },
    );
}
