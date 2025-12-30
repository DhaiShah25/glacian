use rootcause::{Report, prelude::ResultExt};

use super::utils::load_shader_module;
use bytemuck::{Pod, Zeroable};
use glam::Vec3A;
use vulkanalia::vk::{self, DeviceV1_0, DeviceV1_3, Handle, HasBuilder};

pub struct Data {
    pipeline: vk::Pipeline,
    layout: vk::PipelineLayout,
}

#[repr(C, packed)]
#[derive(Debug, Pod, Zeroable, Copy, Clone)]
pub struct PushConstants {
    sky_color: Vec3A,
}

impl PushConstants {
    pub const fn new(sky_color: Vec3A) -> Self {
        Self { sky_color }
    }
}

impl Data {
    pub fn new(device: &vulkanalia::Device) -> Result<Self, Report> {
        let layout = {
            let push_range_contants = [vk::PushConstantRange::builder()
                .stage_flags(vk::ShaderStageFlags::FRAGMENT)
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

        Ok(Self { layout, pipeline })
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
                vk::ShaderStageFlags::FRAGMENT,
                0,
                bytemuck::bytes_of(&constants),
            );
            device.cmd_draw(cmd, 3, 1, 0, 0);
        }
    }

    pub fn destroy(&mut self, device: &vulkanalia::Device) {
        unsafe {
            device.destroy_pipeline(self.pipeline, None);
            device.destroy_pipeline_layout(self.layout, None);
        }
    }
}
