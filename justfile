# https://just.systems

test:
cargo nextest run

build-wasm-web:
wasm-pack build --target web --out-dir pkg

publish-wasm:
wasm-pack publish

build-wasm-typst:
cargo build --target wasm32-unknown-unknown --release --features typst-plugin
