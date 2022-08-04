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
    let mut cache = FileResolver::new();
    cache.set_file_contents("test.sol", src.to_string());

    parse_and_resolve(OsStr::new("test.sol"), &mut cache, Target::Ewasm)
}
