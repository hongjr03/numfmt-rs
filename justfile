# https://just.systems

test:
    cargo nextest run

build-wasm-web:
    wasm-pack build --target web --out-dir pkg

publish-wasm:
    wasm-pack publish