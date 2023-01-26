example-debug: build-plugin-release
	cargo run --package runner

example-release: build-plugin-release
	cargo run --release --package runner

build-plugin-release:
	cargo build --release --package plugin --target wasm32-unknown-unknown

build-plugin-bench:
	RUSTFLAGS="--cfg bench" cargo build --release --package plugin --target wasm32-unknown-unknown

bench: build-plugin-bench
	cargo bench --package scotch-host

expand-runner:
	cargo expand --package runner

expand-plugin:
	cargo expand --package plugin

doc-host:
	cargo doc --package scotch-host --features unstable-doc-cfg,flate2 --open

doc-guest:
	cargo doc --package scotch-guest --open
