use ash::vk;

pub fn make_ubo_fragment_layout_binding<'a>(quantity: u32, index: u32) -> vk::DescriptorSetLayoutBinding<'a> {
  vk::DescriptorSetLayoutBinding {
    descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
    descriptor_count: quantity,
    // binding: index, // ???
    stage_flags: vk::ShaderStageFlags::FRAGMENT,
    ..Default::default()
  }
}

pub fn make_image_sampler_fragment_layout_binding<'a>(quantity: u32, index: u32) -> vk::DescriptorSetLayoutBinding<'a> {
    vk::DescriptorSetLayoutBinding {
        descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
        descriptor_count: quantity,
        binding: index,
        stage_flags: vk::ShaderStageFlags::FRAGMENT,
        ..Default::default()
    }
}
