use crate::render::allocations::AllocatedBuffer;
use rootcause::{Report, prelude::ResultExt};

use super::utils::load_shader_module;
use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3A};
use vulkanalia::vk::{self, DeviceV1_0, DeviceV1_3, Handle, HasBuilder};

pub struct Data {
    pipeline: vk::Pipeline,
    layout: vk::PipelineLayout,
    idx_buffer: AllocatedBuffer,
}

#[repr(C, packed)]
#[derive(Debug, Pod, Zeroable, Copy, Clone)]
pub struct PushConstants {
    view_proj: Mat4,
    sky_color: Vec3A,
    sun_dir: Vec3A,
}

impl PushConstants {
    pub const fn new(view_proj: Mat4, sky_color: Vec3A, sun_dir: Vec3A) -> Self {
        Self {
            view_proj,
            sky_color,
            sun_dir,
        }
    }
}

impl Data {
    pub fn new(
        device: &vulkanalia::Device,
        allocator: &vulkanalia_vma::Allocator,
        transfer_queue: &vk::Queue,
    ) -> Result<Self, Report> {
        let layout = {
            let push_range_contants = [vk::PushConstantRange::builder()
                .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT)
                .size(size_of::<PushConstants>() as u32)];
            let info =
                vk::PipelineLayoutCreateInfo::builder().push_constant_ranges(&push_range_contants);
            unsafe { device.create_pipeline_layout(&info, None) }?
        };
        let pipeline = {
            let shader = load_shader_module("./assets/shaders/skybox.spv", device)
                .context("Issue Loading Skybox Shader")?;

            let dynamic_state_info = vk::PipelineDynamicStateCreateInfo::builder()
                .dynamic_states(&[vk::DynamicState::SCISSOR, vk::DynamicState::VIEWPORT]);

            let input_assembly_info = vk::PipelineInputAssemblyStateCreateInfo::builder()
                .topology(vk::PrimitiveTopology::TRIANGLE_LIST);

            let rasterization_state_info = vk::PipelineRasterizationStateCreateInfo::builder()
                .cull_mode(vk::CullModeFlags::BACK)
                .front_face(vk::FrontFace::CLOCKWISE)
                .line_width(1.)
                .polygon_mode(vk::PolygonMode::FILL);

            let viewport_state_info = vk::PipelineViewportStateCreateInfo::builder()
                .viewport_count(1)
                .scissor_count(1);

            let color_blend_attachment = vk::PipelineColorBlendAttachmentState::builder()
                .blend_enable(false)
                .color_write_mask(
                    vk::ColorComponentFlags::R
                        | vk::ColorComponentFlags::G
                        | vk::ColorComponentFlags::B
                        | vk::ColorComponentFlags::A,
                );
            let color_blend_state_info = vk::PipelineColorBlendStateCreateInfo::builder()
                .logic_op_enable(false)
                .logic_op(vk::LogicOp::COPY)
                .attachments(std::slice::from_ref(&color_blend_attachment));

            let shader_stages = [
                vk::PipelineShaderStageCreateInfo::builder()
                    .stage(vk::ShaderStageFlags::VERTEX)
                    .module(shader)
                    .name(b"vs_main\0"),
                vk::PipelineShaderStageCreateInfo::builder()
                    .stage(vk::ShaderStageFlags::FRAGMENT)
                    .module(shader)
                    .name(b"fs_main\0"),
            ];

            let multisample_state_info = vk::PipelineMultisampleStateCreateInfo::builder()
                .sample_shading_enable(false)
                .rasterization_samples(vk::SampleCountFlags::_1)
                .min_sample_shading(1.)
                .alpha_to_coverage_enable(false)
                .alpha_to_one_enable(false);

            let depth_stencil_state_info = vk::PipelineDepthStencilStateCreateInfo::builder()
                .depth_test_enable(false)
                .depth_write_enable(false)
                .depth_compare_op(vk::CompareOp::NEVER)
                .min_depth_bounds(1.)
                .max_depth_bounds(1.)
                .depth_bounds_test_enable(false)
                .stencil_test_enable(false);

            let mut rendering_create_info = vk::PipelineRenderingCreateInfo::builder()
                .color_attachment_formats(&[vk::Format::R16G16B16A16_SFLOAT]);

            let vert_info = vk::PipelineVertexInputStateCreateInfo::builder();

            let info = vk::GraphicsPipelineCreateInfo::builder()
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
                .layout(layout);

            let create_infos = [info];
            let pipeline = unsafe {
                device
                    .create_graphics_pipelines(vk::PipelineCache::null(), &create_infos, None)
                    .expect("Failed to create graphics pipeline")
                    .0[0]
            };

            unsafe {
                device.destroy_shader_module(shader, None);
            }

            pipeline
        };

        let idx_buffer = AllocatedBuffer::new(
            allocator,
            36 * size_of::<u16>() as u64,
            vk::BufferUsageFlags::INDEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
            vulkanalia_vma::MemoryUsage::AutoPreferDevice,
        );

        let mut staging = AllocatedBuffer::new(
            allocator,
            36 * size_of::<u16>() as u64,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vulkanalia_vma::MemoryUsage::AutoPreferHost,
        );

        let mem = unsafe { allocator.map_memory(staging.allocation) }
            .context("Not Able To Access Necessary Memory For Skybox")?;

        let data_slice: &mut [u8] =
            unsafe { std::slice::from_raw_parts_mut(mem, 36 * size_of::<u16>()) };

        const INDICES: [u16; 36] = [
            // Front face (v0, v1, v2, v3)
            0, 1, 2, // First triangle
            0, 2, 3, // Second triangle
            // Right face (v1, v5, v6, v2)
            1, 5, 6, // First triangle
            1, 6, 2, // Second triangle
            // Back face (v5, v4, v7, v6)
            5, 4, 7, // First triangle
            5, 7, 6, // Second triangle
            // Left face (v4, v0, v3, v7)
            4, 0, 3, // First triangle
            4, 3, 7, // Second triangle
            // Top face (v3, v2, v6, v7)
            3, 2, 6, // First triangle
            3, 6, 7, // Second triangle
            // Bottom face (v4, v5, v1, v0)
            4, 5, 1, // First triangle
            4, 1, 0, // Second triangle
        ];

        let index_bytes = bytemuck::cast_slice(&INDICES);
        data_slice[0..36 * size_of::<u16>()].copy_from_slice(index_bytes);

        unsafe { allocator.unmap_memory(staging.allocation) };

        unsafe {
            let submit_fence = device.create_fence(&vk::FenceCreateInfo::default(), None)?;
            let cmd_pool =
                device.create_command_pool(&vk::CommandPoolCreateInfo::default(), None)?;
            let cmd = device.allocate_command_buffers(
                &vk::CommandBufferAllocateInfo::builder()
                    .command_pool(cmd_pool)
                    .level(vk::CommandBufferLevel::PRIMARY)
                    .command_buffer_count(1),
            )?[0];
            device.begin_command_buffer(
                cmd,
                &vk::CommandBufferBeginInfo::builder()
                    .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT),
            )?;

            device.cmd_copy_buffer(
                cmd,
                staging.buf,
                idx_buffer.buf,
                &[vk::BufferCopy::builder().size(36 * size_of::<u16>() as u64)],
            );

            device.end_command_buffer(cmd)?;

            device.queue_submit2(
                *transfer_queue,
                &[vk::SubmitInfo2::builder().command_buffer_infos(&[
                    vk::CommandBufferSubmitInfo::builder()
                        .command_buffer(cmd)
                        .device_mask(0),
                ])],
                submit_fence,
            )?;
            device.wait_for_fences(&[submit_fence], true, 999999999)?;

            device.destroy_command_pool(cmd_pool, None);
            device.destroy_fence(submit_fence, None);
        }

        staging.flush(allocator);

        Ok(Self {
            layout,
            pipeline,
            idx_buffer,
        })
    }

    pub fn draw(
        &self,
        device: &vulkanalia::Device,
        cmd: vk::CommandBuffer,
        view: vk::ImageView,
        draw_extent: vk::Extent2D,
        constants: PushConstants,
    ) {
        let color = vk::RenderingAttachmentInfo::builder()
            .image_view(view)
            .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE);
        let color = [color];
        let rend_info = vk::RenderingInfo::builder()
            .render_area(
                vk::Rect2D::builder()
                    .extent(draw_extent)
                    .offset(vk::Offset2D { x: 0, y: 0 }),
            )
            .color_attachments(&color)
            .layer_count(1);

        unsafe {
            device.cmd_begin_rendering(cmd, &rend_info);
            device.cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, self.pipeline);

            device.cmd_set_viewport(
                cmd,
                0,
                &[vk::Viewport {
                    width: draw_extent.width as f32,
                    height: draw_extent.height as f32,
                    x: 0.,
                    y: 0.,
                    min_depth: 0.,
                    max_depth: 1.,
                }],
            );
            device.cmd_set_scissor(
                cmd,
                0,
                &[vk::Rect2D::builder()
                    .extent(draw_extent)
                    .offset(vk::Offset2D { x: 0, y: 0 })],
            );

            device.cmd_push_constants(
                cmd,
                self.layout,
                vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                0,
                bytemuck::bytes_of(&constants),
            );
            device.cmd_bind_index_buffer(cmd, self.idx_buffer.buf, 0, vk::IndexType::UINT16);
            device.cmd_draw_indexed(cmd, 36, 1, 0, 0, 0);
        }
    }

    pub fn destroy(&mut self, device: &vulkanalia::Device, allocator: &vulkanalia_vma::Allocator) {
        unsafe {
            device.destroy_pipeline(self.pipeline, None);
            device.destroy_pipeline_layout(self.layout, None);
        }
        self.idx_buffer.flush(allocator);
    }
}
