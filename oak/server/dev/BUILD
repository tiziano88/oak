#
# Copyright 2018 The Project Oak Authors
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.
#

package(
    default_visibility = ["//oak/server:__subpackages__"],
)

cc_library(
    name = "dev_oak_manager",
    srcs = [
        "dev_oak_manager.cc",
    ],
    hdrs = [
        "dev_oak_manager.h",
    ],
    deps = [
        "//oak/proto:application_cc_grpc",
        "//oak/proto:enclave_cc_proto",
        "//oak/proto:manager_cc_grpc",
        "//oak/server:oak_runtime",
        "@com_github_grpc_grpc//:grpc++",
        "@com_google_absl//absl/memory",
        "@com_google_absl//absl/strings",
        "@com_google_asylo//asylo/grpc/auth:null_credentials_options",
        "@com_google_asylo//asylo/identity:init",
        "@com_google_asylo//asylo/util:logging",
    ],
)

cc_binary(
    name = "oak",
    srcs = ["dev_oak_main.cc"],
    deps = [
        ":dev_oak_manager",
        "@com_github_grpc_grpc//:grpc++",
        "@com_google_absl//absl/flags:flag",
        "@com_google_absl//absl/flags:parse",
        "@com_google_absl//absl/synchronization",
        "@com_google_absl//absl/time",
        "@com_google_asylo//asylo/util:logging",
    ],
)
