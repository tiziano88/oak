workflow "New workflow" {
  on = "push"
  resolves = ["GitHub Action for Docker-1"]
}

action "GitHub Action for Docker" {
  uses = "actions/docker/cli@86ab5e854a74b50b7ed798a94d9b8ce175d8ba19"
  args = "build --tag=oak ."
}

action "GitHub Action for Docker-1" {
  uses = "actions/docker/cli@86ab5e854a74b50b7ed798a94d9b8ce175d8ba19"
  needs = ["GitHub Action for Docker"]
  args = "run --interactive --tty oak:latest ./scripts/check_formatting"
}
