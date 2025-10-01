use super::utils::load_shader_module;
use ash::vk;
use bytemuck::{Pod, Zeroable};
use glam::Mat4;

pub struct Data {
    pipeline: vk::Pipeline,
    layout: vk::PipelineLayout,
}

#[repr(C, packed)]
#[derive(Debug, Pod, Zeroable, Copy, Clone)]
pub struct PushConstants {
    view_proj: Mat4,
}

impl PushConstants {
    pub const fn new(view_proj: Mat4) -> Self {
        Self { view_proj }
    }
}

impl Data {
    pub fn new(
        device: &ash::Device,
        allocator: &vk_mem::Allocator,
        transfer_queue: &vk::Queue,
    ) -> Self {
        let layout = {
            let push_range_contants = [vk::PushConstantRange::default()
                .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT)
                .size(size_of::<PushConstants>() as u32)];
            let info =
                vk::PipelineLayoutCreateInfo::default().push_constant_ranges(&push_range_contants);
            unsafe { device.create_pipeline_layout(&info, None) }.unwrap()
        };
        let pipeline = {
            let vert_shader = load_shader_module("./assets/shaders/terrain_vs.spv", device);
            let frag_shader = load_shader_module("./assets/shaders/terrain_fs.spv", device);

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
                .layout(layout);

            let create_infos = [info];
            let pipeline = unsafe {
                device
                    .create_graphics_pipelines(vk::PipelineCache::null(), &create_infos, None)
                    .expect("Failed to create graphics pipeline")[0]
            };

            unsafe {
                device.destroy_shader_module(vert_shader, None);
                device.destroy_shader_module(frag_shader, None);
            }

            pipeline
        };

        Self { layout, pipeline }
    }

    pub fn draw(&self, device: &ash::Device, cmd: vk::CommandBuffer, constants: PushConstants) {
        unsafe {
            device.cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, self.pipeline);

            device.cmd_push_constants(
                cmd,
                self.layout,
                vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                0,
                bytemuck::bytes_of(&constants),
            );
            const CHUNK_COUNT: u32 = 16;
            // TODO: Implement This For Individual Chunks
            // device.cmd_draw_indexed_indirect(cmd, buffer, 0, CHUNK_COUNT, stride);
        }
    }

    pub fn destroy(&mut self, device: &ash::Device) {
        unsafe {
            device.destroy_pipeline(self.pipeline, None);
            device.destroy_pipeline_layout(self.layout, None);
        }
    }
}
