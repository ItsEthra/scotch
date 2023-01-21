example-debug: build-plugin
	cargo run --package runner

example-release: build-plugin
	cargo run --release --package runner

build-plugin:
	cargo build --release --package plugin --target wasm32-unknown-unknown

expand-runner:
	cargo expand --package runner

expand-plugin:
	cargo expand --package plugin
