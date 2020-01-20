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

use crate::io::{Decodable, Encodable};

/// A simple holder for bytes + handles, trivially implementing [`Encodable`] and [`Decodable`] by
/// cloning its internally owned buffers.
#[derive(Debug, PartialEq, Eq)]
pub struct Buffer {
    bytes: Vec<u8>,
    handles: Vec<crate::Handle>,
}

impl Encodable for Buffer {
    fn encode(&self) -> Result<(Vec<u8>, Vec<crate::Handle>), crate::OakError> {
        Ok((self.bytes.clone(), self.handles.clone()))
    }
}

impl Decodable for Buffer {
    fn decode(bytes: &[u8], handles: &[crate::Handle]) -> Result<Self, crate::OakError> {
        Ok(Buffer {
            bytes: bytes.into(),
            handles: handles.into(),
        })
    }
}

#[test]
fn round_trip() {
    fn assert_round_trip(entry: Buffer) {
        let (bytes, handles) = entry.encode().expect("could not encode buffer");
        let decoded = Buffer::decode(&bytes, &handles).expect("could not decode bytes + handles");
        assert_eq!(entry, decoded);
    }
    let entries = vec![
        Buffer {
            bytes: vec![],
            handles: vec![],
        },
        Buffer {
            bytes: vec![14, 12],
            handles: vec![crate::Handle::from_raw(19), crate::Handle::from_raw(88)],
        },
        Buffer {
            bytes: vec![14, 12],
            handles: vec![crate::Handle::invalid()],
        },
    ];
    for entry in entries {
        assert_round_trip(entry);
    }
}
