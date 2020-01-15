//
// Copyright 2018 The Project Oak Authors
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

//! Functionality to help Oak Nodes interact with gRPC.

use crate::io::{Receiver, Sender};
pub use crate::proto::code::Code;
use crate::{proto, Handle, OakError, OakStatus};
use log::info;
use protobuf::{ProtobufEnum, ProtobufResult};

/// Result type that uses a [`proto::status::Status`] type for error values.
pub type Result<T> = std::result::Result<T, proto::status::Status>;

/// Helper to create a gRPC status object.
pub fn build_status(code: Code, msg: &str) -> proto::status::Status {
    let mut status = proto::status::Status::new();
    status.set_code(code.value());
    status.set_message(msg.to_string());
    status
}

/// Channel-holding object that encapsulates response messages into
/// `GrpcResponse` wrapper messages and writes serialized versions to a send
///  channel.
pub struct ChannelResponseWriter {
    sender: Sender,
}

/// Indicate whether a write method should leave the current gRPC method
/// invocation open or close it.
#[derive(PartialEq, Clone, Debug)]
pub enum WriteMode {
    KeepOpen,
    Close,
}

impl ChannelResponseWriter {
    pub fn new(sender: Sender) -> Self {
        ChannelResponseWriter { sender }
    }

    /// Retrieve the Oak handle underlying the writer object.
    pub fn handle(self) -> crate::Handle {
        self.sender.handle
    }

    /// Write out a gRPC response and optionally close out the method
    /// invocation.  Any errors from the channel are silently dropped.
    pub fn write<T: protobuf::Message>(&mut self, rsp: &T, mode: WriteMode) -> ProtobufResult<()> {
        // Put the serialized response into a GrpcResponse message wrapper and
        // serialize it into the channel.
        let mut grpc_rsp = proto::grpc_encap::GrpcResponse::new();
        let mut any = protobuf::well_known_types::Any::new();
        rsp.write_to_writer(&mut any.value)
            .expect("could not write `any` message");
        grpc_rsp.set_rsp_msg(any);
        grpc_rsp.set_last(match mode {
            WriteMode::KeepOpen => false,
            WriteMode::Close => true,
        });
        // TODO: Handle errors.
        self.sender
            .send(&grpc_rsp)
            .expect("could not write gRPC response");
        if mode == WriteMode::Close {
            self.sender.close().expect("could not close channel");
        }
        Ok(())
    }

    /// Write an empty gRPC response and optionally close out the method
    /// invocation. Any errors from the channel are silently dropped.
    pub fn write_empty(&mut self, mode: WriteMode) -> ProtobufResult<()> {
        let mut grpc_rsp = proto::grpc_encap::GrpcResponse::new();
        grpc_rsp.set_rsp_msg(protobuf::well_known_types::Any::new());
        grpc_rsp.set_last(match mode {
            WriteMode::KeepOpen => false,
            WriteMode::Close => true,
        });
        // TODO: Handle errors.
        self.sender
            .send(&grpc_rsp)
            .expect("could not write gRPC response");
        if mode == WriteMode::Close {
            self.sender.close().expect("could not close channel");
        }
        Ok(())
    }

    /// Close out the gRPC method invocation with the given final result.
    pub fn close(&mut self, result: Result<()>) -> ProtobufResult<()> {
        // Build a final GrpcResponse message wrapper and serialize it into the
        // channel.
        let mut grpc_rsp = proto::grpc_encap::GrpcResponse::new();
        grpc_rsp.set_last(true);
        if let Err(status) = result {
            grpc_rsp.set_status(status);
        }
        // TODO: Handle errors.
        self.sender
            .send(&grpc_rsp)
            .expect("could not write gRPC response");
        self.sender.close().expect("could not close channel");
        Ok(())
    }
}

/// Trait for Oak Nodes that act as a gRPC services.
///
/// An `OakNode` instance is normally passed to [`event_loop`], to allow
/// repeated invocation of its `invoke()` method.
pub trait OakNode {
    /// Construct the (single) instance of the node.
    ///
    /// This method may choose to initialize logging by invoking
    /// [`oak_log::init()`].
    ///
    /// [`oak_log::init()`]: ../../oak_log/fn.init.html
    fn new() -> Self
    where
        Self: Sized;

    /// Process a single gRPC method invocation.
    ///
    /// The method name is provided by `method` and the incoming serialized gRPC
    /// request is held in `req`.  Response messages should be written using
    /// `writer.write`, followed by `writer.close`.
    fn invoke(&mut self, method: &str, req: &[u8], writer: ChannelResponseWriter);
}

struct GrpcRequestWrapper {
    grpc_request: proto::grpc_encap::GrpcRequest,
    sender: Sender,
}

impl crate::io::Decodable for GrpcRequestWrapper {
    fn decode(bytes: &[u8], handles: &[Handle]) -> std::result::Result<Self, OakError> {
        let grpc_request = protobuf::parse_from_bytes(bytes)?;
        let handle = handles.get(0).ok_or(OakStatus::ERR_BAD_HANDLE)?;
        let sender = Sender::new(*handle);
        Ok(GrpcRequestWrapper {
            grpc_request,
            sender,
        })
    }
}

/// Perform a gRPC event loop for a Node.
///
/// Invoking the given `node`'s [`invoke`] method for each incoming request that
/// arrives on the inbound channel as a serialized [`GrpcRequest`] message,
/// giving the [`invoke`] method the outbound channel for encapsulated responses
/// to be written to.
///
/// [`invoke`]: OakNode::invoke
/// [`GrpcRequest`]: crate::proto::grpc_encap::GrpcRequest
pub fn event_loop<T: OakNode>(mut node: T, grpc_receiver: Receiver) -> i32 {
    info!("start event loop for node with handle {:?}", grpc_receiver);
    if !grpc_receiver.handle.is_valid() {
        return OakStatus::ERR_CHANNEL_CLOSED.value();
    }
    crate::set_panic_hook();
    loop {
        let grpc_request_wrapper: GrpcRequestWrapper = grpc_receiver
            .receive()
            .expect("could not receive GrpcRequestWrapper");
        let req = grpc_request_wrapper.grpc_request;
        if !req.last {
            panic!("Support for streaming requests not yet implemented");
        }
        node.invoke(
            &req.method_name,
            req.get_req_msg().value.as_slice(),
            ChannelResponseWriter::new(grpc_request_wrapper.sender),
        );
    }
}

/// Generic function that handles request deserialization and response
/// serialization (and sending down a channel) for a function that handles a
/// request/response pair.
///
/// Panics if [de-]serialization or channel operations fail.
pub fn handle_req_rsp<C, R, Q>(mut node_fn: C, req: &[u8], mut writer: ChannelResponseWriter)
where
    C: FnMut(R) -> Result<Q>,
    R: protobuf::Message,
    Q: protobuf::Message,
{
    let r: R = protobuf::parse_from_bytes(&req).expect("Failed to parse request protobuf message");
    let result = match node_fn(r) {
        Ok(rsp) => writer.write(&rsp, WriteMode::Close),
        Err(status) => writer.close(Err(status)),
    };
    if let Err(e) = result {
        panic!("Failed to process response: {}", e)
    }
}

/// Generic function that handles request deserialization and response
/// serialization (and sending down a channel) for a function that handles a
/// request and streams responses.
///
/// Panics if [de-]serialization or channel operations fail.
pub fn handle_req_stream<C, R>(mut node_fn: C, req: &[u8], writer: ChannelResponseWriter)
where
    C: FnMut(R, ChannelResponseWriter),
    R: protobuf::Message,
{
    let r: R = protobuf::parse_from_bytes(&req).expect("Failed to parse request protobuf message");
    node_fn(r, writer)
}

/// Generic function that handles request deserialization and response
/// serialization (and sending down a channel) for a function that handles a
/// stream of requests to produce a response.
///
/// Panics if [de-]serialization or channel operations fail.
pub fn handle_stream_rsp<C, R, Q>(mut node_fn: C, req: &[u8], mut writer: ChannelResponseWriter)
where
    C: FnMut(Vec<R>) -> Result<Q>,
    R: protobuf::Message,
    Q: protobuf::Message,
{
    // TODO(#97): better client-side streaming
    let rr: Vec<R> =
        vec![protobuf::parse_from_bytes(&req).expect("Failed to parse request protobuf message")];
    let result = match node_fn(rr) {
        Ok(rsp) => writer.write(&rsp, WriteMode::Close),
        Err(status) => writer.close(Err(status)),
    };
    if let Err(e) = result {
        panic!("Failed to process response: {}", e)
    }
}

/// Generic function that handles request deserialization and response
/// serialization (and sending down a channel) for a function that handles a
/// stream of requests to produce a stream of responses.
///
/// Panics if [de-]serialization or channel operations fail.
pub fn handle_stream_stream<C, R>(mut node_fn: C, req: &[u8], writer: ChannelResponseWriter)
where
    C: FnMut(Vec<R>, ChannelResponseWriter),
    R: protobuf::Message,
{
    // TODO(#97): better client-side streaming
    let rr: Vec<R> =
        vec![protobuf::parse_from_bytes(&req).expect("Failed to parse request protobuf message")];
    node_fn(rr, writer)
}
