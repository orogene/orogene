#![allow(clippy::single_component_path_imports)]

use cc;
use ci_info::is_ci;

fn main() -> std::io::Result<()> {
    let ci = is_ci();

    if !ci {
        /*

        use std::{path::Path, process::Command};

        let mut search_dir = std::env::current_dir()?;

        search_dir.push("vendor/node/out/Release");

        println!("cargo:rustc-link-search={}", search_dir.to_str().unwrap());

        println!("cargo:rustc-link-lib=static=node");
        println!("cargo:rustc-link-lib=static=uv");
        println!("cargo:rustc-link-lib=static=uvwasi");

        // temporary fix - https://github.com/nodejs/node/issues/27431#issuecomment-487288275
        println!("cargo:rustc-link-lib=static=_stub_code_cache");
        println!("cargo:rustc-link-lib=static=_stub_snapshot");
        // end temporary fix

        println!("cargo:rustc-link-lib=static=v8_base_without_compiler");
        println!("cargo:rustc-link-lib=static=v8_compiler");
        println!("cargo:rustc-link-lib=static=v8_initializers");
        println!("cargo:rustc-link-lib=static=v8_libbase");
        println!("cargo:rustc-link-lib=static=v8_libplatform");
        println!("cargo:rustc-link-lib=static=v8_libsampler");
        println!("cargo:rustc-link-lib=static=v8_snapshot");
        println!("cargo:rustc-link-lib=static=v8_zlib");

        println!("cargo:rustc-link-lib=static=icuucx");
        println!("cargo:rustc-link-lib=static=icui18n");
        println!("cargo:rustc-link-lib=static=icudata");

        println!("cargo:rustc-link-lib=static=zlib");
        println!("cargo:rustc-link-lib=static=brotli");
        println!("cargo:rustc-link-lib=static=cares");
        println!("cargo:rustc-link-lib=static=histogram");
        println!("cargo:rustc-link-lib=static=llhttp");
        println!("cargo:rustc-link-lib=static=nghttp2");
        println!("cargo:rustc-link-lib=static=openssl");
        println!("cargo:rustc-link-lib=static=torque_base");

        let stub_dir = Path::new(search_dir.as_os_str()).join("/obj.target/cctest/src/");
        let code_cache_stub = Path::new(stub_dir.as_os_str()).join("node_code_cache_stub.o");
        let snapshot_stub = Path::new(stub_dir.as_os_str()).join("node_snapshot_stub.o");
        let code_cache_lib = Path::new(search_dir.as_os_str()).join("lib_stub_code_cache.a");
        let snapshot_lib = Path::new(search_dir.as_os_str()).join("lib_stub_snapshot.a");

        Command::new("ar")
            .arg("rcs")
            .arg(code_cache_lib)
            .arg(code_cache_stub)
            .output()?;

        Command::new("ar")
            .arg("rcs")
            .arg(snapshot_lib)
            .arg(snapshot_stub)
            .output()?; */

        cc::Build::new()
            .cpp(true)
            .file("node.cpp")
            .compile("liboronode.a");
    }

    Ok(())
}
