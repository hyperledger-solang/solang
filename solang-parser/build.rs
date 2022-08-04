// SPDX-License-Identifier: Apache-2.0

fn main() {
    lalrpop::Configuration::new()
        .use_cargo_dir_conventions()
        .emit_rerun_directives(true)
        .process()
        .unwrap();
}
