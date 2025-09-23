use ash::vk;

pub fn copy_image_to_image(
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

pub fn transition_image(
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

pub fn load_shader_module(path: &str, device: &ash::Device) -> vk::ShaderModule {
    use std::io::Read;
    let mut file = std::fs::File::open(path).unwrap();
    let mut buf = Vec::new();
    _ = file.read_to_end(&mut buf);

    let create_info = vk::ShaderModuleCreateInfo::default().code(bytemuck::cast_slice(&buf));
    unsafe { device.create_shader_module(&create_info, None) }.unwrap()
}
