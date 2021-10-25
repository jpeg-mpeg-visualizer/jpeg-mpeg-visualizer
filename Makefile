all:
	wasm-pack build --target web

serve:
	http-server -c-1
	
pipeline: check test fmt clippy
	
check:
	cargo check

test:
	cargo test

fmt:
	cargo fmt --all -- --check

clippy:
	cargo clippy -- -A clippy::style -D warnings 
	
