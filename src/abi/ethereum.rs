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
    pub inputs: Vec<ABIParam>,
    // outputs should be skipped if ty is constructor
    pub outputs: Vec<ABIParam>,
    #[serde(rename = "stateMutability")]
    pub mutability: String,
    #[serde(skip_serializing_if = "is_false")]
    pub anonymous: bool,
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

        let ty = if let Type::Struct(_) = param.ty {
            String::from("tuple")
        } else {
            param.ty.to_signature_string(ns)
        };

        ABIParam {
            name: param.name.to_string(),
            ty,
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
            mutability: func.print_mutability(),
            ty: func.ty.to_string(),
            inputs: func
                .params
                .iter()
                .map(|p| parameter_to_abi(p, ns))
                .collect(),
            outputs: func
                .returns
                .iter()
                .map(|p| parameter_to_abi(p, ns))
                .collect(),
            anonymous: false,
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
                        inputs: event
                            .fields
                            .iter()
                            .map(|p| parameter_to_abi(p, ns))
                            .collect(),
                        outputs: Vec::new(),
                        ty: "event".to_owned(),
                        anonymous: event.anonymous,
                    }
                }),
        )
        .collect()
}
