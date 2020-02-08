// ethereum style ABIs
// This is used by Hyperledger Burrow and ewasm

use parser::ast;
use resolver::{Contract, Type};
use serde::Serialize;

#[derive(Serialize)]
pub struct ABIParam {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: String,
    #[serde(rename = "internalType")]
    pub internal_ty: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub components: Vec<ABIParam>,
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
    fn parameter_to_abi(name: &str, ty: &Type, contract: &Contract) -> ABIParam {
        let components = if let Type::Struct(n) = ty {
            contract.structs[*n]
                .fields
                .iter()
                .map(|f| parameter_to_abi(&f.name, &f.ty, contract))
                .collect::<Vec<ABIParam>>()
        } else {
            Vec::new()
        };

        ABIParam {
            name: name.to_string(),
            ty: ty.to_signature_string(contract),
            internal_ty: ty.to_string(contract),
            components,
        }
    }

    contract
        .constructors
        .iter()
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
            inputs: f
                .params
                .iter()
                .map(|p| parameter_to_abi(&p.name, &p.ty, contract))
                .collect(),
            outputs: f
                .returns
                .iter()
                .map(|p| parameter_to_abi(&p.name, &p.ty, contract))
                .collect(),
        })
        .chain(
            contract
                .functions
                .iter()
                .filter(|f| {
                    if let ast::Visibility::Public(_) = f.visibility {
                        true
                    } else {
                        false
                    }
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
                    inputs: f
                        .params
                        .iter()
                        .map(|p| parameter_to_abi(&p.name, &p.ty, contract))
                        .collect(),
                    outputs: f
                        .returns
                        .iter()
                        .map(|p| parameter_to_abi(&p.name, &p.ty, contract))
                        .collect(),
                }),
        )
        .collect()
}
