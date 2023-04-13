#!/usr/bin/env just --justfile
set dotenv-load
set export

AWS_PUBLIC_ECR_ACCOUNT_URI := "public.ecr.aws/f1v1j4j7"

list:
    @just --list

help:
    @just list

fmt *FLAGS:
    cargo +nightly fmt {{FLAGS}}

check *FLAGS:
    cargo clippy --all-targets --all-features --workspace {{FLAGS}}

test *FLAGS:
    @just run-signaling-server
    # test on firefox
    cd ./library && wasm-pack test --headless --firefox
    # test on chrome
    cd ./library && wasm-pack test --headless --chrome

run-signaling-server:
    cd ./signaling-server && cargo run &

publish-docker TAG:
    docker login
    docker build -t wasm-peers/signaling-server .
    docker tag wasm-peers/signaling-server tomkarw/wasm-peers-signaling-server:{{TAG}}
    docker tag wasm-peers/signaling-server tomkarw/wasm-peers-signaling-server:latest
    docker push tomkarw/wasm-peers-signaling-server:{{TAG}}
    docker push tomkarw/wasm-peers-signaling-server:latest

publish-aws TAG:
    aws ecr-public get-login-password --region us-east-1 | docker login --username AWS --password-stdin $AWS_PUBLIC_ECR_ACCOUNT_URI
    docker build -t wasm-peers/signaling-server .
    docker tag wasm-peers/signaling-server $AWS_PUBLIC_ECR_ACCOUNT_URI/wasm-peers/signaling-server:{{TAG}}
    docker tag wasm-peers/signaling-server $AWS_PUBLIC_ECR_ACCOUNT_URI/wasm-peers/signaling-server:latest
    docker push $AWS_PUBLIC_ECR_ACCOUNT_URI/wasm-peers/signaling-server:{{TAG}}
    docker push $AWS_PUBLIC_ECR_ACCOUNT_URI/wasm-peers/signaling-server:latest

pre-commit:
    @just fmt
    @just check
    cargo doc --no-deps --all-features
    @just test
    cargo spellcheck fix

coverage *FLAGS:
    cargo llvm-cov {{FLAGS}} --open

thorough-check:
    cargo +nightly udeps --all-targets
    cargo audit
    cargo upgrades
    @just unused-features

unused-features:
    unused-features analyze
    unused-features build-report --input report.json
    rm report.json
    mv report.html /tmp
    xdg-open /tmp/report.html

init:
    echo # installing git hooks
    pre-commit --version || pip install pre-commit
    pre-commit install || echo "failed to install git hooks!" 1>&2
    echo # installing nightly used by `just fmt` and `cargo udeps`
    rustup install nightly
    echo # installing cargo-binstall for faster setup time
    cargo binstall -V || cargo install cargo-binstall
    echo # things required by `just test`
    cargo binstall cargo-nextest --no-confirm
    echo # things required by `just watch`
    cargo binstall cargo-watch --no-confirm
    echo # things required by `just pre-commit`
    cargo binstall cargo-spellcheck --no-confirm
    echo # things required by `just coverage`
    rustup component add llvm-tools-preview
    cargo binstall cargo-llvm-cov --no-confirm
    echo # things required by `just benchmark`
    cargo binstall cargo-criterion --no-confirm
    echo # things required by `just thorough-check`
    cargo binstall cargo-udeps --no-confirm
    cargo binstall cargo-audit --no-confirm
    cargo binstall cargo-upgrades --no-confirm
    cargo binstall cargo-unused-features --no-confirm
    # echo install project specific tools
    cargo binstall wasm-pack
