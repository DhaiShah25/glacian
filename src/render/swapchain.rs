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
        window: &glfw::Window,
        entry: &ash::Entry,
        instance: &ash::Instance,
        physical_device: &vk::PhysicalDevice,
        queuefamilies: &[u32],
        device: &ash::Device,
    ) -> Self {
        use raw_window_handle::{
            HasDisplayHandle, HasWindowHandle, RawDisplayHandle, RawWindowHandle,
        };
        let surface = match std::env::consts::OS {
            "linux" => {
                let surface = match window.window_handle().unwrap().as_raw() {
                    RawWindowHandle::Wayland(h) => h.surface,
                    _ => unreachable!(),
                };
                let display = match window.display_handle().unwrap().as_raw() {
                    RawDisplayHandle::Wayland(h) => h.display,
                    _ => unreachable!(),
                };
                let surface_create_info = vk::WaylandSurfaceCreateInfoKHR::default()
                    .display(display.as_ptr())
                    .surface(surface.as_ptr());

                let wayland_surface_loader =
                    ash::khr::wayland_surface::Instance::new(entry, instance);
                unsafe { wayland_surface_loader.create_wayland_surface(&surface_create_info, None) }
                    .unwrap()
            }
            "windows" => {
                let (hwnd, hinstance) = match window.window_handle().unwrap().as_raw() {
                    RawWindowHandle::Win32(h) => (h.hwnd, h.hinstance),
                    _ => unreachable!(),
                };

                let surface_create_info = vk::Win32SurfaceCreateInfoKHR::default()
                    .hinstance(hinstance.unwrap().into())
                    .hwnd(hwnd.into());

                let win_surface_loader = ash::khr::win32_surface::Instance::new(entry, instance);
                unsafe { win_surface_loader.create_win32_surface(&surface_create_info, None) }
                    .unwrap()
            }
            _ => unreachable!("This isn't meant to be run on this operating system"),
        };

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
            height: window.get_size().1 as u32,
            width: window.get_size().0 as u32,
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
