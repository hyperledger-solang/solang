// SPDX-License-Identifier: Apache-2.0

mod enums;

#[allow(clippy::unreadable_literal, clippy::naive_bytecount)]
mod expressions;

mod abi;
mod array_boundary_check;
mod arrays;
mod builtins;
mod calls;
mod contracts;
mod debug_buffer_format;
mod errors;
mod events;
mod first;
mod format;
mod function_types;
mod functions;
mod imports;
mod inheritance;
mod libraries;
mod loops;
mod mappings;
mod modifier;
mod primitives;
mod statements;
mod storage;
mod strings;
mod structs;
mod value;
mod variables;
mod yul;
