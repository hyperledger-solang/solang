use crate::ast::{Namespace, Parameter, StructDecl, StructType, Type};
use once_cell::sync::Lazy;
use solang_parser::pt;

static BUILTIN_STRUCTS: Lazy<[StructDecl; 3]> = Lazy::new(|| {
    [
        StructDecl {
            tags: Vec::new(),
            loc: pt::Loc::Builtin,
            builtin: StructType::AccountInfo, // TODO: Remove this
            contract: None,
            name: "AccountInfo".to_string(),
            fields: vec![
                Parameter {
                    loc: pt::Loc::Builtin,
                    id: Some(pt::Identifier {
                        name: String::from("key"),
                        loc: pt::Loc::Builtin,
                    }),
                    ty: Type::Address(false),
                    ty_loc: None,
                    indexed: false,
                    readonly: true,
                    recursive: false,
                },
                Parameter {
                    loc: pt::Loc::Builtin,
                    id: Some(pt::Identifier {
                        name: String::from("lamports"),
                        loc: pt::Loc::Builtin,
                    }),
                    ty: Type::Uint(64),
                    ty_loc: None,
                    indexed: false,
                    readonly: false,
                    recursive: false,
                },
                Parameter {
                    loc: pt::Loc::Builtin,
                    id: Some(pt::Identifier {
                        name: String::from("data"),
                        loc: pt::Loc::Builtin,
                    }),
                    ty: Type::DynamicBytes,
                    ty_loc: None,
                    indexed: false,
                    readonly: true,
                    recursive: false,
                },
                Parameter {
                    loc: pt::Loc::Builtin,
                    id: Some(pt::Identifier {
                        name: String::from("owner"),
                        loc: pt::Loc::Builtin,
                    }),
                    ty: Type::Address(false),
                    ty_loc: None,
                    indexed: false,
                    readonly: true,
                    recursive: false,
                },
                Parameter {
                    loc: pt::Loc::Builtin,
                    id: Some(pt::Identifier {
                        name: String::from("rent_epoch"),
                        loc: pt::Loc::Builtin,
                    }),
                    ty: Type::Uint(64),
                    ty_loc: None,
                    indexed: false,
                    readonly: true,
                    recursive: false,
                },
                Parameter {
                    loc: pt::Loc::Builtin,
                    id: Some(pt::Identifier {
                        name: String::from("is_signer"),
                        loc: pt::Loc::Builtin,
                    }),
                    ty: Type::Bool,
                    ty_loc: None,
                    indexed: false,
                    readonly: true,
                    recursive: false,
                },
                Parameter {
                    loc: pt::Loc::Builtin,
                    id: Some(pt::Identifier {
                        name: String::from("is_writable"),
                        loc: pt::Loc::Builtin,
                    }),
                    ty: Type::Bool,
                    ty_loc: None,
                    indexed: false,
                    readonly: true,
                    recursive: false,
                },
                Parameter {
                    loc: pt::Loc::Builtin,
                    id: Some(pt::Identifier {
                        name: String::from("executable"),
                        loc: pt::Loc::Builtin,
                    }),
                    ty: Type::Bool,
                    ty_loc: None,
                    indexed: false,
                    readonly: true,
                    recursive: false,
                },
            ],
            offsets: Vec::new(),
            storage_offsets: Vec::new(),
        },
        StructDecl {
            tags: Vec::new(),
            loc: pt::Loc::Builtin,
            builtin: StructType::AccountMeta,
            contract: None,
            name: "AccountMeta".to_string(),
            fields: vec![
                Parameter {
                    loc: pt::Loc::Builtin,
                    id: Some(pt::Identifier {
                        name: String::from("pubkey"),
                        loc: pt::Loc::Builtin,
                    }),
                    ty: Type::Ref(Box::new(Type::Address(false))),
                    ty_loc: None,
                    indexed: false,
                    readonly: false,
                    recursive: false,
                },
                Parameter {
                    loc: pt::Loc::Builtin,
                    id: Some(pt::Identifier {
                        name: String::from("is_writable"),
                        loc: pt::Loc::Builtin,
                    }),
                    ty: Type::Bool,
                    ty_loc: None,
                    indexed: false,
                    readonly: false,
                    recursive: false,
                },
                Parameter {
                    loc: pt::Loc::Builtin,
                    id: Some(pt::Identifier {
                        name: String::from("is_signer"),
                        loc: pt::Loc::Builtin,
                    }),
                    ty: Type::Bool,
                    ty_loc: None,
                    indexed: false,
                    readonly: false,
                    recursive: false,
                },
            ],
            offsets: Vec::new(),
            storage_offsets: Vec::new(),
        },
        StructDecl {
            tags: Vec::new(),
            name: "ExternalFunction".to_string(),
            loc: pt::Loc::Builtin,
            builtin: StructType::ExternalFunction,
            contract: None,
            fields: vec![
                Parameter {
                    loc: pt::Loc::Builtin,
                    id: None,
                    ty: Type::Address(false),
                    ty_loc: None,
                    indexed: false,
                    readonly: false,
                    recursive: false,
                },
                Parameter {
                    loc: pt::Loc::Builtin,
                    id: None,
                    ty: Type::Bytes(4),
                    ty_loc: None,
                    indexed: false,
                    readonly: false,
                    recursive: false,
                },
            ],
            offsets: Vec::new(),
            storage_offsets: Vec::new(),
        },
    ]
});

impl StructType {
    pub fn get_definition<'a>(&'a self, ns: &'a Namespace) -> &StructDecl {
        match self {
            StructType::UserDefined(struct_no) => &ns.structs[*struct_no],
            StructType::AccountInfo => &BUILTIN_STRUCTS[0],
            StructType::AccountMeta => &BUILTIN_STRUCTS[1],
            StructType::ExternalFunction => &BUILTIN_STRUCTS[2],
            StructType::None => &BUILTIN_STRUCTS[2],
        }
    }
}
