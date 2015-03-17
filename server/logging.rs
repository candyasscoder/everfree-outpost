use std::io::{self, Write};
use log::{self, Log, LogLevel, LogLevelFilter, LogRecord};

struct Logger;

impl Log for Logger {
    fn enabled(&self, _level: LogLevel, _module: &str) -> bool {
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

        let result = writeln!(&mut io::stderr(), "[{:4}] {}", level, record.args());
        // Being unable to print to the log could be really bad.
        result.unwrap();
    }
}

pub fn init() {
    log::set_logger(|max| {
        max.set(LogLevelFilter::Trace);
        Box::new(Logger) as Box<Log>
    }).unwrap();
}
