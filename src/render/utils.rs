use ash::vk;
use vk_mem::Alloc;

#[derive(Copy, Clone, Debug)]
pub struct FrameData {
    pub render_fence: vk::Fence,
    pub swapchain_semaphore: vk::Semaphore,
    pub render_semaphore: vk::Semaphore,
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
        let render_semaphore =
            unsafe { device.create_semaphore(&vk::SemaphoreCreateInfo::default(), None) }.unwrap();

        Self {
            render_fence,
            swapchain_semaphore,
            render_semaphore,
            pool,
            buf,
        }
    }
}

pub struct AllocatedImage {
    image: vk::Image,
    view: vk::ImageView,
    allocation: vk_mem::Allocation,
    extent: vk::Extent3D,
    format: vk::Format,
}

impl AllocatedImage {
    pub fn new(
        dimensions: (u32, u32),
        allocator: &vk_mem::Allocator,
        device: &ash::Device,
    ) -> Self {
        let extent = vk::Extent3D {
            width: dimensions.0,
            height: dimensions.1,
            depth: 1,
        };

        /* VkImageCreateInfo vkinit::image_create_info(VkFormat format, VkImageUsageFlags usageFlags, VkExtent3D extent)
        {
            VkImageCreateInfo info = {};
            info.sType = VK_STRUCTURE_TYPE_IMAGE_CREATE_INFO;
            info.pNext = nullptr;

            info.imageType = VK_IMAGE_TYPE_2D;

            info.format = format;
            info.extent = extent;

            info.mipLevels = 1;
            info.arrayLayers = 1;

            //for MSAA. we will not be using it by default, so default it to 1 sample per pixel.
            info.samples = VK_SAMPLE_COUNT_1_BIT;

            //optimal tiling, which means the image is stored on the best gpu format
            info.tiling = VK_IMAGE_TILING_OPTIMAL;
            info.usage = usageFlags;

            return info;
        }

        VkImageViewCreateInfo vkinit::imageview_create_info(VkFormat format, VkImage image, VkImageAspectFlags aspectFlags)
        {
            // build a image-view for the depth image to use for rendering
            VkImageViewCreateInfo info = {};
            info.sType = VK_STRUCTURE_TYPE_IMAGE_VIEW_CREATE_INFO;
            info.pNext = nullptr;

            info.viewType = VK_IMAGE_VIEW_TYPE_2D;
            info.image = image;
            info.format = format;
            info.subresourceRange.baseMipLevel = 0;
            info.subresourceRange.levelCount = 1;
            info.subresourceRange.baseArrayLayer = 0;
            info.subresourceRange.layerCount = 1;
            info.subresourceRange.aspectMask = aspectFlags;

            return info;
        }

                 * */
        let (image, allocation) = unsafe {
            allocator.create_image(
                &vk::ImageCreateInfo::default().format(vk::Format::R16G16B16_SFLOAT),
                &vk_mem::AllocationCreateInfo {
                    usage: vk_mem::MemoryUsage::AutoPreferDevice,
                    // required_flags: vk_mem::Memory,
                    // preferred_flags: vk_mem::,
                    // memory_type_bits: (),
                    // user_data: (),
                    // priority: (),
                    ..Default::default()
                },
            )
        }
        .unwrap();

        let view = vk::ImageView::default();

        let format = vk::Format::R16G16B16_SFLOAT;

        Self {
            image,
            view,
            allocation,
            extent,
            format,
        }
    }
}

pub struct DelQueue {
    images: Vec<AllocatedImage>,
}

impl DelQueue {
    pub fn new() -> Self {
        Self { images: vec![] }
    }

    pub fn flush(&mut self, device: &ash::Device, alloc: &vk_mem::Allocator) {
        for i in 0..self.images.len() {
            unsafe {
                device.destroy_image(self.images[i].image, None);
                alloc.destroy_image(self.images[i].image, &mut self.images[i].allocation);
            }
        }
    }
}
