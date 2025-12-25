use std::ffi::CString;

use vulkanalia::vk::{
    self, ExtDebugUtilsExtensionInstanceCommands, Handle, HasBuilder,
    KhrSwapchainExtensionDeviceCommands,
};
use vulkanalia::vk::{DeviceV1_0, DeviceV1_3, InstanceV1_0};
mod allocations;
use allocations::{AllocatedImage, DescriptorAllocator};
mod swapchain;
use swapchain::{FrameData, SwapchainData};
mod mesh_buffer;

mod utils;
use utils::{copy_image_to_image, transition_image};

mod descriptor;
use descriptor::DescriptorLayoutBuilder;

use piglog::prelude::*;
use piglog::warning;

mod debug;

mod skybox;
// mod terrain;

pub struct Renderer {
    pub entry: vulkanalia::Entry,
    pub instance: vulkanalia::Instance,
    pub device: vulkanalia::Device,
    pub physical_device: vk::PhysicalDevice,
    pub qfamindices: u32,
    queue: vk::Queue,

    #[cfg(feature = "logging")]
    debug_messenger: vk::DebugUtilsMessengerEXT,

    swapchain_data: SwapchainData,

    frame_data: [FrameData; 2],
    frame_count: usize,

    allocator: vulkanalia_vma::Allocator,

    draw_image: AllocatedImage,
    draw_extent: vk::Extent2D,

    descriptor_allocator: DescriptorAllocator,

    aspect_ratio: f32,

    skybox_data: skybox::Data,
}

impl Renderer {
    const fn get_current_framedata(&self) -> FrameData {
        self.frame_data[self.frame_count % 2]
    }

    pub fn new(window: &sdl3::video::Window) -> Self
    where
        Self: Sized,
    {
        let loader =
            unsafe { vulkanalia::loader::LibloadingLoader::new(vulkanalia::loader::LIBRARY) }
                .unwrap();
        let entry = unsafe { vulkanalia::Entry::new(loader) }.unwrap();

        let app_info = vk::ApplicationInfo::builder()
            .api_version(vk::make_version(1, 3, 206))
            .application_name(b"Shadow Engine");

        let extensions = window
            .vulkan_instance_extensions()
            .unwrap()
            .into_iter()
            .map(|s| CString::new(s).expect("String contained null bytes"))
            .collect::<Vec<CString>>();

        let mut extension_names: Vec<*const i8> = extensions.iter().map(|cs| cs.as_ptr()).collect();

        #[cfg(feature = "logging")]
        extension_names.push(vk::EXT_DEBUG_UTILS_EXTENSION.name.as_cstr().as_ptr());

        #[cfg(feature = "logging")]
        let mut debugcreateinfo = vk::DebugUtilsMessengerCreateInfoEXT::builder()
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
            .user_callback(Some(debug::callback));

        let create_info = vk::InstanceCreateInfo::builder()
            .application_info(&app_info)
            .enabled_extension_names(&extension_names);

        #[cfg(feature = "logging")]
        let layers = vec![c"VK_LAYER_KHRONOS_validation".as_ptr()];

        #[cfg(feature = "logging")]
        let create_info = create_info
            .enabled_layer_names(&layers)
            .push_next(&mut debugcreateinfo);

        let instance = unsafe { entry.create_instance(&create_info, None) }.unwrap();

        #[cfg(feature = "logging")]
        piglog::note!("Running Vulkan Version: {}", instance.version());

        #[cfg(feature = "logging")]
        let debug_messenger =
            unsafe { instance.create_debug_utils_messenger_ext(&debugcreateinfo, None) }.unwrap();

        let devices = unsafe { instance.enumerate_physical_devices() }.unwrap();
        let physical_device = *devices
            .iter()
            .find(|device| {
                let props = unsafe { instance.get_physical_device_properties(**device) };
                props.device_type == vk::PhysicalDeviceType::DISCRETE_GPU
                    && props.api_version >= vk::make_version(1, 3, 0)
            })
            .unwrap_or_else(|| {
                devices
                    .iter()
                    .find(|device| {
                        let props = unsafe { instance.get_physical_device_properties(**device) };
                        props.device_type == vk::PhysicalDeviceType::INTEGRATED_GPU
                            && props.api_version >= vk::make_version(1, 3, 0)
                    })
                    .unwrap()
            });

        let device_extensions = vec![vk::KHR_SWAPCHAIN_EXTENSION.name.as_cstr().as_ptr()];

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

        let queue_create_info = &[vk::DeviceQueueCreateInfo::builder()
            .queue_priorities(&[1.])
            .queue_family_index(qfamindices)];

        let device = unsafe {
            instance
                .create_device(
                    physical_device,
                    &vk::DeviceCreateInfo::builder()
                        .enabled_extension_names(&device_extensions)
                        .queue_create_infos(queue_create_info)
                        .push_next(
                            &mut vk::PhysicalDeviceVulkan12Features::builder()
                                .descriptor_indexing(true)
                                .buffer_device_address(true),
                        )
                        .push_next(
                            &mut vk::PhysicalDeviceVulkan13Features::builder()
                                .dynamic_rendering(true)
                                .synchronization2(true),
                        ),
                    None,
                )
                .unwrap()
        };

        let queue = unsafe { device.get_device_queue(qfamindices, 0) };

        let swapchain_data =
            SwapchainData::new(window, &instance, &physical_device, &[qfamindices], &device);

        let frame_data = [FrameData::new(&device), FrameData::new(&device)];

        let mut alloc_create_info =
            vulkanalia_vma::AllocatorOptions::new(&instance, &device, physical_device);
        alloc_create_info.flags = vulkanalia_vma::AllocatorCreateFlags::BUFFER_DEVICE_ADDRESS;

        let allocator = unsafe { vulkanalia_vma::Allocator::new(&alloc_create_info) }.unwrap();

        let win_size = window.size();
        let draw_extent = vk::Extent2D {
            height: win_size.1,
            width: win_size.0,
        };

        let draw_image = AllocatedImage::new(
            vk::Format::R16G16B16A16_SFLOAT,
            vk::ImageUsageFlags::TRANSFER_SRC
                | vk::ImageUsageFlags::TRANSFER_DST
                | vk::ImageUsageFlags::STORAGE
                | vk::ImageUsageFlags::COLOR_ATTACHMENT,
            vk::Extent3D::builder()
                .width(draw_extent.width)
                .height(draw_extent.height)
                .depth(1)
                .build(),
            vk::ImageAspectFlags::COLOR,
            &allocator,
            &device,
        );

        let sizes: Vec<(vk::DescriptorType, u32)> = (0..10)
            .map(|_| (vk::DescriptorType::STORAGE_IMAGE, 1))
            .collect();

        let descriptor_allocator = DescriptorAllocator::new(&device, 10, &sizes);
        // let draw_image_descriptor_layout = {
        //     let mut builder = DescriptorLayoutBuilder::new();
        //     builder.add_binding(0, vk::DescriptorType::STORAGE_IMAGE);
        //     builder.build(
        //         &device,
        //         vk::ShaderStageFlags::COMPUTE,
        //         vk::DescriptorSetLayoutCreateFlags::empty(),
        //     )
        // };
        // let draw_image_descriptors =
        //     descriptor_allocator.allocate(&device, draw_image_descriptor_layout);
        //
        // let img_info = [vk::DescriptorImageInfo::default()
        //     .image_layout(vk::ImageLayout::GENERAL)
        //     .image_view(draw_image.view)];
        //
        // let draw_image_write = vk::WriteDescriptorSet::default()
        //     .dst_binding(0)
        //     .dst_set(draw_image_descriptors)
        //     .descriptor_count(1)
        //     .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
        //     .image_info(&img_info);
        //
        // unsafe { device.update_descriptor_sets(&[draw_image_write], &[]) };

        let skybox_data = skybox::Data::new(&device, &allocator, &queue).unwrap();

        let (width, height) = window.size();

        Self {
            entry,
            instance,
            device,
            physical_device,
            qfamindices,
            #[cfg(feature = "logging")]
            debug_messenger,
            swapchain_data,
            frame_data,
            frame_count: 0,
            allocator,

            draw_extent,
            draw_image,

            descriptor_allocator,
            // draw_image_descriptors,
            // draw_image_descriptor_layout,

            // comp_pipeline,
            // comp_pipeline_layout,
            //
            // grapics_pipeline,
            // graphics_pipeline_layout,
            queue,

            skybox_data,

            aspect_ratio: width as f32 / height as f32,
        }
    }

    pub fn resize(&mut self, window: &sdl3::video::Window) {
        let (width, height) = window.size();

        unsafe { self.device.device_wait_idle() }.unwrap();
        self.swapchain_data.flush(&self.device, &self.instance);
        self.swapchain_data = SwapchainData::new(
            window,
            &self.instance,
            &self.physical_device,
            &[self.qfamindices],
            &self.device,
        );

        self.draw_extent = vk::Extent2D { height, width };
        self.draw_image.flush(&self.device, &self.allocator);
        self.draw_image = AllocatedImage::new(
            vk::Format::R16G16B16A16_SFLOAT,
            vk::ImageUsageFlags::TRANSFER_SRC
                | vk::ImageUsageFlags::TRANSFER_DST
                | vk::ImageUsageFlags::STORAGE
                | vk::ImageUsageFlags::COLOR_ATTACHMENT,
            vk::Extent3D::builder()
                .width(self.draw_extent.width)
                .height(self.draw_extent.height)
                .depth(1)
                .build(),
            vk::ImageAspectFlags::COLOR,
            &self.allocator,
            &self.device,
        );

        self.aspect_ratio = width as f32 / height as f32;
    }

    pub fn render(&mut self, view_mat: glam::Mat4, sun_dir: glam::Vec3A) {
        let fence = self.get_current_framedata().render_fence;
        unsafe { self.device.wait_for_fences(&[fence], true, 1_000_000_000) }.unwrap();
        unsafe { self.device.reset_fences(&[fence]) }.unwrap();
        let cmd_buf = self.get_current_framedata().buf;

        let swapchain_images = unsafe {
            self.device
                .get_swapchain_images_khr(self.swapchain_data.swapchain)
        }
        .unwrap();

        let next_img = unsafe {
            self.device
                .acquire_next_image_khr(
                    self.swapchain_data.swapchain,
                    1_000_000_000,
                    self.get_current_framedata().swapchain_semaphore,
                    vk::Fence::null(),
                )
                .unwrap()
        };

        #[cfg(feature = "logging")]
        if next_img.1 == vk::SuccessCode::SUBOPTIMAL_KHR {
            warning!("Suboptimal swapchain for the surface");
        }

        unsafe {
            self.device
                .reset_command_buffer(cmd_buf, vk::CommandBufferResetFlags::empty())
        }
        .unwrap();

        self.draw_extent.width = self.draw_image.extent.width;
        self.draw_extent.height = self.draw_image.extent.height;

        unsafe {
            self.device.begin_command_buffer(
                cmd_buf,
                &vk::CommandBufferBeginInfo::builder()
                    .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT),
            )
        }
        .unwrap();

        transition_image(
            cmd_buf,
            self.draw_image.image,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            &self.device,
        );

        let sky_view_mat = {
            let mut tmp = view_mat;

            tmp.col_mut(3).x = 0.0;
            tmp.col_mut(3).y = 0.0;
            tmp.col_mut(3).z = 0.0;
            tmp
        };

        self.skybox_data.draw(
            &self.device,
            cmd_buf,
            self.draw_image.view,
            self.draw_extent,
            skybox::PushConstants::new(
                glam::Mat4::perspective_infinite_rh(
                    std::f32::consts::FRAC_PI_3,
                    self.aspect_ratio,
                    2.,
                ) * sky_view_mat,
                glam::vec3a(0.7, 0.7, 1.0),
                sun_dir,
            ),
        );

        unsafe { self.device.cmd_end_rendering(cmd_buf) };

        transition_image(
            cmd_buf,
            self.draw_image.image,
            vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
            &self.device,
        );
        transition_image(
            cmd_buf,
            swapchain_images[next_img.0 as usize],
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            &self.device,
        );

        copy_image_to_image(
            cmd_buf,
            self.draw_image.image,
            swapchain_images[next_img.0 as usize],
            self.draw_extent,
            self.swapchain_data.extent,
            &self.device,
        );

        transition_image(
            cmd_buf,
            swapchain_images[next_img.0 as usize],
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            vk::ImageLayout::PRESENT_SRC_KHR,
            &self.device,
        );

        unsafe { self.device.end_command_buffer(cmd_buf) }.unwrap();

        let current_render_semaphore = self.swapchain_data.render_semaphores[next_img.0 as usize];

        unsafe {
            self.device.queue_submit2(
                self.queue,
                &[vk::SubmitInfo2::builder()
                    .wait_semaphore_infos(&[vk::SemaphoreSubmitInfo::builder()
                        .semaphore(self.get_current_framedata().swapchain_semaphore)
                        .stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)])
                    .command_buffer_infos(&[vk::CommandBufferSubmitInfo::builder()
                        .command_buffer(cmd_buf)
                        .device_mask(0)])
                    .signal_semaphore_infos(&[vk::SemaphoreSubmitInfo::builder()
                        .semaphore(current_render_semaphore)
                        .stage_mask(vk::PipelineStageFlags2::ALL_GRAPHICS)])],
                self.get_current_framedata().render_fence,
            )
        }
        .unwrap();

        unsafe {
            self.device.queue_present_khr(
                self.queue,
                &vk::PresentInfoKHR::builder()
                    .swapchains(&[self.swapchain_data.swapchain])
                    .wait_semaphores(&[current_render_semaphore])
                    .image_indices(&[next_img.0]),
            )
        }
        .unwrap();

        self.frame_count = self.frame_count.wrapping_add(1);
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe {
            self.device.device_wait_idle().unwrap();

            self.descriptor_allocator.flush(&self.device);

            self.draw_image.flush(&self.device, &self.allocator);

            self.swapchain_data.flush(&self.device, &self.instance);

            self.skybox_data.destroy(&self.device, &self.allocator);

            #[cfg(feature = "logging")]
            self.instance
                .destroy_debug_utils_messenger_ext(self.debug_messenger, None);

            for i in 0..self.frame_data.len() {
                self.device
                    .destroy_fence(self.frame_data[i].render_fence, None);
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
