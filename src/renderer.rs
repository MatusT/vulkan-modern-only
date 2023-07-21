use ash::vk::{self, Buffer, CommandPoolResetFlags, BufferUsageFlags};
use gpu_allocator::{vulkan::*, MemoryLocation};
use bytemuck::{ Pod, Zeroable };
use glam;
pub struct BufferWithStaging {
    pub buffer: vk::Buffer,
    pub allocation: Allocation,

    pub staging_buffer: vk::Buffer,
    pub staging_allocation: Allocation,
}

impl BufferWithStaging {
    pub fn new(device_fn: &ash::Device, allocator: &mut Allocator) -> BufferWithStaging {
        unsafe {
            let buffer = device_fn
                .create_buffer(
                    &vk::BufferCreateInfo::builder()
                        .size(512)
                        .usage(vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_DST)
                        .build(),
                    None,
                )
                .expect("Could not create buffer.");

            let requirements = device_fn.get_buffer_memory_requirements(buffer);

            let allocation = allocator
                .allocate(&AllocationCreateDesc {
                    name: "Allocation",
                    requirements,
                    location: MemoryLocation::GpuOnly,
                    linear: true, // Buffers are always linear
                    allocation_scheme: AllocationScheme::GpuAllocatorManaged,
                })
                .unwrap();

            // Bind memory to the buffer
            device_fn
                .bind_buffer_memory(buffer, allocation.memory(), allocation.offset())
                .unwrap();

            let staging_buffer = device_fn
                .create_buffer(
                    &vk::BufferCreateInfo::builder()
                        .size(512)
                        .usage(vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_SRC)
                        .build(),
                    None,
                )
                .expect("Could not create buffer.");

            let requirements = device_fn.get_buffer_memory_requirements(buffer);

            let staging_allocation = allocator
                .allocate(&AllocationCreateDesc {
                    name: "Allocation",
                    requirements,
                    location: MemoryLocation::CpuToGpu,
                    linear: true, // Buffers are always linear
                    allocation_scheme: AllocationScheme::GpuAllocatorManaged,
                })
                .unwrap();

            // Bind memory to the buffer
            device_fn
                .bind_buffer_memory(staging_buffer, staging_allocation.memory(), staging_allocation.offset())
                .unwrap();

            BufferWithStaging {
                buffer,
                allocation,
                
                staging_buffer,
                staging_allocation,
            }
        }
    }
}

pub struct PerFrameData {
    pub command_pool: vk::CommandPool,

    pub image_available_semaphore: vk::Semaphore,
    pub render_finished_semaphore: vk::Semaphore,
    pub in_flight_fence: vk::Fence,

    pub test_buffer: BufferWithStaging,
}

impl PerFrameData {
    pub fn new(
        device_fn: &ash::Device,
        allocator: &mut Allocator,
        queue_family_index: u32,
    ) -> PerFrameData {
        let command_pool_create_info = vk::CommandPoolCreateInfo {
            flags: vk::CommandPoolCreateFlags::empty(),
            queue_family_index,
            ..Default::default()
        };

        let command_pool = unsafe {
            device_fn
                .create_command_pool(&command_pool_create_info, None)
                .expect("Could not create command pool.")
        };

        let image_available_semaphore = unsafe {
            device_fn
                .create_semaphore(&vk::SemaphoreCreateInfo::default(), None)
                .expect("")
        };

        let render_finished_semaphore = unsafe {
            device_fn
                .create_semaphore(&vk::SemaphoreCreateInfo::default(), None)
                .expect("")
        };

        let in_flight_fence = unsafe {
            device_fn
                .create_fence(
                    &vk::FenceCreateInfo {
                        flags: vk::FenceCreateFlags::SIGNALED,
                        ..Default::default()
                    },
                    None,
                )
                .expect("")
        };

        let test_buffer = BufferWithStaging::new(device_fn, allocator);

        PerFrameData {
            command_pool,

            image_available_semaphore,
            render_finished_semaphore,
            in_flight_fence,

            test_buffer,
        }
    }
}

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
struct Globals {
    pub transform: glam::Mat4,
}
pub struct Renderer {
    instance: ash::Instance,
    device: ash::Device,
    physical_device: vk::PhysicalDevice,
    queue: vk::Queue,

    allocator: Allocator,

    shader_object_loader: ash::extensions::ext::ShaderObject,
    shaders: crate::shaders::Shaders,

    frames_in_flight: usize,
    current_frame: usize,

    per_frame_data: Vec<PerFrameData>,
}

impl Renderer {
    pub fn new(
        instance: ash::Instance,
        physical_device: vk::PhysicalDevice,
        device: ash::Device,
        shader_object_loader: ash::extensions::ext::ShaderObject,
        queue: vk::Queue,
        queue_family_index: u32,
        frames_in_flight: usize,
    ) -> Renderer {
        let shaders = crate::shaders::Shaders::new(&device, &shader_object_loader);

        let mut allocator = Allocator::new(&AllocatorCreateDesc {
            instance: instance.clone(),
            device: device.clone(),
            physical_device,
            debug_settings: Default::default(),
            buffer_device_address: true, // Ideally, check the BufferDeviceAddressFeatures struct.
        })
        .expect("Could not create allocator");

        let mut per_frame_data = Vec::new();
        for i in 0..frames_in_flight {
            per_frame_data.push(PerFrameData::new(&device, &mut allocator, queue_family_index));
        }

        Renderer {
            instance,
            physical_device,
            device,
            queue,

            allocator,

            shader_object_loader,
            shaders,

            frames_in_flight,
            current_frame: 0,

            per_frame_data,
        }
    }

    pub fn destroy(&mut self) {
        unsafe {
            self.device.device_wait_idle().unwrap();

            // self.device.destroy_command_pool(self.command_pool, None);
        };

        // unsafe {
        //     for semaphore in self.image_available_semaphores.iter_mut() {
        //         self.device.destroy_semaphore(*semaphore, None);
        //     }

        //     for semaphore in self.render_finished_semaphores.iter_mut() {
        //         self.device.destroy_semaphore(*semaphore, None);
        //     }

        //     for fence in self.in_flight_fences.iter_mut() {
        //         self.device.destroy_fence(*fence, None);
        //     }
        // }
    }

    pub fn render(&mut self, swapchain: &mut crate::swapchain::Swapchain) -> bool {
        unsafe {
            let frame_data = &mut self.per_frame_data[self.current_frame];

            let in_flight_fence = frame_data.in_flight_fence;
            let render_finished_semaphore = frame_data.render_finished_semaphore;
            let image_available_semaphore = frame_data.image_available_semaphore;

            // Wait for previous work
            self.device
                .wait_for_fences(&[in_flight_fence], true, std::u64::MAX)
                .expect("");

            self.device.reset_fences(&[in_flight_fence]).expect("");

            // Copy over data
            let globals = Globals {
                transform: glam::Mat4::IDENTITY,
            };

            let buffer = &mut frame_data.test_buffer;
            let staging_map = buffer.staging_allocation.mapped_slice_mut().expect("Coult not map memory");
            
            {
                let (left, right) = staging_map.split_at_mut(64);

                left.copy_from_slice(bytemuck::bytes_of::<Globals>(&globals));
            }

            // Aquire Image
            let acquire_next_image = swapchain.loader.acquire_next_image(
                swapchain.inner,
                std::u64::MAX,
                image_available_semaphore,
                vk::Fence::null(),
            );

            let swapchain_image_index = match acquire_next_image {
                Ok((image_index, _)) => image_index,
                Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                    return true;
                }
                Err(error) => panic!("Error while acquiring next image. Cause: {}", error),
            };

            // Reset command pool
            self.device
                .reset_command_pool(
                    frame_data.command_pool,
                    CommandPoolResetFlags::RELEASE_RESOURCES,
                )
                .expect("");

            let allocate_command_buffer_create_info = vk::CommandBufferAllocateInfo {
                level: vk::CommandBufferLevel::PRIMARY,
                command_pool: frame_data.command_pool,
                command_buffer_count: 1,
                ..Default::default()
            };

            let command_buffer = self.device
                .allocate_command_buffers(&allocate_command_buffer_create_info)
                .expect("Could not allocate command buffers.")[0];

            // Record command buffers
            let shaders = &self.shaders.shaders;
            let command_buffer_begin_info = vk::CommandBufferBeginInfo::builder().build();

            self.device
                .begin_command_buffer(command_buffer, &command_buffer_begin_info)
                .expect("Could not start command buffer recoring.");

            // HERE GO RENDER COMMANDS
            let image_memory_barrier = vk::ImageMemoryBarrier::builder()
                .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
                .old_layout(vk::ImageLayout::UNDEFINED)
                .new_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                .image(swapchain.present_images[swapchain_image_index as usize])
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    base_array_layer: 0,
                    level_count: 1,
                    layer_count: 1,
                })
                .build();

            self.device.cmd_pipeline_barrier(
                command_buffer,
                vk::PipelineStageFlags::TOP_OF_PIPE,
                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[image_memory_barrier],
            );

            let rendering_attachment_infos = vec![vk::RenderingAttachmentInfo::builder()
                .image_view(swapchain.present_image_views[swapchain_image_index as usize])
                .image_layout(vk::ImageLayout::ATTACHMENT_OPTIMAL)
                .load_op(vk::AttachmentLoadOp::CLEAR)
                .store_op(vk::AttachmentStoreOp::STORE)
                .clear_value(vk::ClearValue {
                    color: vk::ClearColorValue {
                        float32: [0.5, 0.5, 0.5, 1.0],
                    },
                })
                .build()];

            let rendering_info = vk::RenderingInfo::builder()
                .render_area(vk::Rect2D {
                    offset: vk::Offset2D::default(),
                    extent: vk::Extent2D {
                        width: swapchain.width,
                        height: swapchain.height,
                    },
                })
                .layer_count(1)
                .color_attachments(&rendering_attachment_infos)
                .build();

            self.device
                .cmd_begin_rendering(command_buffer, &rendering_info);

            self.device.cmd_set_viewport_with_count(
                command_buffer,
                &[vk::Viewport {
                    width: swapchain.width as f32,
                    height: swapchain.height as f32,
                    ..Default::default()
                }],
            );
            self.device.cmd_set_scissor_with_count(
                command_buffer,
                &[vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: 0 },
                    extent: vk::Extent2D {
                        width: swapchain.width as u32,
                        height: swapchain.height as u32,
                    },
                }],
            );
            self.shader_object_loader.cmd_bind_shaders(
                command_buffer,
                &[vk::ShaderStageFlags::VERTEX, vk::ShaderStageFlags::FRAGMENT],
                &[shaders[0], shaders[1]],
            );
            self.shader_object_loader
                .cmd_set_primitive_topology(command_buffer, vk::PrimitiveTopology::TRIANGLE_LIST);

            self.device.cmd_draw(command_buffer, 3, 1, 0, 0);

            self.device.cmd_end_rendering(command_buffer);

            let image_memory_barrier = vk::ImageMemoryBarrier::builder()
                .src_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
                .old_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                .new_layout(vk::ImageLayout::PRESENT_SRC_KHR)
                .image(swapchain.present_images[swapchain_image_index as usize])
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    base_array_layer: 0,
                    level_count: 1,
                    layer_count: 1,
                })
                .build();

            self.device.cmd_pipeline_barrier(
                command_buffer,
                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[image_memory_barrier],
            );

            self.device
                .end_command_buffer(command_buffer)
                .expect("Could not end command buffer recording.");

            // Submit
            let queue_submits = vec![vk::SubmitInfo::builder()
                .command_buffers(&[command_buffer])
                .wait_semaphores(&[image_available_semaphore])
                .wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT])
                .signal_semaphores(&[render_finished_semaphore])
                .build()];

            self.device
                .queue_submit(
                    self.queue,
                    &queue_submits,
                    in_flight_fence,
                )
                .expect("");

            // Present
            let present_info = vk::PresentInfoKHR::builder()
                .wait_semaphores(&[render_finished_semaphore])
                .swapchains(&[swapchain.inner])
                .image_indices(&[swapchain_image_index])
                .build();

            let present_result = swapchain.loader.queue_present(self.queue, &present_info);

            match present_result {
                Ok(true) | Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                    return true;
                }
                Err(error) => panic!("Failed to present queue. Cause: {}", error),
                _ => {}
            }
        };

        self.current_frame = (self.current_frame + 1) % self.frames_in_flight;

        return false;
    }
}
