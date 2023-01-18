example-debug: build-example-plugin
	cargo run --package runner

example-release: build-example-plugin
	cargo run --package runner

build-example-plugin:
	cargo build --release --package plugin --target wasm32-unknown-unknown

expand:
	cargo expand --package runner
