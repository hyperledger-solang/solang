// ethereum style ABIs

use parser::ast;
use resolver::{Namespace, Type};
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

impl Type {
    /// Is this type a struct, or an array of structs?
    fn is_struct_or_array_of_struct(&self) -> Option<usize> {
        match self {
            Type::Struct(n) => Some(*n),
            Type::Array(ty, _) => ty.is_struct_or_array_of_struct(),
            _ => None,
        }
    }
}

pub fn gen_abi(contract_no: usize, ns: &Namespace) -> Vec<ABI> {
    fn parameter_to_abi(name: &str, ty: &Type, ns: &Namespace) -> ABIParam {
        let components = if let Some(n) = ty.is_struct_or_array_of_struct() {
            ns.structs[n]
                .fields
                .iter()
                .map(|f| parameter_to_abi(&f.name, &f.ty, ns))
                .collect::<Vec<ABIParam>>()
        } else {
            Vec::new()
        };

        ABIParam {
            name: name.to_string(),
            ty: ty.to_signature_string(ns),
            internal_ty: ty.to_string(ns),
            components,
        }
    }

    ns.contracts[contract_no]
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
                .map(|p| parameter_to_abi(&p.name, &p.ty, ns))
                .collect(),
            outputs: f
                .returns
                .iter()
                .map(|p| parameter_to_abi(&p.name, &p.ty, ns))
                .collect(),
        })
        .chain(
            ns.contracts[contract_no]
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
                        .map(|p| parameter_to_abi(&p.name, &p.ty, ns))
                        .collect(),
                    outputs: f
                        .returns
                        .iter()
                        .map(|p| parameter_to_abi(&p.name, &p.ty, ns))
                        .collect(),
                }),
        )
        .collect()
}
