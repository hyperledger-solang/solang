// Parity Substrate style ABIs/metadata

use parser::ast;
use resolver;
use serde::Serialize;

/// Substrate contracts abi consists of a a registry of strings and types, the contract itself
#[derive(Serialize)]
pub struct Metadata {
    registry: Registry,
    storage: Storage,
    contract: Contract
}

/// The registry holds strings and types. Presumably this is to avoid duplication
#[derive(Serialize)]
struct Registry {
    strings: Vec<String>,
    types: Vec<Type>
}

#[derive(Serialize)]
#[serde(untagged)]
enum Type {
    Builtin {
        id: String,
        def: String
    },
    Struct {
        id: CustomID,
        def: StructDef
    }
}

#[derive(Serialize)]
struct Contract {
    name: usize,
    constructors: Vec<Constructor>,
    messages: Vec<Message>,
}

#[derive(Serialize)]
struct BuiltinType {
    id: String,
    def: String
}

#[derive(Serialize)]
struct StructType {
    id: CustomID,
    def: StructDef
}

#[derive(Serialize)]
struct CustomID {
    #[serde(rename = "custom.name")]
    name: usize,
    #[serde(rename = "custom.namespace")]
    namespace: Vec<usize>,
    #[serde(rename = "custom.params")]
    params: Vec<usize>,
}

#[derive(Serialize)]
struct StructDef {
    #[serde(rename = "struct.fields")]
    fields: Vec<StructField>
}

#[derive(Serialize)]
struct StructField {
    name: usize,
    #[serde(rename = "type")]
    ty: usize
}

#[derive(Serialize)]
struct Constructor {
    name: usize,
    selector: u32,
    args: Vec<Param>
}

#[derive(Serialize)]
struct Message {
    name: usize,
    selector: u32,
    mutates: bool,
    args: Vec<Param>,
    return_type: Option<ParamType>,
}

#[derive(Serialize)]
struct Param {
    name: usize,
    #[serde(rename = "type")]
    ty: ParamType,
}

#[derive(Serialize)]
struct ParamType {
    ty: usize,
    display_name: Vec<usize>
}

#[derive(Serialize)]
struct Storage {
    #[serde(rename = "struct.type")]
    ty: usize,
    #[serde(rename = "struct.fields")]
    fields: Vec<StorageLayout>
}

#[derive(Serialize)]
struct LayoutField {
    #[serde(rename = "range.offset")]
    offset: String,
    #[serde(rename = "range.len")]
    len: usize,
    #[serde(rename = "range.elem_type")]
    ty: usize
}

#[derive(Serialize)]
struct StorageLayout {
    name: usize,
    layout: StorageFieldLayout
}

#[derive(Serialize)]
#[serde(untagged)]
enum StorageFieldLayout {
    Field(LayoutField),
    Storage(Box<Storage>)
}

/// Create a new registry and create new entries. Note that the registry is
/// accessed by number, and the first entry is 1, not 0.
impl Registry {
    fn new() -> Self {
        Registry{
            strings: Vec::new(),
            types: Vec::new(),
        }
    }

    /// Returns index to string in registry. String is added if not already present
    fn string(&mut self, name: &str) -> usize {
        for (i, s) in self.strings.iter().enumerate() {
            if s == name {
                return i + 1;
            }
        }

        let length = self.strings.len();

        self.strings.push(name.to_owned());

        length + 1
    }

    /// Returns index to builtin type in registry. Type is added if not already present
    fn builtin_type(&mut self, ty: &str) -> usize {
        for (i, s) in self.types.iter().enumerate() {
            match s {
                Type::Builtin { id, .. } if id == ty => {
                    return i + 1;
                },
                _ => ()
            }
        }

        let length = self.types.len();

        self.types.push(Type::Builtin {
            id: ty.to_owned(),
            def: "builtin".to_owned(),
        });

        length + 1
    }

    /// Adds struct type to registry. Does not check for duplication (yet)
    fn struct_type(&mut self, name: &str, fields: Vec<StructField>) -> usize {
        let length = self.types.len();
        let name = self.string(name);

        self.types.push(Type::Struct {
            id: CustomID {
                name,
                namespace: Vec::new(),
                params: Vec::new(),
            },
            def: StructDef {
                fields
            }
        });

        length + 1
    }
}

pub fn gen_abi(resolver_contract: &resolver::Contract) -> Metadata {
    let mut registry = Registry::new();

    let fields = resolver_contract.variables.iter()
        .filter(|v| !v.storage.is_none())
        .map(|v| {
            let (scalety, _) = solty_to_scalety(&v.ty, resolver_contract);

            StructField {
                name: registry.string(&v.name),
                ty: registry.builtin_type(&scalety)
            }
        }).collect();

    let storagety = registry.struct_type("storage", fields);

    let fields = resolver_contract.variables.iter()
        .filter(|v| !v.storage.is_none())
        .map(|v| {
            let storage = v.storage.unwrap();
            let (scalety, len) = solty_to_scalety(&v.ty, resolver_contract);

            StorageLayout {
                name: registry.string(&v.name),
                layout: StorageFieldLayout::Field(LayoutField{
                    offset: format!("0x{:064X}", storage),
                    len,
                    ty: registry.builtin_type(&scalety)
                })
            }
        }).collect();

    let storage = Storage {
        ty: storagety,
        fields: vec!(StorageLayout {
            name: registry.string("Storage"),
            layout: StorageFieldLayout::Storage(Box::new(
                Storage {
                    ty: storagety,
                    fields
                }
            ))
        })
    };

    let constructors = resolver_contract.constructors.iter().map(|f| Constructor{
        name: registry.string("new"),
        selector: f.selector(),
        args: f.params.iter().map(|p| parameter_to_abi(p, resolver_contract, &mut registry)).collect(),
    }).collect();

    let messages = resolver_contract.functions.iter().map(|f| Message{
        name: registry.string(&f.name),
        mutates: f.mutability.is_none(),
        return_type: match f.returns.len() {
            0 => None,
            1 => Some(ty_to_abi(&f.returns[0].ty, resolver_contract, &mut registry)),
            _ => unreachable!()
        },
        selector: f.selector(),
        args: f.params.iter().map(|p| parameter_to_abi(p, resolver_contract, &mut registry)).collect(),
    }).collect();

    let contract = Contract{
        name: registry.string(&resolver_contract.name),
        constructors: constructors,
        messages: messages,
    };

    Metadata{registry, storage, contract}
}

fn solty_to_scalety(ty: &resolver::TypeName, contract: &resolver::Contract) -> (String, usize) {
    let solty = match &ty {
        resolver::TypeName::Elementary(e) => e,
        resolver::TypeName::Enum(ref i) => &contract.enums[*i].ty,
        resolver::TypeName::Noreturn => unreachable!(),
    };

    match solty {
        ast::ElementaryTypeName::Bool => ("bool".into(), 1),
        ast::ElementaryTypeName::Uint(n) => (format!("u{}", n), (n / 8).into()),
        ast::ElementaryTypeName::Int(n) => (format!("i{}", n), (n / 8).into()),
        _ => unreachable!()
    }
}

fn ty_to_abi(ty: &resolver::TypeName, contract: &resolver::Contract, registry: &mut Registry) -> ParamType {
    let solty = match &ty {
        resolver::TypeName::Elementary(e) => e,
        resolver::TypeName::Enum(ref i) => &contract.enums[*i].ty,
        resolver::TypeName::Noreturn => unreachable!(),
    };

    let scalety = match solty {
        ast::ElementaryTypeName::Bool => "bool".into(),
        ast::ElementaryTypeName::Uint(n) => format!("u{}", n),
        ast::ElementaryTypeName::Int(n) => format!("i{}", n),
        _ => unreachable!()
    };

    ParamType{
        ty: registry.builtin_type(&scalety),
        display_name: vec![ registry.string(&solty.to_string()) ],
    }
}

fn parameter_to_abi(param: &resolver::Parameter, contract: &resolver::Contract, registry: &mut Registry) -> Param {
    Param {
        name: registry.string(&param.name),
        ty: ty_to_abi(&param.ty, contract, registry)
    }
}
