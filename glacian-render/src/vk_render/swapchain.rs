use ash::vk;

pub struct SwapchainData {
    pub image_views: Vec<vk::ImageView>,
    pub swapchain_device: ash::khr::swapchain::Device,
    pub swapchain: vk::SwapchainKHR,
    pub surface_loader: ash::khr::surface::Instance,
    pub surface: vk::SurfaceKHR,
    pub extent: vk::Extent2D,
    pub render_semaphores: Vec<vk::Semaphore>,
}

impl SwapchainData {
    pub fn new(
        window: &sdl3::video::Window,
        entry: &ash::Entry,
        instance: &ash::Instance,
        physical_device: &vk::PhysicalDevice,
        queuefamilies: &[u32],
        device: &ash::Device,
    ) -> Self {
        let surface = window.vulkan_create_surface(instance.handle()).unwrap();

        let surface_loader = ash::khr::surface::Instance::new(entry, instance);

        let surface_capabilities = unsafe {
            surface_loader.get_physical_device_surface_capabilities(*physical_device, surface)
        }
        .unwrap();

        let surface_formats = unsafe {
            surface_loader.get_physical_device_surface_formats(*physical_device, surface)
        };

        let image_format = &surface_formats
            .unwrap()
            .into_iter()
            .find(|format| format.format == vk::Format::B8G8R8A8_UNORM)
            .unwrap();

        let extent = vk::Extent2D {
            height: window.size().1,
            width: window.size().0,
        };

        let swapchain_create_info = vk::SwapchainCreateInfoKHR::default()
            .surface(surface)
            .min_image_count(
                3.max(surface_capabilities.min_image_count)
                    .min(surface_capabilities.max_image_count),
            )
            .image_format(image_format.format)
            .image_color_space(image_format.color_space)
            .image_extent(extent)
            .image_array_layers(1)
            .min_image_count(surface_capabilities.min_image_count)
            .image_usage(vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .queue_family_indices(queuefamilies)
            .pre_transform(surface_capabilities.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(vk::PresentModeKHR::FIFO);
        let swapchain_device = ash::khr::swapchain::Device::new(instance, device);
        let swapchain = unsafe {
            swapchain_device
                .create_swapchain(&swapchain_create_info, None)
                .unwrap()
        };

        let swapchain_images = unsafe { swapchain_device.get_swapchain_images(swapchain).unwrap() };

        let mut image_views = Vec::with_capacity(swapchain_images.len());
        let mut render_semaphores = Vec::with_capacity(swapchain_images.len());
        for image in &swapchain_images {
            let subresource_range = vk::ImageSubresourceRange::default()
                .aspect_mask(vk::ImageAspectFlags::COLOR)
                .base_mip_level(0)
                .level_count(1)
                .base_array_layer(0)
                .layer_count(1);
            let imageview_create_info = vk::ImageViewCreateInfo::default()
                .image(*image)
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(vk::Format::B8G8R8A8_UNORM)
                .subresource_range(subresource_range);
            let imageview =
                unsafe { device.create_image_view(&imageview_create_info, None) }.unwrap();
            image_views.push(imageview);
            let render_semaphore =
                unsafe { device.create_semaphore(&vk::SemaphoreCreateInfo::default(), None) }
                    .unwrap();
            render_semaphores.push(render_semaphore);
        }

        Self {
            image_views,
            swapchain_device,
            swapchain,
            surface_loader,
            surface,
            extent,
            render_semaphores,
        }
    }

    pub fn flush(&self, device: &ash::Device) {
        unsafe {
            for iv in &self.image_views {
                device.destroy_image_view(*iv, None);
            }
            for &semaphore in &self.render_semaphores {
                device.destroy_semaphore(semaphore, None);
            }
            self.swapchain_device
                .destroy_swapchain(self.swapchain, None);
            self.surface_loader.destroy_surface(self.surface, None);
        }
    }
}

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
