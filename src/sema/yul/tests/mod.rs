// SPDX-License-Identifier: Apache-2.0

#![cfg(test)]

use crate::{parse_and_resolve, sema::ast, FileResolver, Target};
use std::ffi::OsStr;

mod block;
mod expression;
mod for_loop;
mod functions;
mod mutability;
mod statements;
mod switch;
mod types;
mod unused_variable;

pub(crate) fn parse(src: &'static str) -> ast::Namespace {
    let mut cache = FileResolver::default();
    cache.set_file_contents("test.sol", src.to_string());

    let ns = parse_and_resolve(OsStr::new("test.sol"), &mut cache, Target::EVM);
    ns.print_diagnostics_in_plain(&cache, false);
    ns
}
