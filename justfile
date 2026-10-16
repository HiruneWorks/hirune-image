set dotenv-load := true
set dotenv-path := ".envs/.local/app.env"
set windows-shell := ["powershell.exe", "-NoLogo", "-NoProfile", "-Command"]
python_command := if os() == "windows" { "python" } else { "python3" }

default:
    @just --list

build:
    cargo build --locked --bin api --bin image_worker

api: build
    cargo run --locked --bin api

fmt:
    cargo fmt --all

check:
    cargo xtask check

test-api:
    cargo xtask test-api

test-process:
    cargo xtask test-process

contracts:
    cargo xtask contracts

test-container:
    {{python_command}} scripts/verify_container.py

bench:
    cargo bench --locked --bench image_pipeline

bench-services *arguments:
    {{python_command}} scripts/benchmark_services.py {{arguments}}

test-resources:
    cargo test --locked --test resources
