use std::ffi::{CString, CStr, c_char};

use ash::vk::{self, SurfaceKHR, MAX_EXTENSION_NAME_SIZE, MAX_PHYSICAL_DEVICE_NAME_SIZE};

pub unsafe fn is_device_suitable(
    instance: &ash::Instance,
    surface_loader: &ash::extensions::khr::Surface,
    surface: &SurfaceKHR,
    physical_device: vk::PhysicalDevice,
) -> Option<usize> {
    let mut properties = vk::PhysicalDeviceProperties2::default();
    instance.get_physical_device_properties2(physical_device, &mut properties);

    let mut features = vk::PhysicalDeviceFeatures2::default();
    instance.get_physical_device_features2(physical_device, &mut features);

    let available_extensions = instance.enumerate_device_extension_properties(physical_device).expect("Could not iterate device extensions");
    let available_extensions = available_extensions.into_iter().map(|e| unsafe {
        CStr::from_bytes_until_nul(&std::mem::transmute::<[c_char; MAX_EXTENSION_NAME_SIZE], [u8; MAX_EXTENSION_NAME_SIZE]>(e.extension_name)).unwrap().to_owned()
    }).collect::<Vec<_>>();

    let device_name = CStr::from_bytes_until_nul(&std::mem::transmute::<[c_char; MAX_PHYSICAL_DEVICE_NAME_SIZE], [u8; MAX_PHYSICAL_DEVICE_NAME_SIZE ]>(properties.properties.device_name)).unwrap().to_owned();
    println!("{:?} {:?}", device_name, ash::extensions::ext::ShaderObject::name());
    println!("{:?}", available_extensions);
    println!("{:?}", available_extensions.contains(&ash::extensions::ext::ShaderObject::name().to_owned()));

    let contains_dynamic_rendering = available_extensions.contains(&ash::extensions::khr::DynamicRendering::name().to_owned());
    let contains_shader_object = available_extensions.contains(&ash::extensions::ext::ShaderObject::name().to_owned());

    // if !contains_shader_object || !contains_dynamic_rendering {
    //     return None;
    // }

    let queue_family_index = instance
        .get_physical_device_queue_family_properties(physical_device)
        .iter()
        .enumerate()
        .find_map(|(index, info)| {
            let supports_graphic_and_surface = info
                .queue_flags
                .contains(vk::QueueFlags::GRAPHICS)
                && surface_loader
                    .get_physical_device_surface_support(physical_device, index as u32, *surface)
                    .unwrap();
            if supports_graphic_and_surface {
                Some(index)
            } else {
                None
            }
        });

    return queue_family_index;
}
