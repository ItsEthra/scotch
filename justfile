example-debug: build-example-plugin
	cargo run --package runner --quiet

example-release: build-example-plugin
	cargo run --package runner --quiet

build-example-plugin:
	cargo build --release --package plugin --target wasm32-unknown-unknown --quiet

expand-runner:
	cargo expand --package runner

expand-plugin:
	cargo expand --package plugin
