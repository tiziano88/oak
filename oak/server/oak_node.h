/*
 * Copyright 2019 The Project Oak Authors
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

#ifndef OAK_SERVER_OAK_NODE_H_
#define OAK_SERVER_OAK_NODE_H_

#include <unordered_map>

#include "absl/base/thread_annotations.h"
#include "absl/types/span.h"
#include "oak/proto/node.grpc.pb.h"
#include "oak/server/channel.h"
#include "src/interp/interp.h"

namespace oak {

using ChannelId = uint64_t;

class OakNode final : public Node::Service {
 public:
  OakNode(const std::string& node_id, const std::string& module);

  // Performs an Oak Module invocation.
  grpc::Status ProcessModuleInvocation(grpc::GenericServerContext* context,
                                       const std::vector<char>& request_data,
                                       std::vector<char>* response_data);

 private:
  grpc::Status GetAttestation(grpc::ServerContext* context, const GetAttestationRequest* request,
                              GetAttestationResponse* response) override;

  void InitEnvironment(wabt::interp::Environment* env);

  // Native implementation of the `oak.open_channel` host function.
  wabt::interp::HostFunc::Callback OakOpenChannel(wabt::interp::Environment* env);

  // Native implementation of the `oak.read_channel` host function.
  wabt::interp::HostFunc::Callback OakReadChannel(wabt::interp::Environment* env);

  // Native implementation of the `oak.write_channel` host function.
  wabt::interp::HostFunc::Callback OakWriteChannel(wabt::interp::Environment* env);

  ChannelId NewChannelId();

  wabt::interp::Environment env_;
  // TODO: Use smart pointers.
  wabt::interp::DefinedModule* module_;

  // Incoming gRPC data for the current invocation.
  const ::grpc::GenericServerContext* server_context_;

  std::unordered_map<ChannelId, std::unique_ptr<Channel>> channels_;

  std::string grpc_method_name_;

  // Unique ID of the Oak Node instance. Creating multiple Oak Nodes with the same module and policy
  // configuration will result in Oak Node instances with distinct node_id_.
  const std::string node_id_;

  // Hash of the Oak Module with which this Oak Node was initialized.
  // To be used as the basis for remote attestation based on code identity.
  const std::string module_hash_sha_256_;

  ChannelId channel_id_;
};

}  // namespace oak

#endif  // OAK_SERVER_OAK_NODE_H_
