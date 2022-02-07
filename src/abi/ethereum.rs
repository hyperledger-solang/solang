// ethereum style ABIs
use crate::parser::pt;
use crate::sema::ast::{Namespace, Parameter, Type};
use serde::Serialize;

#[derive(Serialize)]
#[allow(clippy::upper_case_acronyms)]
pub struct ABIParam {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: String,
    #[serde(rename = "internalType")]
    pub internal_ty: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub components: Vec<ABIParam>,
    #[serde(skip_serializing_if = "is_false")]
    pub indexed: bool,
}

#[derive(Serialize)]
#[allow(clippy::upper_case_acronyms)]
pub struct ABI {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub name: String,
    #[serde(rename = "type")]
    pub ty: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inputs: Option<Vec<ABIParam>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outputs: Option<Vec<ABIParam>>,
    #[serde(rename = "stateMutability")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub mutability: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anonymous: Option<bool>,
}

#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_false(boolean: &bool) -> bool {
    !(*boolean)
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
    fn parameter_to_abi(param: &Parameter, ns: &Namespace) -> ABIParam {
        let components = if let Some(n) = param.ty.is_struct_or_array_of_struct() {
            ns.structs[n]
                .fields
                .iter()
                .map(|p| parameter_to_abi(p, ns))
                .collect::<Vec<ABIParam>>()
        } else {
            Vec::new()
        };

        ABIParam {
            name: param.name_as_str().to_owned(),
            ty: param.ty.to_signature_string(true, ns),
            internal_ty: param.ty.to_string(ns),
            components,
            indexed: param.indexed,
        }
    }

    ns.contracts[contract_no]
        .all_functions
        .keys()
        .filter_map(|function_no| {
            let func = &ns.functions[*function_no];

            if let Some(base_contract_no) = func.contract_no {
                if ns.contracts[base_contract_no].is_library() {
                    return None;
                }

                if func.ty == pt::FunctionTy::Constructor && base_contract_no != contract_no {
                    return None;
                }
            }

            if !matches!(
                func.visibility,
                pt::Visibility::Public(_) | pt::Visibility::External(_)
            ) {
                return None;
            }

            if func.ty == pt::FunctionTy::Modifier || !func.has_body {
                return None;
            }

            Some(func)
        })
        .map(|func| ABI {
            name: func.name.to_owned(),
            mutability: format!("{}", func.mutability),
            ty: func.ty.to_string(),
            inputs: if func.ty == pt::FunctionTy::Function || func.ty == pt::FunctionTy::Constructor
            {
                Some(
                    func.params
                        .iter()
                        .map(|p| parameter_to_abi(p, ns))
                        .collect(),
                )
            } else {
                None
            },
            outputs: if func.ty == pt::FunctionTy::Function {
                Some(
                    func.returns
                        .iter()
                        .map(|p| parameter_to_abi(p, ns))
                        .collect(),
                )
            } else {
                None
            },
            anonymous: None,
        })
        .chain(
            ns.contracts[contract_no]
                .sends_events
                .iter()
                .map(|event_no| {
                    let event = &ns.events[*event_no];

                    ABI {
                        name: event.name.to_owned(),
                        mutability: String::new(),
                        inputs: Some(
                            event
                                .fields
                                .iter()
                                .map(|p| parameter_to_abi(p, ns))
                                .collect(),
                        ),
                        outputs: None,
                        ty: "event".to_owned(),
                        anonymous: Some(event.anonymous),
                    }
                }),
        )
        .collect()
}
