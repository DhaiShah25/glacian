use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use tracing::{debug, error, info, trace, warn};

use ash::vk;
use minifb::{Key, Window, WindowOptions};

const WIDTH: usize = 640;
const HEIGHT: usize = 360;

fn main() {
    tracing_subscriber::FmtSubscriber::builder()
        .with_file(true)
        .with_level(true)
        .with_ansi(true)
        .with_line_number(true)
        .without_time()
        .init();

    let mut buffer: Vec<u32> = vec![0; WIDTH * HEIGHT];

    let entry = unsafe { ash::Entry::load() }.unwrap();

    if let Some(version) = unsafe { entry.try_enumerate_instance_version() }.unwrap() {
        let major = vk::api_version_major(version);
        let minor = vk::api_version_minor(version);
        let patch = vk::api_version_patch(version);
        info!("Running Vulkan Version: {}.{}.{}", major, minor, patch);
    }

    let app_info = vk::ApplicationInfo::default()
        .api_version(vk::make_api_version(0, 1, 3, 0))
        .application_name(c"Shadow");

    let mut instance_extensions = vec![ash::khr::surface::NAME.as_ptr()];

    #[cfg(feature = "debug")]
    instance_extensions.push(ash::ext::debug_utils::NAME.as_ptr());

    #[cfg(target_os = "linux")]
    instance_extensions.push(ash::khr::wayland_surface::NAME.as_ptr());
    #[cfg(target_os = "macos")]
    {
        extensions.push(ash::khr::portability_enumeration::NAME.as_ptr());
        extensions.push(ash::khr::portability_subset::NAME.as_ptr());
    }
    #[cfg(target_os = "windows")]
    let extensions = vec![c"VK_KHR_surface".as_ptr()];

    let mut window = Window::new(
        "Shadow Engine",
        WIDTH,
        HEIGHT,
        WindowOptions {
            borderless: true,
            resize: true,
            ..Default::default()
        },
    )
    .unwrap();

    #[cfg(feature = "debug")]
    let instance_layers = vec![c"VK_LAYER_KHRONOS_validation".as_ptr()];
    #[cfg(not(feature = "debug"))]
    let instance_layers = vec![];

    #[cfg(feature = "debug")]
    let mut debugcreateinfo = vk::DebugUtilsMessengerCreateInfoEXT {
        message_severity: vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
            | vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE
            | vk::DebugUtilsMessageSeverityFlagsEXT::INFO
            | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
        message_type: vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
            | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE
            | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION,
        pfn_user_callback: Some(debug_callback),
        ..Default::default()
    };

    let create_info = vk::InstanceCreateInfo::default()
        .application_info(&app_info)
        .enabled_extension_names(&instance_extensions)
        .enabled_layer_names(&instance_layers);

    #[cfg(feature = "debug")]
    let create_info = create_info.push_next(&mut debugcreateinfo);

    let instance = unsafe { entry.create_instance(&create_info, None) }.unwrap();

    #[cfg(feature = "debug")]
    let (debug_utils, utils_messenger) = {
        let debug_utils = ash::ext::debug_utils::Instance::new(&entry, &instance);
        let utils_messenger = unsafe {
            debug_utils
                .create_debug_utils_messenger(&debugcreateinfo, None)
                .unwrap()
        };
        (debug_utils, utils_messenger)
    };

    let devices = unsafe { instance.enumerate_physical_devices() }.unwrap();
    let physical_device = *devices
        .iter()
        .find(|device| {
            unsafe { instance.get_physical_device_properties(**device) }.device_type
                == vk::PhysicalDeviceType::DISCRETE_GPU
        })
        .unwrap_or_else(|| {
            devices
                .iter()
                .find(|device| {
                    unsafe { instance.get_physical_device_properties(**device) }.device_type
                        == vk::PhysicalDeviceType::INTEGRATED_GPU
                })
                .unwrap()
        });

    let mut device_extensions = vec![
        ash::khr::dynamic_rendering::NAME.as_ptr(),
        ash::khr::swapchain::NAME.as_ptr(),
    ];

    #[cfg(feature = "debug")]
    device_extensions.push(ash::khr::line_rasterization::NAME.as_ptr());

    let mut surface = None;
    let mut display = None;

    if let raw_window_handle::RawWindowHandle::Wayland(h) = window.window_handle().unwrap().as_raw()
    {
        surface = Some(h.surface);
    };
    if let raw_window_handle::RawDisplayHandle::Wayland(h) =
        window.display_handle().unwrap().as_raw()
    {
        display = Some(h.display);
    };

    let surface_create_info = vk::WaylandSurfaceCreateInfoKHR::default()
        .display(display.unwrap().as_ptr())
        .surface(surface.unwrap().as_ptr());

    let wayland_surface_loader = ash::khr::wayland_surface::Instance::new(&entry, &instance);
    let surface =
        unsafe { wayland_surface_loader.create_wayland_surface(&surface_create_info, None) }
            .unwrap();
    let surface_loader = ash::khr::surface::Instance::new(&entry, &instance);

    let queuefamilyproperties =
        unsafe { instance.get_physical_device_queue_family_properties(physical_device) };
    dbg!(&queuefamilyproperties);
    let qfamindices = {
        let mut found_graphics_q_index = None;
        for (index, qfam) in queuefamilyproperties.iter().enumerate() {
            if qfam.queue_count > 0
                && qfam.queue_flags.contains(vk::QueueFlags::GRAPHICS)
                && unsafe {
                    surface_loader
                        .get_physical_device_surface_support(physical_device, index as u32, surface)
                        .unwrap()
                }
            {
                found_graphics_q_index = Some(index as u32);
            }
        }
        found_graphics_q_index.unwrap()
    };

    let queue_create_info = vk::DeviceQueueCreateInfo::default()
        .queue_priorities(&[1.])
        .queue_family_index(qfamindices);

    let device_features = vk::PhysicalDeviceFeatures::default();

    let surface_capabilities = unsafe {
        surface_loader.get_physical_device_surface_capabilities(physical_device, surface)
    };
    let surface_present_modes = unsafe {
        surface_loader.get_physical_device_surface_present_modes(physical_device, surface)
    };
    let surface_formats =
        unsafe { surface_loader.get_physical_device_surface_formats(physical_device, surface) };
    dbg!(&surface_capabilities);
    dbg!(&surface_present_modes);
    dbg!(&surface_formats);

    _ = dbg!(unsafe { instance.get_physical_device_queue_family_properties(physical_device) });

    let device = unsafe {
        instance
            .create_device(
                physical_device,
                &vk::DeviceCreateInfo {
                    enabled_extension_count: device_extensions.len() as u32,
                    pp_enabled_extension_names: device_extensions.as_ptr(),
                    queue_create_info_count: 1,
                    p_queue_create_infos: &queue_create_info,
                    p_enabled_features: std::ptr::from_ref(&device_features),
                    ..Default::default()
                },
                None,
            )
            .unwrap()
    };

    let graphics_queue = unsafe { device.get_device_queue(qfamindices, 0) };

    window.set_target_fps(60);

    while window.is_open() && !window.is_key_down(Key::Escape) {
        for i in &mut buffer {
            *i = 255 + 255 + 255;
        }

        window.update_with_buffer(&buffer, WIDTH, HEIGHT).unwrap();
    }

    unsafe {
        surface_loader.destroy_surface(surface, None);
        #[cfg(feature = "debug")]
        debug_utils.destroy_debug_utils_messenger(utils_messenger, None);
        device.destroy_device(None);
        instance.destroy_instance(None);
    }
}

unsafe extern "system" fn debug_callback(
    severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    type_: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _p_user_data: *mut std::ffi::c_void,
) -> u32 {
    use std::ffi::CStr;

    let message = unsafe { CStr::from_ptr((*p_callback_data).p_message) };

    if severity >= vk::DebugUtilsMessageSeverityFlagsEXT::ERROR {
        error!("({:?}) {:?}", type_, message);
    } else if severity >= vk::DebugUtilsMessageSeverityFlagsEXT::WARNING {
        warn!("({:?}) {:?}", type_, message);
    } else if severity >= vk::DebugUtilsMessageSeverityFlagsEXT::INFO {
        info!("({:?}) {:?}", type_, message);
    } else {
        trace!("({:?}) {:?}", type_, message);
    }

    0
}
