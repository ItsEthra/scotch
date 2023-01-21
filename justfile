example-debug: build-plugin-release
	cargo run --package runner

example-release: build-plugin-release
	cargo run --release --package runner

build-plugin-release:
	cargo build --release --package plugin --target wasm32-unknown-unknown

build-plugin-bench:
	RUSTFLAGS="--cfg bench" cargo build --release --package plugin --target wasm32-unknown-unknown

bench: build-plugin-bench
	cargo bench --package host

expand-runner:
	cargo expand --package runner

expand-plugin:
	cargo expand --package plugin
