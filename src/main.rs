slint::include_modules!();

use slint::{Timer, TimerMode};
use util::connection::Server;
use util::thread::kill_double;

fn main() {
    if !kill_double() {
        let server = Server::new();
        let server_dualchannel = server.dual_channel;
        let ui = MainWindow::new();
        let ui_handle = ui.as_weak();
        let timer = Timer::default();

        timer.start(
            TimerMode::Repeated,
            std::time::Duration::from_millis(50),
            move || {
                let _ui = ui_handle.unwrap();
            },
        );

        ui.run();
    }
}
