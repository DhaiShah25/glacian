use sdl3::{event::Event, keyboard::Keycode, sys::keycode};

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
                // Event::KeyDown { .., keycode: Some(Keycode::A), scancode, keymod, repeat, which, raw } => (),
                // Event::KeyUp { .., keycode, scancode, keymod, repeat, which, raw } => (),
                _ => (),
            }
        }
        let view = glam::Mat4::look_to_rh(player_pos, glam::Vec3::Y, glam::Vec3::Z);

        r.render(view, glam::vec3a(1.0, 0.0, 0.0));
    }
}
