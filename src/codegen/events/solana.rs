// SPDX-License-Identifier: Apache-2.0

use crate::codegen::cfg::{ControlFlowGraph, Instr};
use crate::codegen::encoding::abi_encode;
use crate::codegen::events::EventEmitter;
use crate::codegen::expression::expression;
use crate::codegen::vartable::Vartable;
use crate::codegen::{Expression, Options};
use crate::sema::ast;
use crate::sema::ast::{Function, Namespace, Type};
use sha2::{Digest, Sha256};
use solang_parser::pt::Loc;

/// This struct implements the trait 'EventEmitter' to handle the emission of events for Solana.
pub(super) struct SolanaEventEmitter<'a> {
    pub(super) loc: Loc,
    /// Arguments passed to the event
    pub(super) args: &'a [ast::Expression],
    pub(super) ns: &'a Namespace,
    pub(super) event_no: usize,
}

impl EventEmitter for SolanaEventEmitter<'_> {
    fn emit(
        &self,
        contract_no: usize,
        func: &Function,
        cfg: &mut ControlFlowGraph,
        vartab: &mut Vartable,
        opt: &Options,
    ) {
        let discriminator_image = format!("event:{}", self.ns.events[self.event_no].name);
        let mut hasher = Sha256::new();
        hasher.update(discriminator_image.as_bytes());
        let result = hasher.finalize();

        let discriminator =
            Expression::BytesLiteral(Loc::Codegen, Type::Bytes(8), result[..8].to_vec());

        let mut codegen_args = self
            .args
            .iter()
            .map(|e| expression(e, cfg, contract_no, Some(func), self.ns, vartab, opt))
            .collect::<Vec<Expression>>();

        let mut to_be_encoded: Vec<Expression> = vec![discriminator];
        to_be_encoded.append(&mut codegen_args);

        let (abi_encoded, abi_encoded_size) =
            abi_encode(&self.loc, to_be_encoded, self.ns, vartab, cfg, false);

        cfg.add(
            vartab,
            Instr::EmitEvent {
                event_no: self.event_no,
                data: vec![abi_encoded, abi_encoded_size],
                topics: vec![],
            },
        );
    }
}
