use ash::vk;
use thiserror::Error;
use tracing::{debug, error, info, trace, warn};

pub struct VkRenderer {
    pub entry: ash::Entry,
    pub instance: ash::Instance,
    pub device: ash::Device,
    pub physical_device: vk::PhysicalDevice,
    pub qfamindices: u32,
    queue: vk::Queue,
    #[cfg(feature = "debug")]
    debug_utils: (
        ash::ext::debug_utils::Instance,
        ash::vk::DebugUtilsMessengerEXT,
    ),
    image_views: Vec<vk::ImageView>,
    swapchain_device: ash::khr::swapchain::Device,
    swapchain: vk::SwapchainKHR,
    surface_loader: ash::khr::surface::Instance,
    surface: vk::SurfaceKHR,

    frame_data: [FrameData; 2],
    frame_count: usize,

    allocator: vk_mem::Allocator,
}

#[derive(Copy, Clone, Debug)]
struct FrameData {
    render_fence: vk::Fence,
    swapchain_semaphore: vk::Semaphore,
    render_semaphore: vk::Semaphore,
    pool: vk::CommandPool,
    buf: vk::CommandBuffer,
}

impl FrameData {
    fn new(device: &ash::Device) -> Self {
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
        let buf = match cmd_bufs.get(0) {
            Some(s) => *s,
            None => panic!("Unable to allocate command buffers"),
        };

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

#[derive(Error, Debug)]
pub enum VkCreationError {
    #[error("Vulkan was not able to be loaded properly")]
    VulkanLoaderFailed(#[from] ash::LoadingError),
}

impl VkRenderer {
    const fn get_current_framedata(&self) -> FrameData {
        self.frame_data[self.frame_count % 2]
    }

    pub fn new(window: &minifb::Window) -> Result<Self, VkCreationError>
    where
        Self: Sized,
    {
        let entry = unsafe { ash::Entry::load() }?;

        if let Some(version) = unsafe { entry.try_enumerate_instance_version() }.unwrap() {
            let major = vk::api_version_major(version);
            let minor = vk::api_version_minor(version);
            let patch = vk::api_version_patch(version);
            info!("Running Vulkan Version: {}.{}.{}", major, minor, patch);
        }

        let app_info = vk::ApplicationInfo::default()
            .api_version(vk::make_api_version(0, 1, 3, 206))
            .application_name(c"Shadow Engine");

        let mut extension_names = vec![ash::khr::surface::NAME.as_ptr()];

        #[cfg(feature = "debug")]
        extension_names.push(ash::ext::debug_utils::NAME.as_ptr());

        match std::env::consts::OS {
            "linux" => {
                extension_names.push(ash::khr::wayland_surface::NAME.as_ptr());
            }
            "macos" => {
                extension_names.extend_from_slice(&[
                    ash::khr::portability_enumeration::NAME.as_ptr(),
                    ash::khr::portability_subset::NAME.as_ptr(),
                ]);
            }
            "windows" => {
                extension_names.push(ash::khr::win32_surface::NAME.as_ptr());
            }
            _ => {}
        }

        #[cfg(feature = "debug")]
        let mut debugcreateinfo = vk::DebugUtilsMessengerCreateInfoEXT::default()
            .message_severity(
                vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                    | vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE
                    | vk::DebugUtilsMessageSeverityFlagsEXT::INFO
                    | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
            )
            .message_type(
                vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                    | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE
                    | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION,
            )
            .pfn_user_callback(Some(debug_callback));

        let create_info = vk::InstanceCreateInfo::default()
            .application_info(&app_info)
            .enabled_extension_names(&extension_names);

        #[cfg(feature = "debug")]
        let layers = vec![c"VK_LAYER_KHRONOS_validation".as_ptr()];

        #[cfg(feature = "debug")]
        let create_info = create_info
            .enabled_layer_names(&layers)
            .push_next(&mut debugcreateinfo);

        let instance = unsafe { entry.create_instance(&create_info, None) }.unwrap();

        #[cfg(feature = "debug")]
        let debug_utils = {
            let debug_utils = ash::ext::debug_utils::Instance::new(&entry, &instance);
            let utils_messenger = unsafe {
                debug_utils
                    .create_debug_utils_messenger(&debugcreateinfo, None)
                    .unwrap()
            };
            (debug_utils, utils_messenger)
        };

        let devices = unsafe { instance.enumerate_physical_devices() }.unwrap();
        let physical_device = *devices
            .iter()
            .find(|device| {
                let props = unsafe { instance.get_physical_device_properties(**device) };
                props.device_type == vk::PhysicalDeviceType::DISCRETE_GPU
                    && props.api_version >= vk::make_api_version(0, 1, 3, 0)
            })
            .unwrap_or_else(|| {
                devices
                    .iter()
                    .find(|device| {
                        let props = unsafe { instance.get_physical_device_properties(**device) };
                        props.device_type == vk::PhysicalDeviceType::INTEGRATED_GPU
                            && props.api_version >= vk::make_api_version(0, 1, 3, 0)
                    })
                    .unwrap()
            });

        let mut device_extensions = vec![ash::khr::swapchain::NAME.as_ptr()];

        #[cfg(feature = "debug")]
        device_extensions.push(ash::khr::line_rasterization::NAME.as_ptr());

        let queuefamilyproperties =
            unsafe { instance.get_physical_device_queue_family_properties(physical_device) };

        let qfamindices = {
            let mut found_graphics_q_index = None;
            for (index, qfam) in queuefamilyproperties.iter().enumerate() {
                if qfam.queue_count > 0 && qfam.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
                    found_graphics_q_index = Some(index as u32);
                }
            }
            found_graphics_q_index.unwrap()
        };

        let queue_create_info = &[vk::DeviceQueueCreateInfo::default()
            .queue_priorities(&[1.])
            .queue_family_index(qfamindices)];

        let device = unsafe {
            instance
                .create_device(
                    physical_device,
                    &vk::DeviceCreateInfo::default()
                        .enabled_extension_names(&device_extensions)
                        .queue_create_infos(queue_create_info)
                        .push_next(
                            &mut vk::PhysicalDeviceSynchronization2Features::default()
                                .synchronization2(true),
                        ),
                    None,
                )
                .unwrap()
        };

        let queue = unsafe { device.get_device_queue(qfamindices, 0) };

        let (image_views, swapchain_device, swapchain, surface_loader, surface) =
            Self::create_swapchain(
                window,
                &entry,
                &instance,
                &physical_device,
                &[qfamindices],
                &device,
            );

        let frame_data = [FrameData::new(&device), FrameData::new(&device)];

        let mut alloc_create_info =
            vk_mem::AllocatorCreateInfo::new(&instance, &device, physical_device);
        alloc_create_info.flags = vk_mem::AllocatorCreateFlags::BUFFER_DEVICE_ADDRESS;

        let allocator = unsafe { vk_mem::Allocator::new(alloc_create_info) }.unwrap();

        Ok(Self {
            entry,
            instance,
            device,
            physical_device,
            qfamindices,
            #[cfg(feature = "debug")]
            debug_utils,
            image_views,
            swapchain,
            surface_loader,
            surface,
            swapchain_device,
            frame_data,
            frame_count: 0,
            allocator,

            queue,
        })
    }

    fn create_swapchain(
        window: &minifb::Window,
        entry: &ash::Entry,
        instance: &ash::Instance,
        physical_device: &vk::PhysicalDevice,
        queuefamilies: &[u32],
        device: &ash::Device,
    ) -> (
        Vec<vk::ImageView>,
        ash::khr::swapchain::Device,
        vk::SwapchainKHR,
        ash::khr::surface::Instance,
        vk::SurfaceKHR,
    ) {
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
                    ash::khr::wayland_surface::Instance::new(&entry, &instance);
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

                let win_surface_loader = ash::khr::win32_surface::Instance::new(&entry, &instance);
                unsafe { win_surface_loader.create_win32_surface(&surface_create_info, None) }
                    .unwrap()
            }
            _ => unreachable!("This isn't meant to be run on this operating system"),
        };

        let surface_loader = ash::khr::surface::Instance::new(&entry, &instance);

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

        let swapchain_create_info = vk::SwapchainCreateInfoKHR::default()
            .surface(surface)
            .min_image_count(
                3.max(surface_capabilities.min_image_count)
                    .min(surface_capabilities.max_image_count),
            )
            .image_format(image_format.format)
            .image_color_space(image_format.color_space)
            .image_extent(vk::Extent2D {
                height: window.get_size().1 as u32,
                width: window.get_size().0 as u32,
            })
            .image_array_layers(1)
            .min_image_count(surface_capabilities.min_image_count)
            .image_usage(vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .queue_family_indices(&queuefamilies)
            .pre_transform(surface_capabilities.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(vk::PresentModeKHR::MAILBOX);
        let swapchain_device = ash::khr::swapchain::Device::new(&instance, &device);
        let swapchain = unsafe {
            swapchain_device
                .create_swapchain(&swapchain_create_info, None)
                .unwrap()
        };

        let swapchain_images = unsafe { swapchain_device.get_swapchain_images(swapchain).unwrap() };

        let mut image_views = Vec::with_capacity(swapchain_images.len());
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
        }

        (
            image_views,
            swapchain_device,
            swapchain,
            surface_loader,
            surface,
        )
    }

    pub fn resize(&mut self, window: &minifb::Window) {
        let (image_views, swapchain_device, swapchain, surface_loader, surface) =
            Self::create_swapchain(
                window,
                &self.entry,
                &self.instance,
                &self.physical_device,
                &[self.qfamindices],
                &self.device,
            );
        unsafe {
            for iv in &self.image_views {
                self.device.destroy_image_view(*iv, None);
            }
            self.swapchain_device
                .destroy_swapchain(self.swapchain, None);
            self.surface_loader.destroy_surface(self.surface, None);
        }
        self.image_views = image_views;
        self.swapchain = swapchain;
        self.swapchain_device = swapchain_device;
        self.surface_loader = surface_loader;
        self.surface = surface;
    }

    pub fn render(&mut self, resized: bool, window_size: (f32, f32)) {
        let fence = self.get_current_framedata().render_fence;
        unsafe { self.device.wait_for_fences(&[fence], true, 1000000000) }.unwrap();
        unsafe { self.device.reset_fences(&[fence]) }.unwrap();
        let cmd_buf = self.get_current_framedata().buf;

        let swapchain_images =
            unsafe { self.swapchain_device.get_swapchain_images(self.swapchain) }.unwrap();

        let next_img = unsafe {
            self.swapchain_device
                .acquire_next_image(
                    self.swapchain,
                    1000000000,
                    self.get_current_framedata().swapchain_semaphore,
                    vk::Fence::null(),
                )
                .unwrap()
        };

        if next_img.1 {
            warn!("Suboptimal swapchain for the surface");
        }

        unsafe {
            self.device
                .reset_command_buffer(cmd_buf, vk::CommandBufferResetFlags::empty())
        }
        .unwrap();

        unsafe {
            self.device.begin_command_buffer(
                cmd_buf,
                &vk::CommandBufferBeginInfo::default()
                    .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT),
            )
        }
        .unwrap();

        if resized {
            unsafe {
                self.device.cmd_set_viewport(
                    cmd_buf,
                    0,
                    &[vk::Viewport {
                        width: window_size.0 as f32,
                        height: window_size.1 as f32,
                        ..Default::default()
                    }],
                )
            };
            unsafe {
                self.device.cmd_set_scissor(
                    cmd_buf,
                    0,
                    &[vk::Rect2D::default()
                        .extent(vk::Extent2D {
                            height: window_size.1 as u32,
                            width: window_size.0 as u32,
                        })
                        .offset(vk::Offset2D { x: 0, y: 0 })],
                )
            };
        }

        let image_barrier = vk::ImageMemoryBarrier2::default()
            .src_stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
            .src_access_mask(vk::AccessFlags2::MEMORY_WRITE)
            .dst_stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
            .dst_access_mask(vk::AccessFlags2::MEMORY_WRITE | vk::AccessFlags2::MEMORY_READ)
            .old_layout(vk::ImageLayout::UNDEFINED)
            // TODO: Change to optimize this
            .new_layout(vk::ImageLayout::GENERAL)
            .subresource_range(
                vk::ImageSubresourceRange::default()
                    .base_array_layer(0)
                    .base_mip_level(0)
                    .level_count(vk::REMAINING_MIP_LEVELS)
                    .layer_count(vk::REMAINING_ARRAY_LAYERS)
                    .aspect_mask(vk::ImageAspectFlags::COLOR),
            )
            .image(swapchain_images[next_img.0 as usize]);

        unsafe {
            self.device.cmd_pipeline_barrier2(
                cmd_buf,
                &vk::DependencyInfo::default().image_memory_barriers(&[image_barrier]),
            );
        };

        unsafe {
            self.device.cmd_clear_color_image(
                cmd_buf,
                swapchain_images[next_img.0 as usize],
                vk::ImageLayout::GENERAL,
                &vk::ClearColorValue::default(),
                &[vk::ImageSubresourceRange::default()
                    .base_array_layer(0)
                    .base_mip_level(0)
                    .level_count(vk::REMAINING_MIP_LEVELS)
                    .layer_count(vk::REMAINING_ARRAY_LAYERS)
                    .aspect_mask(vk::ImageAspectFlags::COLOR)],
            )
        };

        let image_barrier = vk::ImageMemoryBarrier2::default()
            .src_stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
            .src_access_mask(vk::AccessFlags2::MEMORY_WRITE)
            .dst_stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
            .dst_access_mask(vk::AccessFlags2::MEMORY_WRITE | vk::AccessFlags2::MEMORY_READ)
            .old_layout(vk::ImageLayout::GENERAL)
            // TODO: Change to optimize this
            .new_layout(vk::ImageLayout::PRESENT_SRC_KHR)
            .subresource_range(
                vk::ImageSubresourceRange::default()
                    .base_array_layer(0)
                    .base_mip_level(0)
                    .level_count(vk::REMAINING_MIP_LEVELS)
                    .layer_count(vk::REMAINING_ARRAY_LAYERS)
                    .aspect_mask(vk::ImageAspectFlags::COLOR),
            )
            .image(swapchain_images[next_img.0 as usize]);

        unsafe {
            self.device.cmd_pipeline_barrier2(
                cmd_buf,
                &vk::DependencyInfo::default().image_memory_barriers(&[image_barrier]),
            );
        };

        unsafe { self.device.end_command_buffer(cmd_buf) }.unwrap();
        unsafe {
            self.device.queue_submit2(
                self.queue,
                &[vk::SubmitInfo2::default()
                    .wait_semaphore_infos(&[vk::SemaphoreSubmitInfo::default()
                        .semaphore(self.get_current_framedata().swapchain_semaphore)
                        .stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)])
                    .command_buffer_infos(&[vk::CommandBufferSubmitInfo::default()
                        .command_buffer(cmd_buf)
                        .device_mask(0)])
                    .signal_semaphore_infos(&[vk::SemaphoreSubmitInfo::default()
                        .semaphore(self.get_current_framedata().render_semaphore)
                        .stage_mask(vk::PipelineStageFlags2::ALL_GRAPHICS)])],
                self.get_current_framedata().render_fence,
            )
        }
        .unwrap();

        unsafe {
            self.swapchain_device.queue_present(
                self.queue,
                &vk::PresentInfoKHR::default()
                    .swapchains(&[self.swapchain])
                    .wait_semaphores(&[self.get_current_framedata().render_semaphore])
                    .image_indices(&[next_img.0]),
            )
        }
        .unwrap();

        self.frame_count = self.frame_count.wrapping_add(1);
    }
}

impl Drop for VkRenderer {
    fn drop(&mut self) {
        unsafe {
            self.device.device_wait_idle().unwrap();
            for iv in &self.image_views {
                self.device.destroy_image_view(*iv, None);
            }
            self.swapchain_device
                .destroy_swapchain(self.swapchain, None);
            self.surface_loader.destroy_surface(self.surface, None);

            #[cfg(feature = "debug")]
            self.debug_utils
                .0
                .destroy_debug_utils_messenger(self.debug_utils.1, None);

            for i in 0..self.frame_data.len() {
                self.device
                    .destroy_fence(self.frame_data[i].render_fence, None);
                self.device
                    .destroy_semaphore(self.frame_data[i].render_semaphore, None);
                self.device
                    .destroy_semaphore(self.frame_data[i].swapchain_semaphore, None);
            }

            self.device
                .destroy_command_pool(self.frame_data[0].pool, None);
            self.device
                .destroy_command_pool(self.frame_data[1].pool, None);
            self.device.destroy_device(None);
            self.instance.destroy_instance(None);
        }
    }
}

#[cfg(feature = "debug")]
unsafe extern "system" fn debug_callback(
    severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    type_: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _p_user_data: *mut std::ffi::c_void,
) -> vk::Bool32 {
    use std::ffi::CStr;

    let message = unsafe { CStr::from_ptr((*p_callback_data).p_message) };

    if severity >= vk::DebugUtilsMessageSeverityFlagsEXT::ERROR {
        error!("({:?}) {:?}", type_, message);
    } else if severity >= vk::DebugUtilsMessageSeverityFlagsEXT::WARNING {
        warn!("({:?}) {:?}", type_, message);
    } else if severity >= vk::DebugUtilsMessageSeverityFlagsEXT::INFO {
        info!("({:?}) {:?}", type_, message);
    } else {
        trace!("({:?}) {:?}", type_, message);
    }

    vk::FALSE
}
