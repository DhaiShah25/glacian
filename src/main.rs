use minifb::{Key, Window, WindowOptions};

fn main() {
    tracing_subscriber::FmtSubscriber::builder()
        .pretty()
        .without_time()
        .init();

    let mut window = Window::new(
        "Shadow Engine",
        640,
        360,
        WindowOptions {
            borderless: true,
            resize: true,
            ..Default::default()
        },
    )
    .unwrap();

    let mut r = glade::render::RenderEngine::new(&window).unwrap();

    window.set_target_fps(60);

    let mut window_size = window.get_size();

    while window.is_open() && !window.is_key_down(Key::Escape) {
        window.update();

        if window_size == window.get_size() {
            r.render(false, &window, (window_size.0 as f32, window_size.1 as f32));
        } else {
            window_size = window.get_size();
            r.render(true, &window, (window_size.0 as f32, window_size.1 as f32));
        }
    }
}
