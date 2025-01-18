use crate::UI;
use ratatui::layout::Rect;
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;
use std::sync::atomic::Ordering;
use {
    compact_str::CompactStringExt,
    log::{Level, LevelFilter, Log, Metadata, Record},
    std::collections::VecDeque,
    std::io::Read,
    std::sync::atomic::AtomicU16,
    std::sync::mpsc::{Receiver, SyncSender},
};

pub struct DebugLog {
    debug_log: VecDeque<String>,
    pub(crate) max_rows: AtomicU16,
    rx: Receiver<String>,
}

impl DebugLog {
    pub fn new(max_lines: u16) -> (Self, SyncSender<String>) {
        let (tx, rx) = std::sync::mpsc::sync_channel(1000);
        (
            Self {
                debug_log: VecDeque::new(),
                max_rows: AtomicU16::new(max_lines),
                rx,
            },
            tx,
        )
    }

    pub fn push_logs(&mut self, logs: String) {
        if logs.is_empty() {
            return;
        }

        self.debug_log.push_back(logs);
        let max_row = self.max_rows.load(Ordering::SeqCst) as usize;
        while self.debug_log.len() > max_row {
            self.debug_log.pop_front();
        }
    }

    pub fn update_logs(&mut self) {
        let max_row = self.max_rows.load(Ordering::SeqCst) as usize;

        for log in self.rx.try_iter() {
            if log.is_empty() {
                continue;
            }

            self.debug_log.push_back(log);
            if self.debug_log.len() > max_row {
                self.debug_log.pop_front();
            }
        }
    }

    pub fn get_logs(&mut self) -> String {
        self.debug_log.join_compact("").to_string()
    }
}

#[derive(Clone)]
pub struct LogSubscriber {
    tx: SyncSender<String>,
}

impl LogSubscriber {
    pub fn new(tx: SyncSender<String>) -> Self {
        let log_subscriber = Self { tx };
        log::set_boxed_logger(Box::new(log_subscriber.clone())).unwrap();
        log::set_max_level(LevelFilter::Info);
        log_subscriber
    }

    pub fn push_logs(&self, log: String) {
        if let Err(err) = self.tx.send(log) {
            eprintln!("Error Sending Logs to MPSC Channel: {}", err.0)
        }
    }
}

impl Log for LogSubscriber {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Debug
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let message = format!("[{}] - {}\n", record.level(), record.args());

            self.push_logs(message);
        }
    }

    fn flush(&self) {}
}

pub trait UIExt {
    fn update_debug_output(&mut self);

    fn draw_debug_output(&mut self, f: &mut Frame, area: Rect);
}

impl UIExt for UI {
    fn update_debug_output(&mut self) {
        let mut error_buffer = String::new();
        self.error_redirect
            .read_to_string(&mut error_buffer)
            .unwrap();
        self.debug_output.push_logs(error_buffer);
        self.debug_output.update_logs();
    }

    fn draw_debug_output(&mut self, f: &mut Frame, area: Rect) {
        let mut error_buffer = String::new();
        self.error_redirect
            .read_to_string(&mut error_buffer)
            .unwrap();
        self.debug_output.push_logs(error_buffer);

        let debug_output = self.debug_output.get_logs();

        let p = Paragraph::new(debug_output).block(
            Block::default()
                .title(" Debug Output ")
                .title_bottom(format!("{} log entries", self.debug_output.debug_log.len()))
                .borders(Borders::ALL),
        );
        f.render_widget(p, area);
    }
}
