# Obelisk

Uses [Node.js C++ API](https://nodejs.org/docs/latest-v14.x/api/embedding.html) to embed Node in Orogene.

## Build

- Copy files from `node_c_api` (which are needed to turn C++ API into C API and prevent function name mangling) to `vendor/node/src`
- Run the following:

```sh
cd vendor/node
./configure --enable-static
make -j4
```

After the build succeeds you need to manually put two stubs in the output folder (see [this issue](https://github.com/nodejs/node/issues/27431#issuecomment-487288275)):

```
REL=out/Release
STUBS=$REL/obj.target/cctest/src
ar rcs $REL/lib_stub_code_cache.a $STUBS/node_code_cache_stub.o
ar rcs $REL/lib_stub_snapshot.a $STUBS/node_snapshot_stub.o
```

Finally, build `obelisk` itself with `cargo build` and test with `cargo test`.
