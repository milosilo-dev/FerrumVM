use std::{
    fs::{File, OpenOptions},
    io::Write,
    sync::{Mutex, OnceLock},
};

fn logger() -> &'static Mutex<File> {
    static LOG: OnceLock<Mutex<File>> = OnceLock::new();
    LOG.get_or_init(|| {
        Mutex::new(
            OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open("ferrum-host.log")
                .expect("failed to open ferrum-host.log"),
        )
    })
}

pub fn log_msg(msg: &str) {
    if let Ok(mut file) = logger().lock() {
        let _ = writeln!(file, "{}", msg);
    }
}
