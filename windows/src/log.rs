use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Instant;

static LOG: Mutex<Option<(File, Instant)>> = Mutex::new(None);

pub fn path() -> Option<PathBuf> {
    std::env::current_exe().ok().map(|exe| exe.with_file_name("snowhush.log"))
}

pub fn start() {
    // Portable build: no log file unless asked for with --log.
    if !std::env::args().skip(1).any(|argument| argument == "--log") {
        return;
    }
    let Some(path) = path() else { return };
    if let Some(folder) = path.parent() {
        let _ = std::fs::create_dir_all(folder);
    }
    if let Ok(file) = File::create(path)
        && let Ok(mut log) = LOG.lock()
    {
        *log = Some((file, Instant::now()));
    }
}

pub fn line(arguments: std::fmt::Arguments) {
    if let Ok(mut log) = LOG.lock()
        && let Some((file, started)) = log.as_mut()
    {
        let _ = writeln!(file, "{:>9.2}  {}", started.elapsed().as_secs_f64(), arguments);
    }
}

#[macro_export]
macro_rules! log {
    ($($argument:tt)*) => { $crate::log::line(format_args!($($argument)*)) };
}
