//
// Copyright 2019 The Project Oak Authors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//

//! A logger that sends output to an Oak logging channel, for use with
//! the [log facade].
//!
//! [log facade]: https://crates.io/crates/log

#[cfg(test)]
mod tests;

use log::{Level, Log, Metadata, Record, SetLoggerError};
use oak::io::Sender;
use std::io::Write;

struct OakChannelLogger {
    sender: Sender,
}

impl Log for OakChannelLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= log::max_level()
    }
    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        // Only flush logging channel on newlines.
        let mut channel = std::io::LineWriter::new(self.sender);
        writeln!(
            &mut channel,
            "{}  {} : {} : {}",
            record.level(),
            record.file().unwrap_or_default(),
            record.line().unwrap_or_default(),
            record.args()
        )
        .unwrap();
    }
    fn flush(&self) {}
}

/// Default name for predefined node configuration that corresponds to a logging
/// pseudo-Node.
pub const DEFAULT_CONFIG_NAME: &str = "log";

/// Initialize Node-wide default logging.
///
/// Uses the default level (`Debug`) and the default pre-defined name
/// (`"log"`) identifying logging node configuration.
///
/// # Panics
///
/// Panics if a logger has already been set.
pub fn init_default() {
    init(Level::Debug, DEFAULT_CONFIG_NAME).unwrap();
}

/// Initialize Node-wide logging via a channel to a logging pseudo-Node.
///
/// Initialize logging at the given level, creating a logging pseudo-Node
/// whose configuration is identified by the given name.
///
/// # Errors
///
/// An error is returned if a logger has already been set.
pub fn init(level: Level, config: &str) -> Result<(), SetLoggerError> {
    // Create a channel and pass the read half to a fresh logging Node.
    let (sender, receiver) = oak::channel_create().unwrap();
    oak::node_create(config, receiver);
    receiver.close().unwrap();

    log::set_boxed_logger(Box::new(OakChannelLogger { sender }))?;
    log::set_max_level(level.to_level_filter());
    Ok(())
}
