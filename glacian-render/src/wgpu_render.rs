use pollster::block_on;
use wgpu::Instance;

pub struct WgpuRenderer {
    instance: Instance,
}

impl WgpuRenderer {
    pub fn new(window: &sdl3::video::Window) -> Self {
        use wgpu::InstanceFlags;
        let instance = Instance::new(&wgpu::InstanceDescriptor {
            #[cfg(feature = "debug")]
            flags: InstanceFlags::DEBUG | InstanceFlags::VALIDATION,
            ..Default::default()
        });

        let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptionsBase {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: true,
            compatible_surface: None,
        }))
        .unwrap();

        let (device, queue) = block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("WGPU Logical Device"),
            // TODO: Use push constants instead later on native platforms
            required_features: wgpu::Features::empty(),
            memory_hints: wgpu::MemoryHints::Performance,
            required_limits: wgpu::Limits::defaults(),
            trace: wgpu::Trace::Off,
        }))
        .unwrap();

        dbg!(device.features());
        dbg!(device.limits());

        Self { instance }
    }

    pub fn resize() {}

    pub fn render(&mut self) {}
}
