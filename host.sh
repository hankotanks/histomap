wasm-pack build --target web --no-pack --out-dir ./pkg
cp -r static/* pkg
cp -r js/* pkg
miniserve pkg --index "index.html" -p 8080