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

//! A utility binary to run Oak Runtime.
//!
//! Invoke with:
//!
//! ```shell
//! cargo run --package=oak_loader -- \
//!     --application=<APP_CONFIG_PATH> \
//!     --grpc-tls-private-key=<PRIVATE_KEY_PATH> \
//!     --grpc-tls-certificate=<CERTIFICATE_PATH> \
//!     --root-tls-certificate=<CERTIFICATE_PATH>
//! ```

use anyhow::anyhow;
use core::str::FromStr;
use oak_abi::{
    proto::oak::application::{ApplicationConfiguration, ConfigMap},
    Handle,
};
use oak_runtime::{
    auth::oidc_utils::parse_client_info_json,
    config::{configure_and_run, load_certificate},
    io::Sender,
    RuntimeProxy,
};
use prost::Message;
use slog::{debug, error, info, o, Drain};
use std::{
    collections::HashMap,
    fs::{read_to_string, File},
    io::Read,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use structopt::StructOpt;
use tonic::transport::Identity;

#[cfg(test)]
mod tests;

/// Command line options for the Oak loader.
#[derive(StructOpt, Clone, Debug)]
#[structopt(about = "Oak Loader")]
pub struct Opt {
    #[structopt(long, help = "Application configuration file.")]
    application: String,
    #[structopt(long, help = "Private RSA key file used by gRPC server pseudo-Nodes.")]
    grpc_tls_private_key: String,
    #[structopt(
        long,
        help = "PEM encoded X.509 TLS certificate file used by gRPC server pseudo-Nodes."
    )]
    grpc_tls_certificate: String,
    #[structopt(
        long,
        help = "PEM encoded X.509 TLS root certificate file used to authenticate an external gRPC service."
    )]
    root_tls_certificate: String,
    #[structopt(
        long,
        help = "Path to the downloaded JSON-encoded client identity file for OpenID Connect. \
        OpenID Connect authentication will not be available if this parameter is not specified."
    )]
    oidc_client: Option<String>,
    #[structopt(long, default_value = "9090", help = "Metrics server port number.")]
    metrics_port: u16,
    #[structopt(long, help = "Starts the Runtime without a metrics server.")]
    no_metrics: bool,
    #[structopt(
        long,
        default_value = "1909",
        help = "Introspection server port number."
    )]
    introspect_port: u16,
    #[structopt(long, help = "Starts the Runtime without an introspection server.")]
    no_introspect: bool,
    #[structopt(
        long,
        help = "Configuration files to expose to the Oak Application, each in key=filename format."
    )]
    config_files: Vec<ConfigEntry>,
}

/// A specification of a configuration entry as human readable key and a path to a file whose
/// contents constitutes the actual value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfigEntry {
    key: String,
    filename: String,
}

impl FromStr for ConfigEntry {
    type Err = anyhow::Error;
    fn from_str(v: &str) -> Result<Self, Self::Err> {
        let parts = v.split('=').collect::<Vec<_>>();
        if parts.len() != 2 {
            return Err(anyhow!("could not parse config entry: {}", v));
        }
        Ok(ConfigEntry {
            key: parts[0].to_string(),
            filename: parts[1].to_string(),
        })
    }
}

/// Read the contents of a file.
fn read_file(filename: &str) -> anyhow::Result<Vec<u8>> {
    let mut file = File::open(filename)
        .map_err(|error| anyhow!("Failed to open file <{}>: {:?}", filename, error))?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)
        .map_err(|error| anyhow!("Failed to read file <{}>: {:?}", filename, error))?;
    Ok(buffer)
}

/// Parse a collection of configuration entries and return the contents of
/// the corresponding files as a [`ConfigMap`].
pub fn parse_config_map(config_entries: &[ConfigEntry]) -> anyhow::Result<ConfigMap> {
    let mut file_map = HashMap::new();
    for config_entry in config_entries {
        if file_map.contains_key(&config_entry.key) {
            return Err(anyhow!("duplicate config entry key: {}", config_entry.key));
        }
        file_map.insert(
            config_entry.key.to_string(),
            read_file(&config_entry.filename)?,
        );
    }
    Ok(ConfigMap { items: file_map })
}

/// Send configuration map to the initial Oak Node.
fn send_config_map(
    config_map: ConfigMap,
    runtime: &RuntimeProxy,
    handle: Handle,
) -> anyhow::Result<()> {
    let sender = Sender::new(handle);
    sender.send(config_map, runtime).map_err(|status| {
        anyhow!(
            "could not send configuration map to the initial Node: {:?}",
            status
        )
    })
}

/// Main execution point for the Oak loader.
fn main() -> anyhow::Result<()> {
    // slog_stdlog::init();

    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::CompactFormat::new(decorator).build().fuse();
    // let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain).chan_size(1024).build().fuse();
    let log = slog::Logger::root(drain, o!());

    slog::info!(log, "Logging ready!");

    if cfg!(feature = "oak_debug") {
        // env_logger::init();
    } else {
        eprintln!("No debugging output configured at build time");
    }
    let opt = Opt::from_args();
    debug!(log, "parsed opts: {:?}", opt);

    let config_map = parse_config_map(&opt.config_files)?;
    // We only log the keys here, since the values may be secret.
    debug!(
        log,
        "parsed config map entries: {:?}",
        config_map.items.keys()
    );
    // TODO(#689): Pass the `config_map` object to the Runtime instance, and make it available to
    // the running Oak Application.

    // Load application configuration.
    let app_config_data = read_file(&opt.application)?;
    let application_configuration = ApplicationConfiguration::decode(app_config_data.as_ref())?;

    // Create Runtime config.
    let runtime_configuration = oak_runtime::RuntimeConfiguration {
        metrics_port: if cfg!(feature = "oak_debug") && !opt.no_metrics {
            Some(opt.metrics_port)
        } else {
            None
        },
        introspect_port: if cfg!(feature = "oak_debug") && !opt.no_introspect {
            Some(opt.introspect_port)
        } else {
            None
        },
    };

    // Create the overall gRPC configuration.
    let grpc_tls_private_key = read_to_string(&opt.grpc_tls_private_key)?;
    let grpc_tls_certificate = read_to_string(&opt.grpc_tls_certificate)?;
    let root_tls_certificate = read_to_string(&opt.root_tls_certificate)?;
    let oidc_client_info = match opt.oidc_client {
        Some(oidc_client) => {
            let oidc_file = read_to_string(oidc_client)?;
            Some(
                parse_client_info_json(&oidc_file)
                    .map_err(|error| anyhow!("Failed to parse json: {:?}", error))?,
            )
        }
        _ => None,
    };
    let grpc_configuration = oak_runtime::GrpcConfiguration {
        grpc_server_tls_identity: Some(Identity::from_pem(
            grpc_tls_certificate,
            grpc_tls_private_key,
        )),
        grpc_client_root_tls_certificate: Some(
            load_certificate(&root_tls_certificate)
                .map_err(|()| anyhow!("could not parse TLS certificate"))?,
        ),
        oidc_client_info,
    };

    // Start the Runtime from the given config.
    info!(log, "starting Runtime, config {:?}", runtime_configuration);
    let (runtime, initial_handle) = configure_and_run(
        log.clone(),
        application_configuration,
        runtime_configuration,
        grpc_configuration,
    )
    .map_err(|status| anyhow!("could not start runtime, status: {:?}", status))?;
    info!(
        log,
        "initial node";
        "node_id" => ?runtime.node_id,
        "initial_handle" => initial_handle,
    );

    // Pass in the config map over the initial channel.
    send_config_map(config_map, &runtime, initial_handle)?;
    if let Err(err) = runtime.channel_close(initial_handle) {
        error!(
            log,
            "Failed to close initial handle";
            "initial_handle" => initial_handle,
            "err" => ?err
        );
    }

    let done = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::SIGINT, Arc::clone(&done))?;

    // The Runtime kicks off its own threads for the initial Node and any subsequently created
    // Nodes, so just block the current thread until a signal arrives.
    while !done.load(Ordering::Relaxed) {
        // There are few synchronization mechanisms that are allowed to be used in a signal
        // handler context, so use a primitive sleep loop to watch for the termination
        // notification (rather than something more accurate like `std::sync::Condvar`).
        // See e.g.: http://man7.org/linux/man-pages/man7/signal-safety.7.html
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    info!(log, "stop Runtime");
    runtime.stop_runtime();

    info!(log, "Runtime stopped");
    Ok(())
}
