#
# Copyright 2019 The Project Oak Authors
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
#c
package(default_visibility = ["//visibility:public"])

load(":cc_toolchain_config.bzl", "cc_toolchain_config")

cc_toolchain_config(name = "clang_llvm_toolchain_config")

cc_toolchain_suite(
    name = "clang_llvm",
    toolchains = {
        "wasm32": ":clang_llvm_toolchain",
    },
)

filegroup(
    name = "all_files",
    srcs = [
        "clang.sh",
        "@clang_llvm//:all_files",
    ],
)

cc_toolchain(
    name = "clang_llvm_toolchain",
    all_files = ":all_files",
    compiler_files = ":all_files",
    dwp_files = ":empty",
    linker_files = ":all_files",
    objcopy_files = ":empty",
    strip_files = ":empty",
    supports_param_files = 0,
    toolchain_config = ":clang_llvm_toolchain_config",
    toolchain_identifier = "clang_llvm-toolchain",
)
