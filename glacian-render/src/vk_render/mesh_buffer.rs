use crate::vk_render::allocations::AllocatedBuffer;
use ash::vk;
use bytemuck::NoUninit;

pub struct GPUMeshBuffers {
    pub index_buffer: AllocatedBuffer,
    pub vertex_buffer: AllocatedBuffer,
    pub vertex_buffer_address: vk::DeviceAddress,
}

impl GPUMeshBuffers {
    pub fn new<V: Sized + NoUninit>(
        indices: &[u32],
        vertices: &[V],
        allocator: &vk_mem::Allocator,
        device: &ash::Device,
        transfer_queue: &vk::Queue,
    ) -> Self {
        let vertex_buffer_size = (vertices.len() * size_of::<V>()) as u64;
        let index_buffer_size = (indices.len() * size_of::<u32>()) as u64;
        let total_size = vertex_buffer_size + index_buffer_size;

        let vertex_buffer = AllocatedBuffer::new(
            allocator,
            vertex_buffer_size,
            vk::BufferUsageFlags::STORAGE_BUFFER
                | vk::BufferUsageFlags::TRANSFER_DST
                | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
            vk_mem::MemoryUsage::AutoPreferDevice,
        );

        let device_addr_info = vk::BufferDeviceAddressInfo::default().buffer(vertex_buffer.buf);
        let vertex_buffer_address = unsafe { device.get_buffer_device_address(&device_addr_info) };

        let index_buffer = AllocatedBuffer::new(
            allocator,
            index_buffer_size,
            vk::BufferUsageFlags::INDEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
            vk_mem::MemoryUsage::AutoPreferDevice,
        );
        let mut staging = AllocatedBuffer::new(
            allocator,
            total_size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk_mem::MemoryUsage::AutoPreferHost,
        );

        let mem = unsafe { allocator.map_memory(&mut staging.allocation) }.unwrap();

        let data_slice: &mut [u8] =
            unsafe { std::slice::from_raw_parts_mut(mem as *mut u8, total_size as usize) };

        let vertex_bytes = bytemuck::cast_slice(vertices);
        data_slice[0..(vertex_buffer_size as usize)].copy_from_slice(vertex_bytes);

        let index_bytes = bytemuck::cast_slice(indices);
        data_slice[(vertex_buffer_size as usize)..total_size as usize].copy_from_slice(index_bytes);

        unsafe { allocator.unmap_memory(&mut staging.allocation) };

        unsafe {
            let submit_fence = device
                .create_fence(&vk::FenceCreateInfo::default(), None)
                .unwrap();
            let cmd_pool = device
                .create_command_pool(&vk::CommandPoolCreateInfo::default(), None)
                .unwrap();
            let cmd = device
                .allocate_command_buffers(
                    &vk::CommandBufferAllocateInfo::default()
                        .command_pool(cmd_pool)
                        .level(vk::CommandBufferLevel::PRIMARY)
                        .command_buffer_count(1),
                )
                .unwrap()[0];
            device
                .begin_command_buffer(
                    cmd,
                    &vk::CommandBufferBeginInfo::default()
                        .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT),
                )
                .unwrap();

            device.cmd_copy_buffer(
                cmd,
                staging.buf,
                vertex_buffer.buf,
                &[vk::BufferCopy::default().size(vertex_buffer_size)],
            );
            device.cmd_copy_buffer(
                cmd,
                staging.buf,
                index_buffer.buf,
                &[vk::BufferCopy::default()
                    .size(index_buffer_size)
                    .src_offset(vertex_buffer_size)],
            );

            device.end_command_buffer(cmd).unwrap();

            device
                .queue_submit2(
                    *transfer_queue,
                    &[vk::SubmitInfo2::default().command_buffer_infos(&[
                        vk::CommandBufferSubmitInfo::default()
                            .command_buffer(cmd)
                            .device_mask(0),
                    ])],
                    submit_fence,
                )
                .unwrap();
            device
                .wait_for_fences(&[submit_fence], true, 999999999)
                .unwrap();

            device.destroy_command_pool(cmd_pool, None);
            device.destroy_fence(submit_fence, None);
        }

        staging.flush(allocator);

        Self {
            index_buffer,
            vertex_buffer,
            vertex_buffer_address,
        }
    }
}
