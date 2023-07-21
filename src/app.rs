use std::ffi::{c_char, CStr};

use ash::{
    extensions::ext::DebugUtils,
    prelude::VkResult,
    vk::{self, DebugUtilsMessengerEXT},
    Device, Entry,
};
use raw_window_handle::HasRawDisplayHandle;
use winit::window::Window;

use anyhow::{Context, Result};

use crate::{debug::vulkan_debug_callback, requirements_filters::is_device_suitable};
pub struct App {
    pub entry: ash::Entry,
    pub instance: ash::Instance,
    pub device: ash::Device,

    pub debug_utils_loader: DebugUtils,
    pub debug_call_back: DebugUtilsMessengerEXT,

    pub surface: crate::surface::Surface,
    pub swapchain: crate::swapchain::Swapchain,

    pub queue: ash::vk::Queue,

    pub renderer: crate::renderer::Renderer,

    width: u32,
    height: u32,
}

impl App {
    pub fn new(window: &Window, window_width: u32, window_height: u32) -> Result<Self> {
        let entry = Entry::linked();

        let app_name = unsafe { CStr::from_bytes_with_nul_unchecked(b"VulkanTriangle\0") };

        let layer_names = unsafe {
            [CStr::from_bytes_with_nul_unchecked(
                b"VK_LAYER_KHRONOS_validation\0",
            )]
        };
        let layers_names_raw: Vec<*const c_char> = layer_names
            .iter()
            .map(|raw_name| raw_name.as_ptr())
            .collect();

        let mut extension_names =
            ash_window::enumerate_required_extensions(window.raw_display_handle())
                .unwrap()
                .to_vec();
        extension_names.push(DebugUtils::name().as_ptr());

        let appinfo = vk::ApplicationInfo::builder()
            .application_name(app_name)
            .application_version(0)
            .engine_name(app_name)
            .engine_version(0)
            .api_version(vk::make_api_version(0, 1, 3, 0))
            .build();

        let create_info = vk::InstanceCreateInfo::builder()
            .application_info(&appinfo)
            .enabled_layer_names(&layers_names_raw)
            .enabled_extension_names(&extension_names)
            .flags(vk::InstanceCreateFlags::default())
            .build();

        let instance = unsafe { entry.create_instance(&create_info, None)? };

        let debug_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
            .message_severity(
                vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                    | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                    | vk::DebugUtilsMessageSeverityFlagsEXT::INFO,
            )
            .message_type(
                vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                    | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                    | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
            )
            .pfn_user_callback(Some(vulkan_debug_callback))
            .build();

        let debug_utils_loader = DebugUtils::new(&entry, &instance);
        let debug_call_back = unsafe {
            debug_utils_loader
                .create_debug_utils_messenger(&debug_info, None)
                .unwrap()
        };

        let physical_devices: Vec<vk::PhysicalDevice> =
            unsafe { instance.enumerate_physical_devices()? };

        let surface = crate::surface::Surface::new(&entry, &instance, &window);

        let (physical_device, queue_family_index) = unsafe {
            physical_devices
                .iter()
                .find_map(|physical_device| {
                    if let Some(index) = is_device_suitable(
                        &instance,
                        &surface.loader,
                        &surface.inner,
                        *physical_device,
                    ) {
                        return Some((physical_device, index as u32));
                    } else {
                        return None;
                    }
                })
                .with_context(|| "")?
        };

        let device_extension_names_raw = [
            ash::extensions::khr::Swapchain::name().as_ptr(),
            ash::extensions::khr::DynamicRendering::name().as_ptr(),
            ash::extensions::ext::ShaderObject::name().as_ptr(),
            ash::extensions::khr::BufferDeviceAddress::name().as_ptr(),
        ];
        let features = vk::PhysicalDeviceFeatures::default();
        let priorities = [1.0];

        let queue_info = vk::DeviceQueueCreateInfo::builder()
            .queue_family_index(queue_family_index as u32)
            .queue_priorities(&priorities)
            .build();

        let mut dynamic_rendering_features = vk::PhysicalDeviceDynamicRenderingFeatures::builder()
            .dynamic_rendering(true)
            .build();

        let mut shader_object_features = vk::PhysicalDeviceShaderObjectFeaturesEXT::builder()
            .shader_object(true)
            .build();

        let mut buffer_device_address = vk::PhysicalDeviceBufferDeviceAddressFeaturesKHR::builder()
            .buffer_device_address(true)
            .build();

        let device_create_info = vk::DeviceCreateInfo::builder()
            .queue_create_infos(std::slice::from_ref(&queue_info))
            .enabled_extension_names(&device_extension_names_raw)
            .enabled_features(&features)
            .push_next(&mut shader_object_features)
            .push_next(&mut dynamic_rendering_features)
            .push_next(&mut buffer_device_address)
            .build();

        let device: Device =
            unsafe { instance.create_device(*physical_device, &device_create_info, None)? };
        let _dynamic_rendering_loader =
            ash::extensions::khr::DynamicRendering::new(&instance, &device);
        let shader_object_loader = ash::extensions::ext::ShaderObject::new(&instance, &device);

        let swapchain = crate::swapchain::Swapchain::new(
            &instance,
            &physical_device,
            &device,
            &surface,
            window_width,
            window_height,
        );
        let queue = unsafe { device.get_device_queue(queue_family_index, 0) };

        let renderer = crate::renderer::Renderer::new(
            instance.clone(),
            physical_device.clone(),
            device.clone(),
            shader_object_loader,
            queue,
            queue_family_index,
            2,
        );

        Ok(App {
            entry,
            instance,
            device,

            debug_utils_loader,
            debug_call_back,

            surface,
            swapchain,
            queue,

            renderer,

            width: window_width,
            height: window_height,
        })
    }

    pub fn render(&mut self) -> bool {
        return self.renderer.render(&mut self.swapchain);
    }

    pub fn wait_gpu_idle(&self) {
        unsafe { self.device.device_wait_idle().unwrap() };
    }

    pub fn recreate_swapchain(&mut self, width: u32, height: u32) {
        self.wait_gpu_idle();

        self.width = width;
        self.height = height;

        self.swapchain.recreate(width, height);
    }
}

impl Drop for App {
    fn drop(&mut self) {
        unsafe {
            self.device.device_wait_idle().unwrap();

            self.swapchain.destroy();
            self.renderer.destroy();

            self.device.destroy_device(None);
            self.surface
                .loader
                .destroy_surface(self.surface.inner, None);
            self.debug_utils_loader
                .destroy_debug_utils_messenger(self.debug_call_back, None);
            self.instance.destroy_instance(None);
        }
    }
}
