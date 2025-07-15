use ash::vk;
use std::io::Read;
use tracing::{info, warn};

mod utils;
use utils::{AllocatedImage, DelQueue, DescriptorAllocator, DescriptorLayoutBuilder, FrameData};
mod swapchain;
use swapchain::SwapchainData;
mod debug;

pub struct RenderEngine<'a> {
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

    swapchain_data: SwapchainData,

    frame_data: [FrameData; 2],
    frame_count: usize,

    allocator: Option<vk_mem::Allocator>,

    draw_image: AllocatedImage,
    draw_extent: vk::Extent2D,

    descriptor_allocator: DescriptorAllocator,

    draw_image_descriptors: vk::DescriptorSet,
    draw_image_descriptor_layout: vk::DescriptorSetLayout,

    comp_pipeline: vk::Pipeline,
    comp_pipeline_layout: vk::PipelineLayout,

    grapics_pipeline: vk::Pipeline,
    graphics_pipeline_layout: vk::PipelineLayout,

    del_queue: utils::DelQueue<'a>,
}

impl RenderEngine<'_> {
    const fn get_current_framedata(&self) -> FrameData {
        self.frame_data[self.frame_count % 2]
    }

    pub fn new(window: &glfw::Window) -> Self
    where
        Self: Sized,
    {
        let entry = unsafe { ash::Entry::load() }.unwrap();

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
            .pfn_user_callback(Some(debug::callback));

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

        let device_extensions = vec![ash::khr::swapchain::NAME.as_ptr()];

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
                            &mut vk::PhysicalDeviceBufferDeviceAddressFeatures::default()
                                .buffer_device_address(true),
                        )
                        .push_next(
                            &mut vk::PhysicalDeviceDynamicRenderingFeatures::default()
                                .dynamic_rendering(true),
                        )
                        .push_next(
                            &mut vk::PhysicalDeviceSynchronization2Features::default()
                                .synchronization2(true),
                        ),
                    None,
                )
                .unwrap()
        };

        let queue = unsafe { device.get_device_queue(qfamindices, 0) };

        let swapchain_data = SwapchainData::new(
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

        let win_size = window.get_size();
        let draw_extent = vk::Extent2D {
            height: win_size.1 as u32,
            width: win_size.0 as u32,
        };

        let mut del_queue = DelQueue::new();

        let draw_image = AllocatedImage::new(
            vk::Format::R16G16B16A16_SFLOAT,
            vk::ImageUsageFlags::TRANSFER_SRC
                | vk::ImageUsageFlags::TRANSFER_DST
                | vk::ImageUsageFlags::STORAGE
                | vk::ImageUsageFlags::COLOR_ATTACHMENT,
            vk::Extent3D::default()
                .width(draw_extent.width)
                .height(draw_extent.height)
                .depth(1),
            vk::ImageAspectFlags::COLOR,
            &allocator,
            &device,
        );

        let sizes: Vec<(vk::DescriptorType, u32)> = (0..10)
            .into_iter()
            .map(|_| (vk::DescriptorType::STORAGE_IMAGE, 1))
            .collect();

        let descriptor_allocator = DescriptorAllocator::new(&device, 10, &sizes);
        let draw_image_descriptor_layout = {
            let mut builder = DescriptorLayoutBuilder::new();
            builder.add_binding(0, vk::DescriptorType::STORAGE_IMAGE);
            builder.build(
                &device,
                vk::ShaderStageFlags::COMPUTE,
                vk::DescriptorSetLayoutCreateFlags::empty(),
            )
        };
        let draw_image_descriptors =
            descriptor_allocator.allocate(&device, draw_image_descriptor_layout.clone());

        let img_info = [vk::DescriptorImageInfo::default()
            .image_layout(vk::ImageLayout::GENERAL)
            .image_view(draw_image.view)];

        let draw_image_write = vk::WriteDescriptorSet::default()
            .dst_binding(0)
            .dst_set(draw_image_descriptors)
            .descriptor_count(1)
            .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
            .image_info(&img_info);

        unsafe { device.update_descriptor_sets(&[draw_image_write], &[]) };

        let comp_pipeline_layout = {
            let tmp = [draw_image_descriptor_layout];
            let compute_layout = vk::PipelineLayoutCreateInfo::default().set_layouts(&tmp);

            unsafe { device.create_pipeline_layout(&compute_layout, None) }.unwrap()
        };
        let comp_pipeline = {
            let comp_shader = load_shader_module("./assets/shaders/gradient.spv", &device);
            let comp_pipeline_create_info = vk::ComputePipelineCreateInfo::default()
                .layout(comp_pipeline_layout)
                .stage(
                    vk::PipelineShaderStageCreateInfo::default()
                        .stage(vk::ShaderStageFlags::COMPUTE)
                        .module(comp_shader)
                        .name(c"main"),
                );

            let tmp_device = device.clone();
            del_queue.add(Box::new(move || unsafe {
                tmp_device.destroy_shader_module(comp_shader, None);
            }));

            unsafe {
                device
                    .create_compute_pipelines(
                        vk::PipelineCache::null(),
                        &[comp_pipeline_create_info],
                        None,
                    )
                    .unwrap()[0]
            }
        };

        let graphics_pipeline_layout = {
            let info = vk::PipelineLayoutCreateInfo::default();
            unsafe { device.create_pipeline_layout(&info, None) }.unwrap()
        };
        let grapics_pipeline = {
            let vert_shader = load_shader_module("./assets/shaders/vert.spv", &device);
            let frag_shader = load_shader_module("./assets/shaders/frag.spv", &device);

            let tmp_device = device.clone();
            del_queue.add(Box::new(move || unsafe {
                tmp_device.destroy_shader_module(vert_shader, None);
                tmp_device.destroy_shader_module(frag_shader, None);
            }));

            let dynamic_state_info = vk::PipelineDynamicStateCreateInfo::default()
                .dynamic_states(&[vk::DynamicState::SCISSOR, vk::DynamicState::VIEWPORT]);

            let input_assembly_info = vk::PipelineInputAssemblyStateCreateInfo::default()
                .topology(vk::PrimitiveTopology::TRIANGLE_LIST);

            let rasterization_state_info = vk::PipelineRasterizationStateCreateInfo::default()
                .cull_mode(vk::CullModeFlags::BACK)
                .front_face(vk::FrontFace::CLOCKWISE)
                .line_width(1.)
                .polygon_mode(vk::PolygonMode::FILL);

            let viewport_state_info = vk::PipelineViewportStateCreateInfo::default()
                .viewport_count(1)
                .scissor_count(1);

            let color_blend_attachment = vk::PipelineColorBlendAttachmentState::default()
                .blend_enable(false)
                .color_write_mask(
                    vk::ColorComponentFlags::R
                        | vk::ColorComponentFlags::G
                        | vk::ColorComponentFlags::B
                        | vk::ColorComponentFlags::A,
                );
            let color_blend_state_info = vk::PipelineColorBlendStateCreateInfo::default()
                .logic_op_enable(false)
                .logic_op(vk::LogicOp::COPY)
                .attachments(std::slice::from_ref(&color_blend_attachment));

            let shader_stages = [
                vk::PipelineShaderStageCreateInfo::default()
                    .stage(vk::ShaderStageFlags::VERTEX)
                    .module(vert_shader)
                    .name(c"main"),
                vk::PipelineShaderStageCreateInfo::default()
                    .stage(vk::ShaderStageFlags::FRAGMENT)
                    .module(frag_shader)
                    .name(c"main"),
            ];

            let multisample_state_info = vk::PipelineMultisampleStateCreateInfo::default()
                .sample_shading_enable(false)
                .rasterization_samples(vk::SampleCountFlags::TYPE_1)
                .min_sample_shading(1.)
                .alpha_to_coverage_enable(false)
                .alpha_to_one_enable(false);

            let depth_stencil_state_info = vk::PipelineDepthStencilStateCreateInfo::default()
                .depth_test_enable(false)
                .depth_write_enable(false)
                .depth_compare_op(vk::CompareOp::NEVER)
                .min_depth_bounds(1.)
                .max_depth_bounds(1.)
                .depth_bounds_test_enable(false)
                .stencil_test_enable(false);

            let mut rendering_create_info = vk::PipelineRenderingCreateInfo::default()
                .color_attachment_formats(&[vk::Format::R16G16B16A16_SFLOAT]);

            let vert_info = vk::PipelineVertexInputStateCreateInfo::default();

            let info = vk::GraphicsPipelineCreateInfo::default()
                .dynamic_state(&dynamic_state_info)
                .input_assembly_state(&input_assembly_info)
                .rasterization_state(&rasterization_state_info)
                .viewport_state(&viewport_state_info)
                .color_blend_state(&color_blend_state_info)
                .stages(&shader_stages)
                .vertex_input_state(&vert_info)
                .depth_stencil_state(&depth_stencil_state_info)
                .multisample_state(&multisample_state_info)
                .push_next(&mut rendering_create_info)
                .layout(graphics_pipeline_layout);

            let create_infos = [info];
            unsafe {
                device
                    .create_graphics_pipelines(vk::PipelineCache::null(), &create_infos, None)
                    .expect("Failed to create graphics pipeline")[0]
            }
        };

        Self {
            entry,
            instance,
            device,
            physical_device,
            qfamindices,
            #[cfg(feature = "debug")]
            debug_utils,
            swapchain_data,
            frame_data,
            frame_count: 0,
            allocator: Some(allocator),

            draw_extent,
            draw_image,

            descriptor_allocator,
            draw_image_descriptors,
            draw_image_descriptor_layout,

            comp_pipeline,
            comp_pipeline_layout,

            grapics_pipeline,
            graphics_pipeline_layout,

            queue,
            del_queue,
        }
    }

    pub fn resize(&mut self, window: &glfw::Window, width: u32, height: u32) {
        unsafe { self.device.device_wait_idle() }.unwrap();
        self.swapchain_data.flush(&self.device);
        self.swapchain_data = SwapchainData::new(
            window,
            &self.entry,
            &self.instance,
            &self.physical_device,
            &[self.qfamindices],
            &self.device,
        );
        self.draw_extent = vk::Extent2D { height, width };
        self.draw_image
            .flush(&self.device, &self.allocator.as_ref().unwrap());
        self.draw_image = AllocatedImage::new(
            vk::Format::R16G16B16A16_SFLOAT,
            vk::ImageUsageFlags::TRANSFER_SRC
                | vk::ImageUsageFlags::TRANSFER_DST
                | vk::ImageUsageFlags::STORAGE
                | vk::ImageUsageFlags::COLOR_ATTACHMENT,
            vk::Extent3D::default()
                .width(self.draw_extent.width)
                .height(self.draw_extent.height)
                .depth(1),
            vk::ImageAspectFlags::COLOR,
            &self.allocator.as_ref().unwrap(),
            &self.device,
        );

        let img_info = [vk::DescriptorImageInfo::default()
            .image_layout(vk::ImageLayout::GENERAL)
            .image_view(self.draw_image.view)];

        let draw_image_write = vk::WriteDescriptorSet::default()
            .dst_binding(0)
            .dst_set(self.draw_image_descriptors)
            .descriptor_count(1)
            .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
            .image_info(&img_info);

        unsafe { self.device.update_descriptor_sets(&[draw_image_write], &[]) };
    }

    fn draw_background(&self, cmd: vk::CommandBuffer) {
        unsafe {
            self.device.cmd_clear_color_image(
                cmd,
                self.draw_image.image,
                vk::ImageLayout::GENERAL,
                &vk::ClearColorValue::default(),
                &[vk::ImageSubresourceRange::default()
                    .base_array_layer(0)
                    .base_mip_level(0)
                    .level_count(vk::REMAINING_MIP_LEVELS)
                    .layer_count(vk::REMAINING_ARRAY_LAYERS)
                    .aspect_mask(vk::ImageAspectFlags::COLOR)],
            );
            self.device
                .cmd_bind_pipeline(cmd, vk::PipelineBindPoint::COMPUTE, self.comp_pipeline);
            self.device.cmd_bind_descriptor_sets(
                cmd,
                vk::PipelineBindPoint::COMPUTE,
                self.comp_pipeline_layout,
                0,
                &[self.draw_image_descriptors],
                &[],
            );
            self.device.cmd_dispatch(
                cmd,
                (self.draw_extent.width as f32 / 16.).ceil() as u32,
                (self.draw_extent.height as f32 / 16.).ceil() as u32,
                1,
            );
        };
    }

    fn draw_geometry(&self, cmd: vk::CommandBuffer) {
        let color = vk::RenderingAttachmentInfo::default()
            .image_view(self.draw_image.view)
            .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .load_op(vk::AttachmentLoadOp::LOAD)
            .store_op(vk::AttachmentStoreOp::STORE);
        let color = [color];
        let rend_info = vk::RenderingInfo::default()
            .render_area(
                vk::Rect2D::default()
                    .extent(vk::Extent2D {
                        height: self.draw_extent.height,
                        width: self.draw_extent.width,
                    })
                    .offset(vk::Offset2D { x: 0, y: 0 }),
            )
            .color_attachments(&color)
            .layer_count(1);

        unsafe {
            self.device.cmd_begin_rendering(cmd, &rend_info);
            self.device.cmd_bind_pipeline(
                cmd,
                vk::PipelineBindPoint::GRAPHICS,
                self.grapics_pipeline,
            );

            self.device.cmd_set_viewport(
                cmd,
                0,
                &[vk::Viewport {
                    width: self.draw_extent.width as f32,
                    height: self.draw_extent.height as f32,
                    x: 0.,
                    y: 0.,
                    min_depth: 0.,
                    max_depth: 1.,
                }],
            );
            self.device.cmd_set_scissor(
                cmd,
                0,
                &[vk::Rect2D::default()
                    .extent(vk::Extent2D {
                        height: self.draw_extent.height,
                        width: self.draw_extent.width,
                    })
                    .offset(vk::Offset2D { x: 0, y: 0 })],
            );

            self.device.cmd_draw(cmd, 3, 1, 0, 0);
            self.device.cmd_end_rendering(cmd);
        }
    }

    pub fn render(&mut self, resized: bool) {
        let fence = self.get_current_framedata().render_fence;
        unsafe { self.device.wait_for_fences(&[fence], true, 1_000_000_000) }.unwrap();
        unsafe { self.device.reset_fences(&[fence]) }.unwrap();
        let cmd_buf = self.get_current_framedata().buf;

        let swapchain_images = unsafe {
            self.swapchain_data
                .swapchain_device
                .get_swapchain_images(self.swapchain_data.swapchain)
        }
        .unwrap();

        let next_img = unsafe {
            self.swapchain_data
                .swapchain_device
                .acquire_next_image(
                    self.swapchain_data.swapchain,
                    1_000_000_000,
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

        self.draw_extent.width = self.draw_image.extent.width;
        self.draw_extent.height = self.draw_image.extent.height;

        unsafe {
            self.device.begin_command_buffer(
                cmd_buf,
                &vk::CommandBufferBeginInfo::default()
                    .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT),
            )
        }
        .unwrap();

        transition_image(
            cmd_buf,
            self.draw_image.image,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::GENERAL,
            &self.device,
        );

        self.draw_background(cmd_buf);

        transition_image(
            cmd_buf,
            self.draw_image.image,
            vk::ImageLayout::GENERAL,
            vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            &self.device,
        );

        self.draw_geometry(cmd_buf);

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
                &[vk::SubmitInfo2::default()
                    .wait_semaphore_infos(&[vk::SemaphoreSubmitInfo::default()
                        .semaphore(self.get_current_framedata().swapchain_semaphore)
                        .stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)])
                    .command_buffer_infos(&[vk::CommandBufferSubmitInfo::default()
                        .command_buffer(cmd_buf)
                        .device_mask(0)])
                    .signal_semaphore_infos(&[vk::SemaphoreSubmitInfo::default()
                        .semaphore(current_render_semaphore)
                        .stage_mask(vk::PipelineStageFlags2::ALL_GRAPHICS)])],
                self.get_current_framedata().render_fence,
            )
        }
        .unwrap();

        unsafe {
            self.swapchain_data.swapchain_device.queue_present(
                self.queue,
                &vk::PresentInfoKHR::default()
                    .swapchains(&[self.swapchain_data.swapchain])
                    .wait_semaphores(&[current_render_semaphore])
                    .image_indices(&[next_img.0]),
            )
        }
        .unwrap();

        self.frame_count = self.frame_count.wrapping_add(1);
    }
}

impl Drop for RenderEngine<'_> {
    fn drop(&mut self) {
        unsafe {
            self.device.device_wait_idle().unwrap();

            self.descriptor_allocator.flush(&self.device);
            self.device
                .destroy_descriptor_set_layout(self.draw_image_descriptor_layout, None);

            self.del_queue.flush();
            self.draw_image
                .flush(&self.device, self.allocator.as_ref().unwrap());

            self.swapchain_data.flush(&self.device);

            self.device
                .destroy_pipeline_layout(self.comp_pipeline_layout, None);
            self.device.destroy_pipeline(self.comp_pipeline, None);

            self.device
                .destroy_pipeline_layout(self.graphics_pipeline_layout, None);
            self.device.destroy_pipeline(self.grapics_pipeline, None);

            #[cfg(feature = "debug")]
            self.debug_utils
                .0
                .destroy_debug_utils_messenger(self.debug_utils.1, None);

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

            drop(self.allocator.take());

            self.device.destroy_device(None);
            self.instance.destroy_instance(None);
        }
    }
}

fn copy_image_to_image(
    cmd: vk::CommandBuffer,
    source: vk::Image,
    destination: vk::Image,
    src_size: vk::Extent2D,
    dst_size: vk::Extent2D,
    device: &ash::Device,
) {
    // TODO: change to device.cmd_copy_image2
    let mut blit_region = vk::ImageBlit2::default()
        .src_subresource(
            vk::ImageSubresourceLayers::default()
                .aspect_mask(vk::ImageAspectFlags::COLOR)
                .base_array_layer(0)
                .layer_count(1)
                .mip_level(0),
        )
        .dst_subresource(
            vk::ImageSubresourceLayers::default()
                .aspect_mask(vk::ImageAspectFlags::COLOR)
                .base_array_layer(0)
                .layer_count(1)
                .mip_level(0),
        );

    blit_region.src_offsets[1] = vk::Offset3D::default()
        .x(src_size.width as i32)
        .y(src_size.height as i32)
        .z(1);
    blit_region.dst_offsets[1] = vk::Offset3D::default()
        .x(dst_size.width as i32)
        .y(dst_size.height as i32)
        .z(1);

    let binding = [blit_region];

    let blit_info = vk::BlitImageInfo2::default()
        .dst_image(destination)
        .dst_image_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
        .src_image(source)
        .src_image_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL)
        .filter(vk::Filter::LINEAR)
        .regions(&binding);

    unsafe { device.cmd_blit_image2(cmd, &blit_info) };
}

fn transition_image(
    cmd: vk::CommandBuffer,
    image: vk::Image,
    curr_layout: vk::ImageLayout,
    new_layout: vk::ImageLayout,
    device: &ash::Device,
) {
    let subresource_range = vk::ImageSubresourceRange::default()
        .base_array_layer(0)
        .base_mip_level(0)
        .level_count(vk::REMAINING_MIP_LEVELS)
        .layer_count(vk::REMAINING_ARRAY_LAYERS);

    let subresource_range = if new_layout == vk::ImageLayout::DEPTH_ATTACHMENT_OPTIMAL {
        subresource_range.aspect_mask(vk::ImageAspectFlags::DEPTH)
    } else {
        subresource_range.aspect_mask(vk::ImageAspectFlags::COLOR)
    };

    let img_barrier = vk::ImageMemoryBarrier2::default()
        .src_stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
        .src_access_mask(vk::AccessFlags2::MEMORY_WRITE)
        .dst_stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
        .dst_access_mask(vk::AccessFlags2::MEMORY_WRITE | vk::AccessFlags2::MEMORY_READ)
        .old_layout(curr_layout)
        .new_layout(new_layout)
        .subresource_range(subresource_range)
        .image(image);
    unsafe {
        device.cmd_pipeline_barrier2(
            cmd,
            &vk::DependencyInfo::default().image_memory_barriers(&[img_barrier]),
        );
    };
}

fn load_shader_module(path: &str, device: &ash::Device) -> vk::ShaderModule {
    let mut file = std::fs::File::open(path).unwrap();
    let mut buf = Vec::new();
    _ = file.read_to_end(&mut buf);

    let create_info = vk::ShaderModuleCreateInfo::default().code(bytemuck::cast_slice(&buf));
    unsafe { device.create_shader_module(&create_info, None) }.unwrap()
}
