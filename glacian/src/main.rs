use sdl3::{event::Event, keyboard::Keycode};
use std::time::Instant;

fn main() {
    tracing_subscriber::fmt().init();

    let sdl_context = sdl3::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let mut event_pump = sdl_context.event_pump().unwrap();

    let window = video_subsystem
        .window("rust-sdl3 demo", 800, 600)
        .position_centered()
        .vulkan()
        .metal_view()
        .resizable()
        .build()
        .unwrap();

    let mut r = glacian_render::Renderer::new(&window);

    let mut player_pos = glam::vec3(0., 0., 0.);

    let start_time = Instant::now();

    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => {
                    break 'running;
                }
                Event::Window {
                    win_event: sdl3::event::WindowEvent::Resized(..),
                    ..
                } => {
                    r.resize(&window);
                }
                Event::KeyDown {
                    keycode: Some(Keycode::W),
                    ..
                } => player_pos.y += 1.,
                Event::KeyDown {
                    keycode: Some(Keycode::A),
                    ..
                } => player_pos.x -= 1.,
                Event::KeyDown {
                    keycode: Some(Keycode::S),
                    ..
                } => player_pos.y -= 1.,
                Event::KeyDown {
                    keycode: Some(Keycode::D),
                    ..
                } => player_pos.x += 1.,
                Event::KeyDown {
                    keycode: Some(Keycode::Space),
                    ..
                } => player_pos.z -= 1.,
                Event::KeyDown {
                    keycode: Some(Keycode::Down),
                    ..
                } => player_pos.z += 1.,
                // Event::KeyDown { .., keycode: Some(Keycode::A), scancode, keymod, repeat, which, raw } => (),
                // Event::KeyUp { .., keycode, scancode, keymod, repeat, which, raw } => (),
                _ => (),
            }
        }
        let view = glam::Mat4::look_to_rh(player_pos, glam::Vec3::X, glam::Vec3::Z);

        let elapsed_time = start_time.elapsed().as_secs_f32();

        let sun_dir_vector =
            glam::vec3a(elapsed_time.sin() * 1.5, elapsed_time.cos() * 1.5, -0.5).normalize();

        r.render(view, sun_dir_vector);

        std::thread::sleep(std::time::Duration::from_millis(16));
    }
}
