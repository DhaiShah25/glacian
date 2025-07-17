use glfw::{Action, Context, Key, WindowEvent};
use noise::NoiseFn;

fn main() {
    tracing_subscriber::FmtSubscriber::builder()
        .pretty()
        .without_time()
        .init();

    let mut glfw = glfw::init_no_callbacks().unwrap();

    glfw.window_hint(glfw::WindowHint::ClientApi(glfw::ClientApiHint::NoApi));

    let (mut window, events) = glfw
        .create_window(640, 360, "Shadow Engine", glfw::WindowMode::Windowed)
        .expect("Failed to create window");

    window.make_current();
    window.set_key_polling(true);
    window.set_size_polling(true);
    window.set_cursor_pos_polling(true);

    window.set_cursor_mode(glfw::CursorMode::Disabled);

    let mut r = glade::render::RenderEngine::new(&window);

    let mut prev_coords = (0., 0.);

    let in_gui = false;

    while !window.should_close() {
        window.swap_buffers();

        if in_gui {
            glfw.wait_events();
        } else {
            glfw.poll_events();
        }

        for (_, event) in glfw::flush_messages(&events) {
            // println!("{event:?}");
            match event {
                WindowEvent::Key(Key::Escape, _, Action::Press, _) => window.set_should_close(true),
                WindowEvent::Size(w, h) => r.resize(&window, w as u32, h as u32),
                WindowEvent::CursorPos(x, y) => {
                    // dbg!(calc_velocity(prev_coords.0, x));
                    // dbg!(calc_velocity(prev_coords.1, y));
                    prev_coords = (x, y);
                    if !in_gui {
                        let size = window.get_size();
                        window.set_cursor_pos(size.0 as f64 / 2., size.1 as f64 / 2.);
                    }
                }
                _ => {}
            }
        }

        r.render();
    }
}

fn calc_velocity(x1: f64, x2: f64) -> f64 {
    (x2 - x1).abs()
}
