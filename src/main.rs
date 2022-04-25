slint::include_modules!();

use slint::{Model, Timer, TimerMode};

fn main() {
    let ui = MainWindow::new();

    let ui_handle = ui.as_weak();

    let timer = Timer::default();
    timer.start(
        TimerMode::Repeated,
        std::time::Duration::from_secs(5),
        move || {
            let ui = ui_handle.unwrap();
            let app_state_count = ui.get_app_states().row_count() as i32;
            let mut app_state = ui.get_app_state();

            println!("{}, {}", app_state_count, app_state);

            app_state += 1;

            if app_state == app_state_count {
                app_state = 0;
            }

            ui.set_app_state(app_state);
        },
    );

    ui.run();
}
