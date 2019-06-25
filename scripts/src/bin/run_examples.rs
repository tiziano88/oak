extern crate glob;
extern crate users;

use glob::glob;
use std::process::Command;

fn build_server() -> std::process::Child {
    run_in_docker(&mut vec![
        "bazel".to_string(),
        "build".to_string(),
        "--config=enc-sim".to_string(),
        "//oak/server:oak".to_string(),
    ])
}

fn run_server() -> std::process::Child {
    run_in_docker(&mut vec![
        "./bazel-bin/oak/server/oak".to_string(),
        "--grpc_port=8888".to_string(),
    ])
}

fn run_in_docker(args: &mut Vec<String>) -> std::process::Child {
    let docker_uid = users::get_current_uid();
    let docker_gid = users::get_current_gid();
    let docker_user = users::get_current_username()
        .expect("could not get username")
        .into_string()
        .expect("could not parse username");
    let mut docker_args = vec![
        "run".to_string(),
        "--tty".to_string(),
        "--rm".to_string(),
        format!("--user={}:{}", docker_uid, docker_gid),
        format!("--env=USER={}", docker_user),
        format!(
            "--volume={}:/.cache/bazel",
            std::fs::canonicalize("bazel-cache")
                .unwrap()
                .to_str()
                .unwrap()
        ),
        format!(
            "--volume={}:/usr/local/cargo/registry",
            std::fs::canonicalize("cargo-cache")
                .unwrap()
                .to_str()
                .unwrap()
        ),
        format!(
            "--volume={}:/opt/my-project",
            std::fs::canonicalize(".").unwrap().to_str().unwrap()
        ),
        "--workdir=/opt/my-project".to_string(),
        "--network=host".to_string(),
        "oak:latest".to_string(),
    ];
    docker_args.append(args);
    let mut cmd = Command::new("docker");
    cmd.args(docker_args);
    println!("command: {:?}", cmd);
    cmd.spawn().expect("could not run docker command")
}

fn main() {
    std::fs::create_dir_all("bazel-cache").unwrap();
    std::fs::create_dir_all("cargo-cache").unwrap();
    build_server().wait().unwrap();
    let mut server = run_server();
    server.wait().unwrap();
    for entry in glob("examples/**/run").expect("could not find matches") {
        let entry = entry.expect("could not parse match");
    }
    println!("Hello World!");
}
