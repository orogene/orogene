# Obelisk

Uses [Node.js C++ API](https://nodejs.org/docs/latest-v14.x/api/embedding.html) to embed Node in Orogene.

## Build

_Currently only works on Mac OS_

First, run the JS script that copies the embedder API wrapper to Node's source and modifies build configuration:

```
npm install
node prepare.js
```

Hint: make sure you fetch `node` repository files first!

Then build Node:

```sh
cd vendor/node
./configure --enable-static
make -j4
```

## Test

Build `obelisk` itself with `cargo build` and run `cargo run fixtures/test.js`.
