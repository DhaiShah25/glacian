use ash::vk;
use vk_mem::Alloc;

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

#[derive(Debug)]
pub struct AllocatedImage {
    pub image: vk::Image,
    pub view: vk::ImageView,
    pub allocation: vk_mem::Allocation,
    pub extent: vk::Extent3D,
    pub format: vk::Format,
}

impl AllocatedImage {
    pub fn new(
        format: vk::Format,
        usage_flags: vk::ImageUsageFlags,
        extent: vk::Extent3D,
        aspect_flags: vk::ImageAspectFlags,
        allocator: &vk_mem::Allocator,
        device: &ash::Device,
    ) -> Self {
        let img_create_info = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .format(format)
            .extent(extent)
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(usage_flags);

        let (image, allocation) = unsafe {
            allocator.create_image(
                &img_create_info,
                &vk_mem::AllocationCreateInfo {
                    usage: vk_mem::MemoryUsage::AutoPreferDevice,
                    required_flags: vk::MemoryPropertyFlags::DEVICE_LOCAL,
                    ..Default::default()
                },
            )
        }
        .unwrap();

        let img_view_create_info = vk::ImageViewCreateInfo::default()
            .view_type(vk::ImageViewType::TYPE_2D)
            .image(image)
            .format(format)
            .subresource_range(
                vk::ImageSubresourceRange::default()
                    .base_mip_level(0)
                    .level_count(1)
                    .base_array_layer(0)
                    .layer_count(1)
                    .aspect_mask(aspect_flags),
            );

        let view = unsafe { device.create_image_view(&img_view_create_info, None) }.unwrap();

        Self {
            image,
            view,
            allocation,
            extent,
            format,
        }
    }

    pub fn flush(&mut self, device: &ash::Device, alloc: &vk_mem::Allocator) {
        unsafe {
            alloc.destroy_image(self.image, &mut self.allocation);
            device.destroy_image_view(self.view, None);
        }
    }
}

pub struct DelQueue<'a> {
    deletors: std::collections::VecDeque<Box<dyn FnOnce() + 'a>>,
}

impl<'a> DelQueue<'a> {
    pub const fn new() -> Self {
        Self {
            deletors: std::collections::VecDeque::new(),
        }
    }

    pub fn add(&mut self, func: Box<dyn FnOnce() + 'a>) {
        self.deletors.push_back(func);
    }

    pub fn flush(&mut self) {
        for del in self.deletors.drain(..).rev() {
            del()
        }
    }
}

pub struct DescriptorLayoutBuilder<'a> {
    bindings: Vec<vk::DescriptorSetLayoutBinding<'a>>,
}

impl<'a> DescriptorLayoutBuilder<'a> {
    pub const fn new() -> Self {
        Self { bindings: vec![] }
    }

    pub fn add_binding(&mut self, binding: u32, descriptor_type: vk::DescriptorType) {
        self.bindings.push(
            vk::DescriptorSetLayoutBinding::default()
                .binding(binding)
                .descriptor_count(1)
                .descriptor_type(descriptor_type),
        );
    }

    pub fn clear(&mut self) {
        self.bindings.clear();
    }

    pub fn build(
        &mut self,
        device: &ash::Device,
        shader_stages: vk::ShaderStageFlags,
        flags: vk::DescriptorSetLayoutCreateFlags,
    ) -> vk::DescriptorSetLayout {
        self.bindings
            .iter_mut()
            .for_each(|binding| binding.stage_flags |= shader_stages);

        let info = vk::DescriptorSetLayoutCreateInfo::default()
            .bindings(&self.bindings)
            .flags(flags);

        unsafe { device.create_descriptor_set_layout(&info, None) }.unwrap()
    }
}

pub struct DescriptorAllocator {
    pool: vk::DescriptorPool,
}

impl DescriptorAllocator {
    pub fn new(
        device: &ash::Device,
        max_sets: u32,
        pool_ratios: &[(vk::DescriptorType, u32)],
    ) -> Self {
        let pool_sizes: Vec<vk::DescriptorPoolSize> = pool_ratios
            .iter()
            .map(|ratio| {
                vk::DescriptorPoolSize::default()
                    .descriptor_count(ratio.1 * max_sets)
                    .ty(ratio.0)
            })
            .collect();

        let pool_info = vk::DescriptorPoolCreateInfo::default()
            .max_sets(max_sets)
            .pool_sizes(&pool_sizes);

        Self {
            pool: unsafe { device.create_descriptor_pool(&pool_info, None) }.unwrap(),
        }
    }

    pub fn clear_descriptors(&mut self, device: &ash::Device) {
        unsafe { device.reset_descriptor_pool(self.pool, vk::DescriptorPoolResetFlags::empty()) }
            .unwrap();
    }

    pub fn flush(&mut self, device: &ash::Device) {
        unsafe {
            device.destroy_descriptor_pool(self.pool, None);
        };
    }

    pub fn allocate(
        &self,
        device: &ash::Device,
        layout: vk::DescriptorSetLayout,
    ) -> vk::DescriptorSet {
        let layout = [layout];
        unsafe {
            device.allocate_descriptor_sets(
                &vk::DescriptorSetAllocateInfo::default()
                    .descriptor_pool(self.pool)
                    .set_layouts(&layout),
            )
        }
        .unwrap()[0]
    }
}

pub struct AllocatedBuffer {
    pub buf: vk::Buffer,
    pub allocation: vk_mem::Allocation,
}

impl AllocatedBuffer {
    pub fn new(
        allocator: &vk_mem::Allocator,
        size: u64,
        usage: vk::BufferUsageFlags,
        memory_usage: vk_mem::MemoryUsage,
    ) -> Self {
        let info = vk::BufferCreateInfo::default().size(size).usage(usage);
        let alloc_create_info = vk_mem::AllocationCreateInfo {
            usage: memory_usage,
            flags: vk_mem::AllocationCreateFlags::MAPPED
                | vk_mem::AllocationCreateFlags::HOST_ACCESS_SEQUENTIAL_WRITE,
            ..Default::default()
        };

        let (buf, allocation) =
            unsafe { allocator.create_buffer(&info, &alloc_create_info) }.unwrap();

        Self { buf, allocation }
    }

    pub fn flush(&mut self, allocator: &vk_mem::Allocator) {
        unsafe { allocator.destroy_buffer(self.buf, &mut self.allocation) };
    }
}

#[repr(C, packed)]
#[derive(Clone, Copy, bytemuck::Zeroable, bytemuck::Pod, Default)]
pub struct Vertex {
    pub position: glam::Vec3,
    pub uv_x: f32,
    pub normal: glam::Vec3,
    pub uv_y: f32,
    pub color: glam::Vec4,
}

pub struct GPUMeshBuffers {
    pub index_buffer: AllocatedBuffer,
    pub vertex_buffer: AllocatedBuffer,
    pub vertex_buffer_address: vk::DeviceAddress,
}

impl GPUMeshBuffers {
    pub fn new(
        indices: &[u32],
        vertices: &[Vertex],
        allocator: &vk_mem::Allocator,
        device: &ash::Device,
        transfer_queue: &vk::Queue,
    ) -> Self {
        let vertex_buffer_size = (vertices.len() * size_of::<Vertex>()) as u64;
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

#[repr(C, packed)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Copy, Clone)]
pub struct GPUDrawPushConstants {
    // TODO: Research Affine Transformations
    pub world_matrix: glam::Mat4,
    pub vertex_buffer: vk::DeviceAddress,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, bytemuck::Zeroable, bytemuck::Pod)]
pub struct SkyPushConstants {
    time_of_day_yaw: u32,
    pitch_and_padding: u32,
}

impl SkyPushConstants {
    pub fn new(time_of_day: u16, yaw: u16, pitch: u8) -> Self {
        // Pack time_of_day_u16 into the lower 16 bits and yaw_u16 into the upper 16 bits
        let time_of_day_yaw_packed = (time_of_day as u32) | ((yaw as u32) << 16);

        // Pitch is a single u8. Place it in the lowest byte of a u32.
        // The remaining bytes of `pitch_and_padding` implicitly become padding,
        // ensuring 4-byte alignment for this member.
        let pitch_and_padding_packed = pitch as u32;

        Self {
            time_of_day_yaw: time_of_day_yaw_packed,
            pitch_and_padding: pitch_and_padding_packed,
        }
    }
}
