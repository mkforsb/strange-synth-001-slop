# Requires the Dioxus CLI (`dx`, 0.7.x) and the wasm32-unknown-unknown target.

PUBLIC := target/dx/strange-attractor/release/web/public

.PHONY: web

## Release web build with relative asset paths, servable from any directory
web:
	dx bundle --platform web --release
	python3 tools/relative_paths.py $(PUBLIC)
