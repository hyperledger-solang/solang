// ethereum style ABIs
// This is used by Hyperledger Burrow

use parser::ast;
use resolver::{Contract, Parameter, TypeName};
use serde::Serialize;

#[derive(Serialize)]
pub struct ABIParam {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: String,
}

#[derive(Serialize)]
pub struct ABI {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: String,
    pub inputs: Vec<ABIParam>,
    pub outputs: Vec<ABIParam>,
    pub constant: bool,
    pub payable: bool,
    #[serde(rename = "stateMutability")]
    pub mutability: &'static str,
}

pub fn gen_abi(contract: &Contract) -> Vec<ABI> {
    let mut abis = Vec::new();

    for f in &contract.constructors {
        abis.push(ABI {
            name: "".to_owned(),
            constant: match &f.cfg {
                Some(cfg) => !cfg.writes_contract_storage,
                None => false,
            },
            mutability: match &f.mutability {
                Some(n) => n.to_string(),
                None => "nonpayable",
            },
            payable: match &f.mutability {
                Some(ast::StateMutability::Payable(_)) => true,
                _ => false,
            },
            ty: "constructor".to_owned(),
            inputs: f.params.iter().map(|p| parameter_to_abi(p, contract)).collect(),
            outputs: f.returns.iter().map(|p| parameter_to_abi(p, contract)).collect(),
        })

    }

    for f in &contract.functions {
        abis.push(ABI {
            name: f.name.to_owned(),
            constant: match &f.cfg {
                Some(cfg) => !cfg.writes_contract_storage,
                None => false,
            },
            mutability: match &f.mutability {
                Some(n) => n.to_string(),
                None => "nonpayable",
            },
            payable: match &f.mutability {
                Some(ast::StateMutability::Payable(_)) => true,
                _ => false,
            },
            ty: if f.name == "" {
                "fallback".to_owned()
            } else {
                "function".to_owned()
            },
            inputs: f.params.iter().map(|p| parameter_to_abi(p, contract)).collect(),
            outputs: f.returns.iter().map(|p| parameter_to_abi(p, contract)).collect(),
        })
    }

    abis
}

fn parameter_to_abi(param: &Parameter, contract: &Contract) -> ABIParam {
    ABIParam {
        name: param.name.to_string(),
        ty: match &param.ty {
            TypeName::Elementary(e) => e.to_string(),
            TypeName::Enum(ref i) => contract.enums[*i].ty.to_string(),
            TypeName::Noreturn => unreachable!(),
        },
    }
}

