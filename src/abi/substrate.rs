// Parity Substrate style ABIs/metadata

use parser::ast;
use resolver;
use serde::{Deserialize, Serialize};

/// Substrate contracts abi consists of a a registry of strings and types, the contract itself
#[derive(Deserialize, Serialize)]
pub struct Metadata {
    pub registry: Registry,
    storage: Storage,
    pub contract: Contract,
}

impl Metadata {
    pub fn get_function(&self, name: &str) -> Option<&Message> {
        self.contract
            .messages
            .iter()
            .find(|m| name == self.registry.get_str(m.name))
    }
}

/// The registry holds strings and types. Presumably this is to avoid duplication
#[derive(Deserialize, Serialize)]
pub struct Registry {
    strings: Vec<String>,
    types: Vec<Type>,
}

#[derive(Deserialize, Serialize)]
pub struct Array {
    #[serde(rename = "array.len")]
    len: usize,
    #[serde(rename = "array.type")]
    ty: usize,
}

#[derive(Deserialize, Serialize)]
#[serde(untagged)]
enum Type {
    Builtin { id: String, def: String },
    BuiltinArray { id: Array, def: String },
    Struct { id: CustomID, def: StructDef },
}

#[derive(Deserialize, Serialize)]
pub struct Contract {
    pub name: usize,
    pub constructors: Vec<Constructor>,
    pub messages: Vec<Message>,
}

#[derive(Deserialize, Serialize)]
struct BuiltinType {
    id: String,
    def: String,
}

#[derive(Deserialize, Serialize)]
struct StructType {
    id: CustomID,
    def: StructDef,
}

#[derive(Deserialize, Serialize)]
struct CustomID {
    #[serde(rename = "custom.name")]
    name: usize,
    #[serde(rename = "custom.namespace")]
    namespace: Vec<usize>,
    #[serde(rename = "custom.params")]
    params: Vec<usize>,
}

#[derive(Deserialize, Serialize)]
struct StructDef {
    #[serde(rename = "struct.fields")]
    fields: Vec<StructField>,
}

#[derive(Deserialize, Serialize)]
struct StructField {
    name: usize,
    #[serde(rename = "type")]
    ty: usize,
}

#[derive(Deserialize, Serialize)]
pub struct Constructor {
    pub name: usize,
    pub selector: u32,
    pub docs: Vec<String>,
    args: Vec<Param>,
}

#[derive(Deserialize, Serialize)]
pub struct Message {
    pub name: usize,
    pub selector: u32,
    pub docs: Vec<String>,
    mutates: bool,
    args: Vec<Param>,
    return_type: Option<ParamType>,
}

#[derive(Deserialize, Serialize)]
struct Param {
    name: usize,
    #[serde(rename = "type")]
    ty: ParamType,
}

#[derive(Deserialize, Serialize)]
struct ParamType {
    ty: usize,
    display_name: Vec<usize>,
}

#[derive(Deserialize, Serialize)]
struct Storage {
    #[serde(rename = "struct.type")]
    ty: usize,
    #[serde(rename = "struct.fields")]
    fields: Vec<StorageLayout>,
}

#[derive(Deserialize, Serialize)]
struct LayoutField {
    #[serde(rename = "range.offset")]
    offset: String,
    #[serde(rename = "range.len")]
    len: usize,
    #[serde(rename = "range.elem_type")]
    ty: usize,
}

#[derive(Deserialize, Serialize)]
struct StorageLayout {
    name: usize,
    layout: StorageFieldLayout,
}

#[derive(Deserialize, Serialize)]
#[serde(untagged)]
enum StorageFieldLayout {
    Field(LayoutField),
    Storage(Box<Storage>),
}

/// Create a new registry and create new entries. Note that the registry is
/// accessed by number, and the first entry is 1, not 0.
impl Registry {
    fn new() -> Self {
        Registry {
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

    /// Returns the string at the specified index
    pub fn get_str(&self, index: usize) -> &str {
        &self.strings[index - 1]
    }

    /// Returns index to builtin type in registry. Type is added if not already present
    fn builtin_type(&mut self, ty: &str) -> usize {
        for (i, s) in self.types.iter().enumerate() {
            match s {
                Type::Builtin { id, .. } if id == ty => {
                    return i + 1;
                }
                _ => (),
            }
        }

        let length = self.types.len();

        self.types.push(Type::Builtin {
            id: ty.to_owned(),
            def: "builtin".to_owned(),
        });

        length + 1
    }

    /// Returns index to builtin type in registry. Type is added if not already present
    fn builtin_bytes_type(&mut self, array_len: usize) -> usize {
        let elem = self.builtin_type("u8");

        for (i, s) in self.types.iter().enumerate() {
            match s {
                Type::BuiltinArray {
                    id: Array { len, ty },
                    ..
                } if *len == array_len && *ty == elem => {
                    return i + 1;
                }
                _ => (),
            }
        }

        let length = self.types.len();

        self.types.push(Type::BuiltinArray {
            id: Array {
                len: array_len,
                ty: elem,
            },
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
            def: StructDef { fields },
        });

        length + 1
    }
}

pub fn load(bs: &str) -> Result<Metadata, serde_json::error::Error> {
    serde_json::from_str(bs)
}

pub fn gen_abi(resolver_contract: &resolver::Contract) -> Metadata {
    let mut registry = Registry::new();

    let fields = resolver_contract
        .variables
        .iter()
        .filter(|v| !v.is_storage())
        .map(|v| {
            let (scalety, _) = solty_to_scalety(&v.ty, resolver_contract);

            StructField {
                name: registry.string(&v.name),
                ty: registry.builtin_type(&scalety),
            }
        })
        .collect();

    let storagety = registry.struct_type("storage", fields);

    let fields = resolver_contract
        .variables
        .iter()
        .filter_map(|v| {
            if let resolver::ContractVariableType::Storage(storage) = v.var {
                let (scalety, len) = solty_to_scalety(&v.ty, resolver_contract);

                Some(StorageLayout {
                    name: registry.string(&v.name),
                    layout: StorageFieldLayout::Field(LayoutField {
                        offset: format!("0x{:064X}", storage),
                        len,
                        ty: registry.builtin_type(&scalety),
                    }),
                })
            } else {
                None
            }
        })
        .collect();

    let storage = Storage {
        ty: storagety,
        fields: vec![StorageLayout {
            name: registry.string("Storage"),
            layout: StorageFieldLayout::Storage(Box::new(Storage {
                ty: storagety,
                fields,
            })),
        }],
    };

    let constructors = resolver_contract
        .constructors
        .iter()
        .map(|f| Constructor {
            name: registry.string("new"),
            selector: f.selector(),
            args: f
                .params
                .iter()
                .map(|p| parameter_to_abi(p, resolver_contract, &mut registry))
                .collect(),
            docs: f.doc.clone(),
        })
        .collect();

    let messages = resolver_contract
        .functions
        .iter()
        .filter(|f| {
            if let ast::Visibility::Public(_) = f.visibility {
                true
            } else {
                false
            }
        })
        .map(|f| Message {
            name: registry.string(&f.name),
            mutates: f.mutability.is_none(),
            return_type: match f.returns.len() {
                0 => None,
                1 => Some(ty_to_abi(
                    &f.returns[0].ty,
                    resolver_contract,
                    &mut registry,
                )),
                _ => unreachable!(),
            },
            selector: f.selector(),
            args: f
                .params
                .iter()
                .map(|p| parameter_to_abi(p, resolver_contract, &mut registry))
                .collect(),
            docs: f.doc.clone(),
        })
        .collect();

    let contract = Contract {
        name: registry.string(&resolver_contract.name),
        constructors,
        messages,
    };

    Metadata {
        registry,
        storage,
        contract,
    }
}

fn solty_to_scalety(ty: &resolver::Type, contract: &resolver::Contract) -> (String, usize) {
    match ty_to_primitive(ty, contract) {
        ast::PrimitiveType::Bool => ("bool".into(), 1),
        ast::PrimitiveType::Uint(n) => (format!("u{}", n), (n / 8).into()),
        ast::PrimitiveType::Int(n) => (format!("i{}", n), (n / 8).into()),
        ast::PrimitiveType::Bytes(n) => (format!("bytes{}", n), *n as usize),
        ast::PrimitiveType::Address => ("address".into(), 20),
        _ => unreachable!(),
    }
}

fn ty_to_abi(
    ty: &resolver::Type,
    contract: &resolver::Contract,
    registry: &mut Registry,
) -> ParamType {
    let primitive = ty_to_primitive(ty, contract);

    match primitive {
        ast::PrimitiveType::Bytes(n) => ParamType {
            ty: registry.builtin_bytes_type(*n as usize),
            display_name: vec![],
        },
        ast::PrimitiveType::Address => ParamType {
            ty: registry.builtin_bytes_type(160 as usize),
            display_name: vec![],
        },
        _ => {
            let scalety = primitive_to_string(*primitive);

            ParamType {
                ty: registry.builtin_type(&scalety),
                display_name: vec![registry.string(&scalety.to_string())],
            }
        }
    }
}

// For a given resolved type, return the underlying primitive
fn ty_to_primitive<'a>(
    ty: &'a resolver::Type,
    resolved_contract: &'a resolver::Contract,
) -> &'a ast::PrimitiveType {
    match ty {
        resolver::Type::Primitive(e) => e,
        resolver::Type::Enum(ref i) => &resolved_contract.enums[*i].ty,
        resolver::Type::FixedArray(_, _) => unimplemented!(),
        resolver::Type::Undef => unreachable!(),
        resolver::Type::Ref(ty) => ty_to_primitive(ty, resolved_contract),
    }
}

// For a given primitive, give the name as Substrate would like it (i.e. 64 bits
// signed int is i64, not int64).
fn primitive_to_string(ty: ast::PrimitiveType) -> String {
    match ty {
        ast::PrimitiveType::Bool => "bool".into(),
        ast::PrimitiveType::Uint(n) => format!("u{}", n),
        ast::PrimitiveType::Int(n) => format!("i{}", n),
        ast::PrimitiveType::Bytes(n) => format!("bytes{}", n),
        ast::PrimitiveType::Address => "address".into(),
        _ => unreachable!(),
    }
}

fn parameter_to_abi(
    param: &resolver::Parameter,
    contract: &resolver::Contract,
    registry: &mut Registry,
) -> Param {
    Param {
        name: registry.string(&param.name),
        ty: ty_to_abi(&param.ty, contract, registry),
    }
}
