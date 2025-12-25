use vulkanalia::vk::{self, DeviceV1_0, HasBuilder};

pub struct DescriptorLayoutBuilder<'a> {
    bindings: Vec<vk::DescriptorSetLayoutBindingBuilder<'a>>,
}

impl<'a> DescriptorLayoutBuilder<'a> {
    pub const fn new() -> Self {
        Self { bindings: vec![] }
    }

    pub fn add_binding(&mut self, binding: u32, descriptor_type: vk::DescriptorType) {
        self.bindings.push(
            vk::DescriptorSetLayoutBinding::builder()
                .binding(binding)
                .descriptor_count(1)
                .descriptor_type(descriptor_type),
        );
    }

    pub fn clear(&mut self) {
        self.bindings.clear();
    }

    pub fn build(
        &mut self,
        device: &vulkanalia::Device,
        shader_stages: vk::ShaderStageFlags,
        flags: vk::DescriptorSetLayoutCreateFlags,
    ) -> vk::DescriptorSetLayout {
        self.bindings
            .iter_mut()
            .for_each(|binding| binding.stage_flags |= shader_stages);

        let info = vk::DescriptorSetLayoutCreateInfo::builder()
            .bindings(&self.bindings)
            .flags(flags);

        unsafe { device.create_descriptor_set_layout(&info, None) }.unwrap()
    }
}
