use glfw::{Action, Context, Key, WindowEvent};

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

    let mut r = glade::render::RenderEngine::new(&window);

    let mut resized = false;

    while !window.should_close() {
        window.swap_buffers();
        glfw.poll_events();

        for (_, event) in glfw::flush_messages(&events) {
            println!("{event:?}");
            match event {
                WindowEvent::Key(Key::Escape, _, Action::Press, _) => window.set_should_close(true),
                WindowEvent::Size(w, h) => {
                    r.resize(&window, w as u32, h as u32);
                    resized = !resized;
                }
                _ => {}
            }
        }

        r.render(resized);
    }
}
