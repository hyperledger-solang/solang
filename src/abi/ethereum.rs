// ethereum style ABIs
use parser::pt;
use sema::ast::{Namespace, Parameter, Type};
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
    #[serde(skip_serializing_if = "is_false")]
    pub indexed: bool,
}

#[derive(Serialize)]
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

        ABIParam {
            name: param.name.to_string(),
            ty: param.ty.to_signature_string(ns),
            internal_ty: param.ty.to_string(ns),
            components,
            indexed: param.indexed,
        }
    }

    ns.contracts[contract_no]
        .functions
        .iter()
        .filter(|f| match f.visibility {
            pt::Visibility::Public(_) | pt::Visibility::External(_) => true,
            _ => false,
        })
        .map(|f| ABI {
            name: f.name.to_owned(),
            mutability: f.print_mutability(),
            ty: f.ty.to_string(),
            inputs: f.params.iter().map(|p| parameter_to_abi(p, ns)).collect(),
            outputs: f.returns.iter().map(|p| parameter_to_abi(p, ns)).collect(),
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
