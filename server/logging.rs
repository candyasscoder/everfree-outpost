use std::io::{self, Stderr, Write};
use log::{self, Log, LogLevel, LogLevelFilter, LogRecord};

struct Logger;

impl Log for Logger {
    fn enabled(&self, level: LogLevel, module: &str) -> bool {
        true
    }

    fn log(&self, record: &LogRecord) {
        let level = match record.level() {
            LogLevel::Error => "ERR",
            LogLevel::Warn => "WARN",
            LogLevel::Info => "INFO",
            LogLevel::Debug => "DBG",
            LogLevel::Trace => "TRC",
        };

        writeln!(&mut io::stderr(), "[{:4}] {}", level, record.args());
    }
}

pub fn init() {
    log::set_logger(|max| {
        max.set(LogLevelFilter::Trace);
        Box::new(Logger) as Box<Log>
    });
}
