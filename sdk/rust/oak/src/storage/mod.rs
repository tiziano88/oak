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

//! Helper library for accessing Oak storage services.

use crate::grpc;
use crate::io::{Receiver, Sender};
use crate::proto::grpc_encap::{GrpcRequest, GrpcResponse};
use crate::proto::storage_channel::{
    StorageChannelDeleteRequest, StorageChannelDeleteResponse, StorageChannelReadRequest,
    StorageChannelReadResponse, StorageChannelWriteRequest, StorageChannelWriteResponse,
};
use crate::{Handle, OakError, OakStatus};
use log::info;
use protobuf::{Message, ProtobufEnum};

/// Default name for predefined node config that corresponds to a storage
/// pseudo-Node.
pub const DEFAULT_CONFIG_NAME: &str = "storage";

/// Local representation of the connection to an external storage service.
pub struct Storage {
    sender: Sender,
}

struct StorageRequestWrapper {
    grpc_request: GrpcRequest,
    receiver: Receiver,
}

impl crate::io::Encodable for &StorageRequestWrapper {
    fn encode(&self) -> Result<(Vec<u8>, Vec<Handle>), OakError> {
        let bytes = self.grpc_request.write_to_bytes()?;
        let handles = vec![self.receiver.handle];
        Ok((bytes, handles))
    }
}

impl Storage {
    /// Create a default `Storage` instance assuming the default pre-defined
    /// name (`"storage"`) identifying storage node config.
    pub fn default() -> Option<Storage> {
        Storage::new(DEFAULT_CONFIG_NAME)
    }

    /// Create a `Storage` instance using the given name identifying storage
    /// node configuration.
    pub fn new(config: &str) -> Option<Storage> {
        // Create a channel and pass the read half to a fresh storage Node.
        let (sender, receiver) = match crate::channel_create() {
            Ok(x) => x,
            Err(_e) => return None,
        };
        if crate::node_create(config, receiver) != crate::OakStatus::OK {
            sender.close().unwrap();
            receiver.close().unwrap();
            return None;
        }

        receiver.close().unwrap();
        Some(Storage { sender })
    }

    fn execute_operation<Req, Res>(
        &mut self,
        grpc_method_name: &str,
        operation_request: &Req,
    ) -> grpc::Result<Res>
    where
        Req: protobuf::Message,
        Res: protobuf::Message,
    {
        info!(
            "StorageChannelRequest: {}",
            protobuf::text_format::print_to_string(operation_request)
        );

        let mut request_any = protobuf::well_known_types::Any::new();
        // TODO: rust-protobuf does not provide pack_from/unpack_to functions.
        // https://github.com/stepancheg/rust-protobuf/issues/455
        request_any.set_type_url(
            String::from("type.googleapis.com/") + operation_request.descriptor().full_name(),
        );
        operation_request
            .write_to_writer(&mut request_any.value)
            .expect("could not serialize storage operation request as `any`");
        let mut grpc_request = GrpcRequest::new();
        grpc_request.method_name = grpc_method_name.to_owned();
        grpc_request.set_req_msg(request_any);

        // Create an ephemeral channel for the response.
        let (sender, receiver) = match crate::channel_create() {
            Ok(x) => x,
            Err(status) => {
                return Err(grpc::build_status(
                    grpc::Code::INTERNAL,
                    &format!(
                        "failed to create storage response channel: {}",
                        status.value()
                    ),
                ));
            }
        };

        let storage_request_wrapper = StorageRequestWrapper {
            grpc_request,
            receiver,
        };

        sender
            .send(&storage_request_wrapper)
            .expect("could not send storage operation request");
        sender.close().expect("could not close channel");

        let mut grpc_response: GrpcResponse = receiver
            .receive()
            .expect("could not receive storage operation response");
        info!(
            "StorageChannelResponse: {}",
            protobuf::text_format::print_to_string(&grpc_response)
        );
        receiver.close().expect("could not close channel");

        let status = grpc_response.take_status();
        if status.code != 0 {
            Err(status)
        } else {
            let response = protobuf::parse_from_bytes(grpc_response.get_rsp_msg().value.as_slice())
                .expect("could not parse storage operation response");
            Ok(response)
        }
    }

    /// Read the value associated with the given `name` from the storage
    /// instance identified by `name`.
    pub fn read(&mut self, storage_name: &[u8], name: &[u8]) -> grpc::Result<Vec<u8>> {
        let mut read_request = StorageChannelReadRequest::new();
        read_request.storage_name = storage_name.to_owned();
        read_request.mut_item().set_name(name.to_owned());

        // TODO: Automatically generate boilerplate from the proto definition.
        self.execute_operation::<StorageChannelReadRequest, StorageChannelReadResponse>(
            "/oak.StorageNode/Read",
            &read_request,
        )
        .map(|r| r.get_item().get_value().to_vec())
    }

    /// Set the value associated with the given `name` from the storage instance
    /// identified by `name`.
    pub fn write(&mut self, storage_name: &[u8], name: &[u8], value: &[u8]) -> grpc::Result<()> {
        let mut write_request = StorageChannelWriteRequest::new();
        // TODO: Set policy for item.
        write_request.storage_name = storage_name.to_owned();
        write_request.mut_item().set_name(name.to_owned());
        write_request.mut_item().set_value(value.to_owned());

        // TODO: Automatically generate boilerplate from the proto definition.
        self.execute_operation::<StorageChannelWriteRequest, StorageChannelWriteResponse>(
            "/oak.StorageNode/Write",
            &write_request,
        )
        .map(|_| ())
    }

    /// Delete the value associated with the given `name` from the storage
    /// instance identified by `name`.
    pub fn delete(&mut self, storage_name: &[u8], name: &[u8]) -> grpc::Result<()> {
        let mut delete_request = StorageChannelDeleteRequest::new();
        delete_request.storage_name = storage_name.to_owned();
        delete_request.mut_item().set_name(name.to_owned());

        // TODO: Automatically generate boilerplate from the proto definition.
        self.execute_operation::<StorageChannelDeleteRequest, StorageChannelDeleteResponse>(
            "/oak.StorageNode/Delete",
            &delete_request,
        )
        .map(|_| ())
    }
}
