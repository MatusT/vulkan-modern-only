use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};

#[derive(Clone)]
pub struct Surface {
    pub inner: ash::vk::SurfaceKHR,
    pub loader: ash::extensions::khr::Surface,
}

pub struct SurfaceInfo {
    pub capabilities: ash::vk::SurfaceCapabilitiesKHR,
    pub formats: Vec<ash::vk::SurfaceFormatKHR>,
    pub present_modes: Vec<ash::vk::PresentModeKHR>,
}

impl Surface {
    pub fn new(
        entry: &ash::Entry,
        instance: &ash::Instance,
        window: &winit::window::Window,
    ) -> Surface {
        let inner = unsafe {
            ash_window::create_surface(
                &entry,
                &instance,
                window.raw_display_handle(),
                window.raw_window_handle(),
                None,
            )
            .unwrap()
        };

        let loader = ash::extensions::khr::Surface::new(&entry, &instance);

        Surface {
            inner,
            loader,
        }
    }

    pub fn info(&self, physical_device: &ash::vk::PhysicalDevice) -> SurfaceInfo {
        let formats = unsafe {
            self.loader
                .get_physical_device_surface_formats(*physical_device, self.inner)
                .unwrap()
        };

        let capabilities = unsafe {
            self.loader
                .get_physical_device_surface_capabilities(*physical_device, self.inner)
                .unwrap()
        };

        let present_modes = unsafe {
            self.loader
                .get_physical_device_surface_present_modes(*physical_device, self.inner)
                .unwrap()
        };

        SurfaceInfo { capabilities, formats, present_modes }

    }
}
