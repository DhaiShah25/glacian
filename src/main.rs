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
    window.set_cursor_pos_polling(true);

    window.set_cursor_mode(glfw::CursorMode::Disabled);

    let mut r = glade::render::RenderEngine::new(&window);

    // for scene in gltf::Gltf::open("./assets/Untitled.glb").unwrap().scenes() {
    //     for node in scene.nodes() {
    //         let mesh = node.mesh().unwrap();
    //         for prim in mesh.primitives() {
    //             // dbg!(prim.indices());
    //             prim.attributes().for_each(|attr| {
    //                 if attr.0 == gltf::Semantic::Positions {
    //                     println!("{:?}", attr.1);
    //                 }
    //             });
    //         }
    //     }
    // }

    let mut view_dir = glade::ViewDirection::new(window.get_size());

    let in_gui = false;

    let mut time = 0;

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
                WindowEvent::Size(w, h) => {
                    r.resize(&window, w as u32, h as u32);
                    view_dir.resize((w, h));
                }
                WindowEvent::CursorPos(x, y) => {
                    view_dir.update((x as i32, y as i32));
                    if !in_gui {
                        let size = window.get_size();
                        window.set_cursor_pos(size.0 as f64 / 2., size.1 as f64 / 2.);
                    }
                }
                _ => {}
            }
        }

        r.render((view_dir.yaw, view_dir.pitch), time);

        time = time.wrapping_add(5);
        dbg!(time);
    }
}
