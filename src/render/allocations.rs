use vulkanalia::vk::{self, DeviceV1_0, HasBuilder};
use vulkanalia_vma::Alloc;

#[derive(Debug)]
pub struct AllocatedImage {
    pub image: vk::Image,
    pub view: vk::ImageView,
    pub allocation: vulkanalia_vma::Allocation,
    pub extent: vk::Extent3D,
}

impl AllocatedImage {
    pub fn new(
        format: vk::Format,
        usage_flags: vk::ImageUsageFlags,
        extent: vk::Extent3D,
        aspect_flags: vk::ImageAspectFlags,
        allocator: &vulkanalia_vma::Allocator,
        device: &vulkanalia::Device,
    ) -> Self {
        let img_create_info = vk::ImageCreateInfo::builder()
            .image_type(vk::ImageType::_2D)
            .format(format)
            .extent(extent)
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlags::_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(usage_flags);

        let (image, allocation) = unsafe {
            allocator.create_image(
                img_create_info,
                &vulkanalia_vma::AllocationOptions {
                    usage: vulkanalia_vma::MemoryUsage::AutoPreferDevice,
                    required_flags: vk::MemoryPropertyFlags::DEVICE_LOCAL,
                    ..Default::default()
                },
            )
        }
        .unwrap();

        let img_view_create_info = vk::ImageViewCreateInfo::builder()
            .view_type(vk::ImageViewType::_2D)
            .image(image)
            .format(format)
            .subresource_range(
                vk::ImageSubresourceRange::builder()
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
        }
    }

    pub fn flush(&mut self, device: &vulkanalia::Device, alloc: &vulkanalia_vma::Allocator) {
        unsafe {
            alloc.destroy_image(self.image, self.allocation);
            device.destroy_image_view(self.view, None);
        }
    }
}

pub struct DescriptorAllocator {
    pool: vk::DescriptorPool,
}

impl DescriptorAllocator {
    pub fn new(
        device: &vulkanalia::Device,
        max_sets: u32,
        pool_ratios: &[(vk::DescriptorType, u32)],
    ) -> Self {
        let pool_sizes: Vec<vk::DescriptorPoolSizeBuilder> = pool_ratios
            .iter()
            .map(|ratio| {
                vk::DescriptorPoolSize::builder()
                    .descriptor_count(ratio.1 * max_sets)
                    .type_(ratio.0)
            })
            .collect();

        let pool_info = vk::DescriptorPoolCreateInfo::builder()
            .max_sets(max_sets)
            .pool_sizes(&pool_sizes);

        Self {
            pool: unsafe { device.create_descriptor_pool(&pool_info, None) }.unwrap(),
        }
    }

    pub fn clear_descriptors(&mut self, device: &vulkanalia::Device) {
        unsafe { device.reset_descriptor_pool(self.pool, vk::DescriptorPoolResetFlags::empty()) }
            .unwrap();
    }

    pub fn flush(&mut self, device: &vulkanalia::Device) {
        unsafe {
            device.destroy_descriptor_pool(self.pool, None);
        };
    }

    pub fn allocate(
        &self,
        device: &vulkanalia::Device,
        layout: vk::DescriptorSetLayout,
    ) -> vk::DescriptorSet {
        let layout = [layout];
        unsafe {
            device.allocate_descriptor_sets(
                &vk::DescriptorSetAllocateInfo::builder()
                    .descriptor_pool(self.pool)
                    .set_layouts(&layout),
            )
        }
        .unwrap()[0]
    }
}

pub struct AllocatedBuffer {
    pub buf: vk::Buffer,
    pub allocation: vulkanalia_vma::Allocation,
}

impl AllocatedBuffer {
    pub fn new(
        allocator: &vulkanalia_vma::Allocator,
        size: u64,
        usage: vk::BufferUsageFlags,
        memory_usage: vulkanalia_vma::MemoryUsage,
    ) -> Self {
        let info = vk::BufferCreateInfo::builder().size(size).usage(usage);
        let alloc_create_info = vulkanalia_vma::AllocationOptions {
            usage: memory_usage,
            flags: vulkanalia_vma::AllocationCreateFlags::MAPPED
                | vulkanalia_vma::AllocationCreateFlags::HOST_ACCESS_SEQUENTIAL_WRITE,
            ..Default::default()
        };

        let (buf, allocation) =
            unsafe { allocator.create_buffer(info, &alloc_create_info) }.unwrap();

        Self { buf, allocation }
    }

    pub fn flush(&mut self, allocator: &vulkanalia_vma::Allocator) {
        unsafe { allocator.destroy_buffer(self.buf, self.allocation) };
    }
}
