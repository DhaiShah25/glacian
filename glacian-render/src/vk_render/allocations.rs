use ash::vk;
use vk_mem::Alloc;

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

pub struct AllocatedBuffer {
    pub buf: vk::Buffer,
    pub allocation: vk_mem::Allocation,
}

impl AllocatedBuffer {
    pub fn new(
        allocator: &vk_mem::Allocator,
        size: u64,
        usage: vk::BufferUsageFlags,
        memory_usage: vk_mem::MemoryUsage,
    ) -> Self {
        let info = vk::BufferCreateInfo::default().size(size).usage(usage);
        let alloc_create_info = vk_mem::AllocationCreateInfo {
            usage: memory_usage,
            flags: vk_mem::AllocationCreateFlags::MAPPED
                | vk_mem::AllocationCreateFlags::HOST_ACCESS_SEQUENTIAL_WRITE,
            ..Default::default()
        };

        let (buf, allocation) =
            unsafe { allocator.create_buffer(&info, &alloc_create_info) }.unwrap();

        Self { buf, allocation }
    }

    pub fn flush(&mut self, allocator: &vk_mem::Allocator) {
        unsafe { allocator.destroy_buffer(self.buf, &mut self.allocation) };
    }
}
