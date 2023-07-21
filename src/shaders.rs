use std::{ffi::CString, fs};

use ash::vk;
use hassle_rs::compile_hlsl;

pub struct Shaders {
    pub shaders: Vec<vk::ShaderEXT>,
}

impl Shaders {
    pub fn new(
        device: &ash::Device,
        shader_object_loader: &ash::extensions::ext::ShaderObject,
    ) -> Shaders {
        unsafe {
            let code_hlsl = fs::read_to_string("./shaders/triangle.hlsl")
                .expect("Should have been able to read the file");

            let vertex_spirv = compile_hlsl(
                "shaders/triangle.hlsl",
                &code_hlsl,
                "vertexMain",
                "vs_6_6",
                &vec!["-spirv"],
                &vec![],
            ).expect("Should have been able to compile the shader");
            let vertex_name = CString::new("vertexMain").expect("CString::new failed");
            let vertex = vk::ShaderCreateInfoEXT::builder()
                .stage(vk::ShaderStageFlags::VERTEX)
                .flags(vk::ShaderCreateFlagsEXT::LINK_STAGE)
                .next_stage(vk::ShaderStageFlags::FRAGMENT)
                .code_type(vk::ShaderCodeTypeEXT::SPIRV)
                .code(&vertex_spirv)
                .name(&vertex_name)
                .build();

            let fragment_spirv = compile_hlsl(
                "shaders/triangle.hlsl",
                &code_hlsl,
                "pixelMain",
                "ps_6_6",
                &vec!["-spirv"],
                &vec![],
            ).expect("Should have been able to compile the shader");
            let fragment_name = CString::new("pixelMain").expect("CString::new failed");
            let fragment = vk::ShaderCreateInfoEXT::builder()
                .stage(vk::ShaderStageFlags::FRAGMENT)
                .next_stage(vk::ShaderStageFlags::empty())
                .flags(vk::ShaderCreateFlagsEXT::LINK_STAGE)
                .code_type(vk::ShaderCodeTypeEXT::SPIRV)
                .code(&fragment_spirv)
                .name(&fragment_name)
                .build();

            let shaders = shader_object_loader
                .create_shaders(&[vertex, fragment], None)
                .expect("Could not compile shaders");

            Shaders { shaders }
        }
    }
}
