---
name: build
description: build the rust project
disable-model-invocation: false
---

cargo clean
cargo  update
cargo build -r
