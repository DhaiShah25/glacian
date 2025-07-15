use ash::vk;
use vk_mem::Alloc;

#[derive(Copy, Clone, Debug)]
pub struct FrameData {
    pub render_fence: vk::Fence,
    pub swapchain_semaphore: vk::Semaphore,
    pub pool: vk::CommandPool,
    pub buf: vk::CommandBuffer,
}

impl FrameData {
    pub fn new(device: &ash::Device) -> Self {
        let pool = unsafe {
            device.create_command_pool(
                &vk::CommandPoolCreateInfo::default()
                    .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER),
                None,
            )
        }
        .unwrap();
        let cmd_bufs = unsafe {
            device.allocate_command_buffers(
                &vk::CommandBufferAllocateInfo::default()
                    .command_pool(pool)
                    .level(vk::CommandBufferLevel::PRIMARY)
                    .command_buffer_count(1),
            )
        }
        .unwrap();

        let buf = cmd_bufs
            .first()
            .map_or_else(|| panic!("Unable to allocate command buffers"), |s| *s);

        let render_fence = unsafe {
            device.create_fence(
                &vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED),
                None,
            )
        }
        .unwrap();
        let swapchain_semaphore =
            unsafe { device.create_semaphore(&vk::SemaphoreCreateInfo::default(), None) }.unwrap();

        Self {
            render_fence,
            swapchain_semaphore,
            pool,
            buf,
        }
    }
}

#[derive(Debug)]
pub struct AllocatedImage {
    pub image: vk::Image,
    pub view: vk::ImageView,
    pub allocation: vk_mem::Allocation,
    pub extent: vk::Extent3D,
    pub format: vk::Format,
}

impl AllocatedImage {
    pub fn new(
        format: vk::Format,
        usage_flags: vk::ImageUsageFlags,
        extent: vk::Extent3D,
        aspect_flags: vk::ImageAspectFlags,
        allocator: &vk_mem::Allocator,
        device: &ash::Device,
    ) -> Self {
        let img_create_info = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .format(format)
            .extent(extent)
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(usage_flags);

        let (image, allocation) = unsafe {
            allocator.create_image(
                &img_create_info,
                &vk_mem::AllocationCreateInfo {
                    usage: vk_mem::MemoryUsage::AutoPreferDevice,
                    required_flags: vk::MemoryPropertyFlags::DEVICE_LOCAL,
                    ..Default::default()
                },
            )
        }
        .unwrap();

        let img_view_create_info = vk::ImageViewCreateInfo::default()
            .view_type(vk::ImageViewType::TYPE_2D)
            .image(image)
            .format(format)
            .subresource_range(
                vk::ImageSubresourceRange::default()
                    .base_mip_level(0)
                    .level_count(1)
                    .base_array_layer(0)
                    .layer_count(1)
                    .aspect_mask(aspect_flags),
            );

        let view = unsafe { device.create_image_view(&img_view_create_info, None) }.unwrap();

        Self {
            image,
            view,
            allocation,
            extent,
            format,
        }
    }

    pub fn flush(&mut self, device: &ash::Device, alloc: &vk_mem::Allocator) {
        unsafe {
            alloc.destroy_image(self.image, &mut self.allocation);
            device.destroy_image_view(self.view, None);
        }
    }
}

pub struct DelQueue<'a> {
    deletors: std::collections::VecDeque<Box<dyn FnOnce() + 'a>>,
}

impl<'a> DelQueue<'a> {
    pub const fn new() -> Self {
        Self {
            deletors: std::collections::VecDeque::new(),
        }
    }

    pub fn add(&mut self, func: Box<dyn FnOnce() + 'a>) {
        self.deletors.push_back(func);
    }

    pub fn flush(&mut self) {
        for del in self.deletors.drain(..).rev() {
            del()
        }
    }
}

pub struct DescriptorLayoutBuilder<'a> {
    bindings: Vec<vk::DescriptorSetLayoutBinding<'a>>,
}

impl<'a> DescriptorLayoutBuilder<'a> {
    pub const fn new() -> Self {
        Self { bindings: vec![] }
    }

    pub fn add_binding(&mut self, binding: u32, descriptor_type: vk::DescriptorType) {
        self.bindings.push(
            vk::DescriptorSetLayoutBinding::default()
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
        device: &ash::Device,
        shader_stages: vk::ShaderStageFlags,
        flags: vk::DescriptorSetLayoutCreateFlags,
    ) -> vk::DescriptorSetLayout {
        self.bindings
            .iter_mut()
            .for_each(|binding| binding.stage_flags |= shader_stages);

        let info = vk::DescriptorSetLayoutCreateInfo::default()
            .bindings(&self.bindings)
            .flags(flags);

        unsafe { device.create_descriptor_set_layout(&info, None) }.unwrap()
    }
}

pub struct DescriptorAllocator {
    pool: vk::DescriptorPool,
}

impl DescriptorAllocator {
    pub fn new(
        device: &ash::Device,
        max_sets: u32,
        pool_ratios: &[(vk::DescriptorType, u32)],
    ) -> Self {
        let pool_sizes: Vec<vk::DescriptorPoolSize> = pool_ratios
            .iter()
            .map(|ratio| {
                vk::DescriptorPoolSize::default()
                    .descriptor_count(ratio.1 * max_sets)
                    .ty(ratio.0)
            })
            .collect();

        let pool_info = vk::DescriptorPoolCreateInfo::default()
            .max_sets(max_sets)
            .pool_sizes(&pool_sizes);

        Self {
            pool: unsafe { device.create_descriptor_pool(&pool_info, None) }.unwrap(),
        }
    }

    pub fn clear_descriptors(&mut self, device: &ash::Device) {
        unsafe { device.reset_descriptor_pool(self.pool, vk::DescriptorPoolResetFlags::empty()) }
            .unwrap();
    }

    pub fn flush(&mut self, device: &ash::Device) {
        unsafe {
            device.destroy_descriptor_pool(self.pool, None);
        };
    }

    pub fn allocate(
        &self,
        device: &ash::Device,
        layout: vk::DescriptorSetLayout,
    ) -> vk::DescriptorSet {
        let layout = [layout];
        unsafe {
            device.allocate_descriptor_sets(
                &vk::DescriptorSetAllocateInfo::default()
                    .descriptor_pool(self.pool)
                    .set_layouts(&layout),
            )
        }
        .unwrap()[0]
    }
}
