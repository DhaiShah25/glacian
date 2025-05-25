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

    match unsafe { entry.try_enumerate_instance_version() }.unwrap() {
        // Vulkan 1.1+
        Some(version) => {
            let major = vk::api_version_major(version);
            let minor = vk::api_version_minor(version);
            let patch = vk::api_version_patch(version);
            tracing::info!("Running Vulkan Version: {}.{}.{}", major, minor, patch);
        }
        // Vulkan 1.0
        None => {}
    }

    let app_info = vk::ApplicationInfo {
        api_version: vk::make_api_version(0, 1, 3, 0),
        ..Default::default()
    };

    let mut instance_extensions = vec![ash::khr::surface::NAME.as_ptr()];

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
    .unwrap_or_else(|e| {
        panic!("{}", e);
    });

    #[cfg(feature = "debug")]
    let instance_layers = vec![
        c"VK_LAYER_KHRONOS_validation".as_ptr(),
        c"VK_LAYER_MANGOHUD_overlay_x86_64".as_ptr(),
    ];
    #[cfg(not(feature = "debug"))]
    let instance_layers = vec![];

    let create_info = vk::InstanceCreateInfo {
        p_application_info: &app_info,
        enabled_extension_count: instance_extensions.len() as u32,
        pp_enabled_extension_names: instance_extensions.as_ptr(),
        enabled_layer_count: instance_layers.len() as u32,
        pp_enabled_layer_names: instance_layers.as_ptr(),
        ..Default::default()
    };
    let instance = unsafe { entry.create_instance(&create_info, None) }.unwrap();
    let surface_instance = ash::khr::surface::Instance::new(&entry, &instance);

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

    let device = unsafe {
        instance
            .create_device(
                physical_device,
                &vk::DeviceCreateInfo {
                    enabled_extension_count: device_extensions.len() as u32,
                    pp_enabled_extension_names: device_extensions.as_ptr(),
                    ..Default::default()
                },
                None,
            )
            .unwrap()
    };

    // Limit to max ~60 fps update rate
    window.set_target_fps(60);

    while window.is_open() && !window.is_key_down(Key::Escape) {
        for i in buffer.iter_mut() {
            *i = 0; // write something more funny here!
        }

        // We unwrap here as we want this code to exit if it fails. Real applications may want to handle this in a different way
        window.update_with_buffer(&buffer, WIDTH, HEIGHT).unwrap();
    }

    unsafe {
        device.destroy_device(None);
        instance.destroy_instance(None);
    }
}
