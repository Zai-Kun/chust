```sh
mkdir -p crates/chust-wasm/www/ort

cd /tmp
npm pack onnxruntime-web@1.27.0

mkdir -p /tmp/ort-web-package
tar -xf onnxruntime-web-1.27.0.tgz \
    -C /tmp/ort-web-package \
    --strip-components=1

cp -a /tmp/ort-web-package/dist/. \
    /path/to/chust-wasm/ort/
```
