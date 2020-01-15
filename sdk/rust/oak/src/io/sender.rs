//
// Copyright 2020 The Project Oak Authors
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

use crate::io::Encodable;
use crate::{Handle, OakError, OakStatus};
use protobuf::ProtobufEnum;
use serde::{Deserialize, Serialize};

/// Wrapper for a handle to the send half of a channel.
///
/// For use when the underlying [`Handle`] is known to be for a send half.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct Sender {
    pub handle: Handle,
}

impl Sender {
    pub fn new(handle: Handle) -> Self {
        Sender { handle }
    }

    /// Close the underlying channel used by this sender.
    pub fn close(&self) -> Result<(), OakStatus> {
        let status = crate::channel_close(self.handle);
        match OakStatus::from_i32(status as i32) {
            Some(OakStatus::OK) => Ok(()),
            Some(err) => Err(err),
            None => Err(OakStatus::OAK_STATUS_UNSPECIFIED),
        }
    }

    /// Attempt to send a value on this sender.
    ///
    /// See https://doc.rust-lang.org/std/sync/mpsc/struct.Sender.html#method.send
    pub fn send<T>(&self, t: T) -> Result<(), OakError>
    where
        T: Encodable,
    {
        let (bytes, handles) = t.encode()?;
        let status = crate::channel_write(self.handle, &bytes, &handles);
        match OakStatus::from_i32(status as i32) {
            Some(OakStatus::OK) => Ok(()),
            Some(err) => Err(err.into()),
            None => Err(OakStatus::OAK_STATUS_UNSPECIFIED.into()),
        }
    }
}
