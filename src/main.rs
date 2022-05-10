slint::include_modules!();

use slint::{Timer, TimerMode};
use util::connection::{command::DeviceConfigurationDescriptor, CommandTrait, Server};
use util::thread::kill_double;

fn main() {
    if !kill_double() {
        let server = Server::new();
        let mut server_dualchannel = server.dual_channel;
        let ui = MainWindow::new();
        let ui_handle = ui.as_weak();
        let timer = Timer::default();

        timer.start(
            TimerMode::Repeated,
            std::time::Duration::from_millis(50),
            move || {
                let _ui = ui_handle.unwrap();
                let (_thread_id, is_running, data) = server_dualchannel.recv().unwrap();

                if is_running && data.len() > 0 {
                    println!("{:?}", DeviceConfigurationDescriptor::from_bytes(data));
                }
            },
        );

        ui.run();
    }
}
