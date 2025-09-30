use sdl3::{event::Event, keyboard::Keycode};
use std::time::Instant;

const SENSITIVITY: f32 = std::f32::consts::PI / 512.;

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

    sdl_context.mouse().set_relative_mouse_mode(&window, true);

    let mut r = glacian_render::Renderer::new(&window);

    let mut player_pos = glam::vec3(0., 0., 0.);

    let (mut yaw, mut pitch) = (0., std::f32::consts::PI);

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
                Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => {
                    sdl_context.mouse().set_relative_mouse_mode(
                        &window,
                        !sdl_context.mouse().relative_mouse_mode(&window),
                    );
                }
                // Event::KeyDown { .., keycode: Some(Keycode::A), scancode, keymod, repeat, which, raw } => (),
                // Event::KeyUp { .., keycode, scancode, keymod, repeat, which, raw } => (),
                Event::MouseMotion { xrel, yrel, .. } => {
                    yaw += xrel * SENSITIVITY;
                    pitch -= yrel * SENSITIVITY;
                }
                _ => (),
            }
        }
        let view = glam::Mat4::look_to_rh(
            player_pos,
            glam::vec3(pitch.cos() * yaw.cos(), yaw.sin(), yaw.sin() * pitch.cos()),
            glam::Vec3::Z,
        );

        dbg!(view);

        let elapsed_time = start_time.elapsed().as_secs_f32();

        dbg!(yaw / std::f32::consts::PI, pitch / std::f32::consts::PI);

        // --- REWRITTEN SUN_DIR CALCULATION ---

        // This calculates a unit vector that rotates in the X-Y plane (the ground plane)
        // with a fixed negative Z component to make it point slightly down.
        let sun_dir_vector = glam::vec3a(
            elapsed_time.cos(), // X component
            elapsed_time.sin(), // Y component
            0.5,                // Fixed Z component (downward angle)
        )
        .normalize();

        // -------------------------------------

        r.render(view, sun_dir_vector);
        std::thread::sleep(std::time::Duration::from_millis(16));
    }
}
