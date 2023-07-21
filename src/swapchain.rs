#[derive(Clone)]
pub struct Swapchain {
    physical_device: ash::vk::PhysicalDevice,
    device: ash::Device,
    surface: crate::surface::Surface,

    pub inner: ash::vk::SwapchainKHR,
    pub loader: ash::extensions::khr::Swapchain,

    pub width: u32,
    pub height: u32,

    pub present_images: Vec<ash::vk::Image>,
    pub present_image_views: Vec<ash::vk::ImageView>,
}

impl Swapchain {
    pub fn new(
        instance: &ash::Instance,
        physical_device: &ash::vk::PhysicalDevice,
        device: &ash::Device,
        surface: &crate::surface::Surface,
        width: u32,
        height: u32,
    ) -> Swapchain {
        let loader = ash::extensions::khr::Swapchain::new(instance, device);

        let surface_info = surface.info(physical_device);
        let format = surface_info.formats[0];

        let mut desired_image_count = surface_info.capabilities.min_image_count + 1;
        if surface_info.capabilities.max_image_count > 0
            && desired_image_count > surface_info.capabilities.max_image_count
        {
            desired_image_count = surface_info.capabilities.max_image_count;
        }

        let surface_resolution = match surface_info.capabilities.current_extent.width {
            std::u32::MAX => ash::vk::Extent2D { width, height },
            _ => surface_info.capabilities.current_extent,
        };

        let pre_transform = if surface_info
            .capabilities
            .supported_transforms
            .contains(ash::vk::SurfaceTransformFlagsKHR::IDENTITY)
        {
            ash::vk::SurfaceTransformFlagsKHR::IDENTITY
        } else {
            surface_info.capabilities.current_transform
        };

        let present_mode = surface_info
            .present_modes
            .iter()
            .cloned()
            .find(|&mode| mode == ash::vk::PresentModeKHR::MAILBOX)
            .unwrap_or(ash::vk::PresentModeKHR::FIFO);

        let swapchain_create_info = ash::vk::SwapchainCreateInfoKHR::builder()
            .surface(surface.inner)
            .min_image_count(desired_image_count)
            .image_color_space(format.color_space)
            .image_format(format.format)
            .image_extent(surface_resolution)
            .image_usage(ash::vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(ash::vk::SharingMode::EXCLUSIVE)
            .pre_transform(pre_transform)
            .composite_alpha(ash::vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(present_mode)
            .clipped(true)
            .image_array_layers(1)
            .build();

        let inner = unsafe {
            loader
                .create_swapchain(&swapchain_create_info, None)
                .unwrap()
        };

        let present_images: Vec<ash::vk::Image> =
            unsafe { loader.get_swapchain_images(inner).unwrap() };
        let present_image_views: Vec<ash::vk::ImageView> = present_images
            .iter()
            .map(|&image| {
                let create_view_info = ash::vk::ImageViewCreateInfo::builder()
                    .view_type(ash::vk::ImageViewType::TYPE_2D)
                    .format(format.format)
                    .components(ash::vk::ComponentMapping {
                        r: ash::vk::ComponentSwizzle::R,
                        g: ash::vk::ComponentSwizzle::G,
                        b: ash::vk::ComponentSwizzle::B,
                        a: ash::vk::ComponentSwizzle::A,
                    })
                    .subresource_range(ash::vk::ImageSubresourceRange {
                        aspect_mask: ash::vk::ImageAspectFlags::COLOR,
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: 0,
                        layer_count: 1,
                    })
                    .image(image)
                    .build();
                unsafe { device.create_image_view(&create_view_info, None).unwrap() }
            })
            .collect();

        Swapchain {
            physical_device: physical_device.clone(),
            device: device.clone(),
            surface: surface.clone(),

            inner,
            loader,

            width,
            height,

            present_images,
            present_image_views,
        }
    }

    pub fn destroy(&mut self) {
        unsafe {
            self.present_image_views
                .iter()
                .for_each(|v| self.device.destroy_image_view(*v, None));
            
            self.loader.destroy_swapchain(self.inner, None);
        };
    }
    
    pub fn recreate(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;

        self.destroy();

        let surface_info = self.surface.info(&self.physical_device);
        let format = surface_info.formats[0];

        let mut desired_image_count = surface_info.capabilities.min_image_count + 1;
        if surface_info.capabilities.max_image_count > 0
            && desired_image_count > surface_info.capabilities.max_image_count
        {
            desired_image_count = surface_info.capabilities.max_image_count;
        }

        let surface_resolution = match surface_info.capabilities.current_extent.width {
            std::u32::MAX => ash::vk::Extent2D { width: self.width, height: self.height },
            _ => surface_info.capabilities.current_extent,
        };

        let pre_transform = if surface_info
            .capabilities
            .supported_transforms
            .contains(ash::vk::SurfaceTransformFlagsKHR::IDENTITY)
        {
            ash::vk::SurfaceTransformFlagsKHR::IDENTITY
        } else {
            surface_info.capabilities.current_transform
        };

        let present_mode = surface_info
            .present_modes
            .iter()
            .cloned()
            .find(|&mode| mode == ash::vk::PresentModeKHR::MAILBOX)
            .unwrap_or(ash::vk::PresentModeKHR::FIFO);

        let swapchain_create_info = ash::vk::SwapchainCreateInfoKHR::builder()
            .surface(self.surface.inner)
            .min_image_count(desired_image_count)
            .image_color_space(format.color_space)
            .image_format(format.format)
            .image_extent(surface_resolution)
            .image_usage(ash::vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(ash::vk::SharingMode::EXCLUSIVE)
            .pre_transform(pre_transform)
            .composite_alpha(ash::vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(present_mode)
            .clipped(true)
            .image_array_layers(1)
            .build();

        self.inner = unsafe {
            self.loader
                .create_swapchain(&swapchain_create_info, None)
                .unwrap()
        };

        self.present_images =
            unsafe { self.loader.get_swapchain_images(self.inner).unwrap() };
        self.present_image_views = self.present_images
            .iter()
            .map(|&image| {
                let create_view_info = ash::vk::ImageViewCreateInfo::builder()
                    .view_type(ash::vk::ImageViewType::TYPE_2D)
                    .format(format.format)
                    .components(ash::vk::ComponentMapping {
                        r: ash::vk::ComponentSwizzle::R,
                        g: ash::vk::ComponentSwizzle::G,
                        b: ash::vk::ComponentSwizzle::B,
                        a: ash::vk::ComponentSwizzle::A,
                    })
                    .subresource_range(ash::vk::ImageSubresourceRange {
                        aspect_mask: ash::vk::ImageAspectFlags::COLOR,
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: 0,
                        layer_count: 1,
                    })
                    .image(image)
                    .build();
                unsafe { self.device.create_image_view(&create_view_info, None).unwrap() }
            })
            .collect();
    }
}

// impl Drop for Swapchain