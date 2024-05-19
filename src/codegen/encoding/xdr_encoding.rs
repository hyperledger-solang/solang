use soroban_sdk::xdr;

use crate::codegen::cfg::{ControlFlowGraph, Instr};
use crate::codegen::encoding::AbiEncoding;
use crate::codegen::vartable::Vartable;
use crate::codegen::{Builtin, Expression};
use crate::sema::ast::StructType;
use crate::sema::ast::{Namespace, Type, Type::Uint};

use primitive_types::U256;
use solang_parser::pt::Loc::Codegen;
use std::collections::HashMap;

use super::buffer_validator::BufferValidator;

pub(super) struct XDREncoding {
    storage_cache: HashMap<usize, Expression>,
    packed_encoder: bool,
}

impl XDREncoding {
    pub fn new(packed_encoder: bool) -> Self {
        Self {
            storage_cache: HashMap::new(),
            packed_encoder,
        }
    }
}

impl AbiEncoding for XDREncoding {
    fn size_width(
        &self,
        size: &Expression,
        vartab: &mut Vartable,
        cfg: &mut ControlFlowGraph,
    ) -> Expression {
        Expression::NumberLiteral {
            loc: Codegen,
            ty: Uint(32),
            value: 4.into(),
        }
    }

    fn encode_external_function(
        &mut self,
        expr: &Expression,
        buffer: &Expression,
        offset: &Expression,
        ns: &Namespace,
        vartab: &mut Vartable,
        cfg: &mut ControlFlowGraph,
    ) -> Expression {
        todo!("encode_external_function")
    }

    fn encode_size(
        &mut self,
        expr: &Expression,
        buffer: &Expression,
        offset: &Expression,
        _ns: &Namespace,
        vartab: &mut Vartable,
        cfg: &mut ControlFlowGraph,
    ) -> Expression {
        println!("encode_size: {:?}", expr);
        Expression::NumberLiteral {
            loc: Codegen,
            ty: Uint(32),
            value: 4.into(),
        }
        //encode_compact(expr, Some(buffer), Some(offset), vartab, cfg)
    }

    fn decode_external_function(
        &self,
        buffer: &Expression,
        offset: &Expression,
        ty: &Type,
        validator: &mut BufferValidator,
        ns: &Namespace,
        vartab: &mut Vartable,
        cfg: &mut ControlFlowGraph,
    ) -> (Expression, Expression) {
        todo!("decode_external_function")
    }

    fn retrieve_array_length(
        &self,
        buffer: &Expression,
        offset: &Expression,
        vartab: &mut Vartable,
        cfg: &mut ControlFlowGraph,
    ) -> (usize, Expression) {
        todo!("retrieve_array_length")
    }

    fn storage_cache_insert(&mut self, arg_no: usize, expr: Expression) {
        self.storage_cache.insert(arg_no, expr);
    }

    fn storage_cache_remove(&mut self, arg_no: usize) -> Option<Expression> {
        self.storage_cache.remove(&arg_no)
    }

    fn calculate_string_size(
        &self,
        expr: &Expression,
        vartab: &mut Vartable,
        cfg: &mut ControlFlowGraph,
    ) -> Expression {
        todo!("calculate_string_size")
    }

    fn is_packed(&self) -> bool {
        self.packed_encoder
    }

    /// TODO: This is used and tested for error data (Error and Panic) only.
    fn const_encode(&self, args: &[Expression]) -> Option<Vec<u8>> {
        todo!("const_encode")
    }
}
