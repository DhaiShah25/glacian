use sdl3::{event::Event, keyboard::Keycode};
use std::time::Instant;
mod render;
use rootcause::prelude::Report;

const SENSITIVITY: f32 = std::f32::consts::PI / 1024.;

fn main() -> Result<(), Report> {
    let sdl_context = sdl3::init()?;
    let video_subsystem = sdl_context.video()?;

    let mut event_pump = sdl_context.event_pump()?;

    let window = video_subsystem
        .window("rust-sdl3 demo", 800, 600)
        .position_centered()
        .vulkan()
        .maximized()
        .metal_view()
        .build()?;

    sdl_context.mouse().set_relative_mouse_mode(&window, true);

    let mut r = render::Renderer::new(&window);

    let mut player_pos = glam::vec3(0., 0., 0.);

    let (mut yaw, mut pitch) = (0., 0.);

    let start_time = Instant::now();

    'running: loop {
        let mut velocity = glam::Vec3::ZERO;
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
                } => velocity.y += 1.,
                Event::KeyDown {
                    keycode: Some(Keycode::A),
                    ..
                } => velocity.x -= 1.,
                Event::KeyDown {
                    keycode: Some(Keycode::S),
                    ..
                } => velocity.y -= 1.,
                Event::KeyDown {
                    keycode: Some(Keycode::D),
                    ..
                } => velocity.x += 1.,
                Event::KeyDown {
                    keycode: Some(Keycode::Space),
                    ..
                } => velocity.z -= 1.,
                Event::KeyDown {
                    keycode: Some(Keycode::Down),
                    ..
                } => velocity.z += 1.,
                Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => {
                    sdl_context.mouse().set_relative_mouse_mode(
                        &window,
                        !sdl_context.mouse().relative_mouse_mode(&window),
                    );
                    break 'running;
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
        player_pos.x += velocity.x * pitch.cos() - velocity.y * pitch.sin();
        player_pos.y += velocity.x * pitch.sin() + velocity.y * pitch.cos();

        let view = glam::Mat4::look_to_rh(
            player_pos,
            glam::vec3(pitch.cos() * yaw.cos(), yaw.sin(), yaw.sin() * pitch.cos()),
            glam::Vec3::Z,
        );

        let elapsed_time = start_time.elapsed().as_secs_f32() / 4.0;

        let sky_color = glam::vec3a(0.7, 0.7, 1.0)
            .lerp(glam::vec3a(0., 0., 0.), ((elapsed_time).cos() + 1.) * 0.5);

        dbg!(sky_color);

        r.render(view, sky_color);
        std::thread::sleep(std::time::Duration::from_millis(16));
    }
    Ok(())
}
