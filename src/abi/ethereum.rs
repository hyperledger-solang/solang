// ethereum style ABIs
// This is used by Hyperledger Burrow and ewasm

use parser::ast;
use resolver::{Contract, Parameter, Type};
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
    fn parameter_to_abi(param: &Parameter, contract: &Contract) -> ABIParam {
        ABIParam {
            name: param.name.to_string(),
            ty: match &param.ty {
                Type::Primitive(e) => e.to_string(),
                Type::Enum(ref i) => contract.enums[*i].ty.to_string(),
                Type::Noreturn => unreachable!(),
            },
        }
    }
    
    contract.constructors.iter()
        .map(|f| ABI {
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
        .chain(contract.functions.iter()
            .filter(|f| if let ast::Visibility::Public(_) = f.visibility {
                true
            } else {
                false
            })
            .map(|f| ABI {
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
        )
        .collect()
}