// SPDX-License-Identifier: Apache-2.0

fn main() {
    println!("cargo:rerun-if-changed=src/solidity.lalrpop");
    lalrpop::Configuration::new()
        .set_in_dir("src")
        .set_out_dir("src")
        .process()
        .unwrap();
}
