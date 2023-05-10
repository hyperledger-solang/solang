// SPDX-License-Identifier: Apache-2.0

use std::process::Command;

fn main() {
    #[cfg(feature = "llvm")]
    {
        Command::new("make")
            .args(["-C", "stdlib"])
            .output()
            .expect("Could not build stdlib");

        // compile our linker
        let cxxflags = Command::new("llvm-config")
            .args(["--cxxflags"])
            .output()
            .expect("could not execute llvm-config");

        let cxxflags = String::from_utf8(cxxflags.stdout).unwrap();

        let mut build = cc::Build::new();

        build.file("src/linker/linker.cpp").cpp(true);

        if !cfg!(target_os = "windows") {
            build.flag("-Wno-unused-parameter");
        }

        for flag in cxxflags.split_whitespace() {
            build.flag(flag);
        }

        build.compile("liblinker.a");

        // add the llvm linker
        let libdir = Command::new("llvm-config")
            .args(["--libdir"])
            .output()
            .unwrap();
        let libdir = String::from_utf8(libdir.stdout).unwrap();

        println!("cargo:libdir={libdir}");
        for lib in &["lldELF", "lldCommon", "lldWasm"] {
            println!("cargo:rustc-link-lib=static={lib}");
        }

        // And all the symbols we're not using, needed by Windows and debug builds
        println!("cargo:rustc-link-lib=static=lldMachO");
    }

    let output = Command::new("git")
        .args(["describe", "--tags", "--always"])
        .output()
        .unwrap();
    let solang_version = if output.stdout.is_empty() {
        format!("v{}", env!("CARGO_PKG_VERSION"))
    } else {
        String::from_utf8(output.stdout).unwrap()
    };

    println!("cargo:rustc-env=SOLANG_VERSION={solang_version}");

    // Make sure we have an 8MiB stack on Windows. Windows defaults to a 1MB
    // stack, which is not big enough for debug builds
    #[cfg(windows)]
    println!("cargo:rustc-link-arg=/STACK:8388608");
}
