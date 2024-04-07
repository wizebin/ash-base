use std::{
    cell::RefCell, default::Default, sync::{Arc, Mutex},
};
use winit::{
    event_loop::{EventLoop, EventLoopBuilder},
    platform::{macos::EventLoopBuilderExtMacOS},
    window::{Window, WindowBuilder},
};

pub fn make_winit_window(app_name: &str) -> (RefCell<EventLoop<()>>, Arc<Mutex<Window>>) {
    let event_loop = RefCell::new(EventLoopBuilder::default()
        .with_activate_ignoring_other_apps(false)
        .build()
        .unwrap());

    let mut window = WindowBuilder::new()
        .with_title(app_name);

    if option_env!("ALWAYS_ON_TOP").is_some() {
        window = window.with_inner_size(winit::dpi::LogicalSize::new(
            f64::from(200),
            f64::from(200),
        ))
        .with_window_level(winit::window::WindowLevel::AlwaysOnTop)
        .with_position(winit::dpi::LogicalPosition::new(2580, 1100));
    } else {
        window = window.with_inner_size(winit::dpi::LogicalSize::new(
            f64::from(800),
            f64::from(600),
        ));
    }
    let window = window.build(&event_loop.borrow())
        .unwrap();
    let window = Arc::new(Mutex::new(window));

    (event_loop, window)
}
