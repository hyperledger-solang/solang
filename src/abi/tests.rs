// SPDX-License-Identifier: Apache-2.0

#![cfg(test)]

use crate::abi::anchor::generate_anchor_idl;
use crate::codegen::{codegen, Options};
use crate::file_resolver::FileResolver;
use crate::sema::ast::Namespace;
use crate::{codegen, parse_and_resolve, Target};
use anchor_syn::idl::types::{
    IdlAccount, IdlAccountItem, IdlEnumVariant, IdlEvent, IdlEventField, IdlField, IdlType,
    IdlTypeDefinition, IdlTypeDefinitionTy,
};
use semver::Version;
use serde_json::json;
use std::ffi::OsStr;

fn generate_namespace(src: &'static str) -> Namespace {
    let mut cache = FileResolver::default();
    cache.set_file_contents("test.sol", src.to_string());
    parse_and_resolve(OsStr::new("test.sol"), &mut cache, Target::Solana)
}

#[test]
fn version_name_and_docs() {
    let src = r#"
/// @title MyContract
/// @author Lucas
contract caller {
    function doThis(int64 a) public pure returns (int64) {
        return a + 2;
    }

    function doThat(int32 b) public pure returns (int32) {
        return b + 3;
    }

    function do_call() pure public returns (int64, int32) {
        return (this.doThis(5), this.doThat(3));
    }
}
    "#;

    let mut ns = generate_namespace(src);
    codegen(&mut ns, &Options::default());
    let idl = generate_anchor_idl(0, &ns, "0.1.0");
    assert_eq!(idl.version, Version::parse("0.1.0").unwrap().to_string());
    assert_eq!(idl.name, "caller");
    assert!(idl.docs.is_some());
    assert_eq!(idl.docs.as_ref().unwrap().len(), 2);
    assert_eq!(idl.docs.as_ref().unwrap()[0], "title: MyContract");
    assert_eq!(idl.docs.as_ref().unwrap()[1], "author: Lucas");
}

#[test]
fn constants_and_types() {
    let src = r#"
    contract caller {
    int32 public constant cte1 = -90;
    uint64[3] public constant cte2 = [90, 875, 1044];
    string public constant cte3 = "Rio";
    string[4] public constant cte4 = ["Baku", "Paris", "Sao Paulo", "Auckland"];
    MyStruct public constant cte5 = MyStruct(125, "ab");
    Week public constant cte6 = Week.Tuesday;

    struct MyStruct {
        uint8 g;
        bytes2 d;
    }

    enum Week {Monday, Tuesday, Wednesday}
}
    "#;
    let mut ns = generate_namespace(src);
    codegen(&mut ns, &Options::default());
    let idl = generate_anchor_idl(0, &ns, "0.1.0");

    assert!(idl.constants.is_empty());

    assert_eq!(idl.types.len(), 2);

    assert_eq!(idl.types[0].name, "Week");
    assert!(idl.types[0].docs.is_none());
    assert_eq!(
        idl.types[0].ty,
        IdlTypeDefinitionTy::Enum {
            variants: vec![
                IdlEnumVariant {
                    name: "Monday".to_string(),
                    fields: None,
                },
                IdlEnumVariant {
                    name: "Tuesday".to_string(),
                    fields: None,
                },
                IdlEnumVariant {
                    name: "Wednesday".to_string(),
                    fields: None,
                }
            ]
        }
    );

    assert_eq!(idl.types[1].name, "cte5_returns");
    assert_eq!(
        idl.types[1].docs,
        Some(vec![
            "Data structure to hold the multiple returns of function cte5".into()
        ])
    );
    assert_eq!(
        idl.types[1].ty,
        IdlTypeDefinitionTy::Struct {
            fields: vec![
                IdlField {
                    name: "g".to_string(),
                    docs: None,
                    ty: IdlType::U8,
                },
                IdlField {
                    name: "d".to_string(),
                    docs: None,
                    ty: IdlType::Array(IdlType::U8.into(), 2)
                }
            ]
        }
    );
}

#[test]
fn instructions_and_types() {
    let src = r#"
    contract caller {

    string private my_string;
    uint64 public cte;
    uint64[] public cte2;

    struct MetaData {
        bool b;
        bool c;
    }

    function sum(uint256 a, int256 b) public pure returns (int256) {
        MetaData d = MetaData(true, false);
        return notInIdl(a, d) + b;
    }

    /// @param c input
    function setString(string c) public {
        my_string = c;
    }

    /// @return the string
    function getString() public view returns (string) {
        return my_string;
    }

    function notInIdl(uint256 c, MetaData dd) private pure returns (int256) {
        if (dd.c && dd.b) {
            return 0;
        }
        return int256(c);
    }

    function multipleReturns() public returns (uint64, string) {
        cte += 1;
        return (cte, my_string);
    }

    modifier doSomething() {
        require(block.number >= 50);
        _;
    }

    fallback() external {
        setString("error2");
    }
}
    "#;

    let mut ns = generate_namespace(src);
    codegen(&mut ns, &Options::default());
    let idl = generate_anchor_idl(0, &ns, "0.1.0");

    assert_eq!(idl.instructions.len(), 7);

    // implicit constructor
    assert_eq!(idl.instructions[0].name, "new");
    assert!(idl.instructions[0].docs.is_none());
    assert_eq!(
        idl.instructions[0].accounts,
        vec![idl_account("dataAccount", true, false)]
    );
    assert!(idl.instructions[0].args.is_empty());
    assert!(idl.instructions[0].returns.is_none());

    // cte accessor function
    assert_eq!(idl.instructions[1].name, "cte");
    assert!(idl.instructions[1].docs.is_none());
    assert_eq!(
        idl.instructions[1].accounts,
        vec![idl_account("dataAccount", false, false)]
    );
    assert!(idl.instructions[1].args.is_empty());
    assert_eq!(idl.instructions[1].returns, Some(IdlType::U64));

    // cte2 accessor function
    assert_eq!(idl.instructions[2].name, "cte2");
    assert!(idl.instructions[2].docs.is_none());
    assert_eq!(
        idl.instructions[2].accounts,
        vec![idl_account("dataAccount", false, false)]
    );
    assert_eq!(
        idl.instructions[2].args,
        vec![IdlField {
            name: "arg0".to_string(),
            docs: None,
            ty: IdlType::U256,
        }]
    );
    assert_eq!(idl.instructions[2].returns, Some(IdlType::U64));

    // sum function
    assert_eq!(idl.instructions[3].name, "sum");
    assert!(idl.instructions[3].docs.is_none());
    assert_eq!(idl.instructions[3].accounts, vec![]);
    assert_eq!(
        idl.instructions[3].args,
        vec![
            IdlField {
                name: "a".to_string(),
                docs: None,
                ty: IdlType::U256
            },
            IdlField {
                name: "b".to_string(),
                docs: None,
                ty: IdlType::I256
            }
        ]
    );
    assert_eq!(idl.instructions[3].returns, Some(IdlType::I256));

    assert_eq!(idl.instructions[4].name, "setString");
    assert_eq!(
        idl.instructions[4].docs,
        Some(vec!["param: input".to_string()])
    );
    assert_eq!(
        idl.instructions[4].accounts,
        vec![idl_account("dataAccount", true, false)]
    );
    assert_eq!(
        idl.instructions[4].args,
        vec![IdlField {
            name: "c".to_string(),
            docs: None,
            ty: IdlType::String
        }]
    );
    assert!(idl.instructions[4].returns.is_none());

    assert_eq!(idl.instructions[5].name, "getString");
    assert_eq!(
        idl.instructions[5].docs,
        Some(vec!["return: the string".to_string()])
    );
    assert_eq!(
        idl.instructions[5].accounts,
        vec![idl_account("dataAccount", false, false)]
    );
    assert!(idl.instructions[5].args.is_empty());
    assert_eq!(idl.instructions[5].returns, Some(IdlType::String));

    assert_eq!(idl.instructions[6].name, "multipleReturns");
    assert!(idl.instructions[6].docs.is_none());
    assert_eq!(
        idl.instructions[6].accounts,
        vec![idl_account("dataAccount", true, false),]
    );
    assert!(idl.instructions[6].args.is_empty());
    assert_eq!(
        idl.instructions[6].returns,
        Some(IdlType::Defined("multipleReturns_returns".to_string()))
    );

    assert!(idl.accounts.is_empty());

    assert_eq!(idl.types.len(), 1);

    assert_eq!(
        idl.types[0],
        IdlTypeDefinition {
            name: "multipleReturns_returns".to_string(),
            docs: Some(vec![
                "Data structure to hold the multiple returns of function multipleReturns"
                    .to_string()
            ]),
            ty: IdlTypeDefinitionTy::Struct {
                fields: vec![
                    IdlField {
                        name: "return_0".to_string(),
                        docs: None,
                        ty: IdlType::U64
                    },
                    IdlField {
                        name: "return_1".to_string(),
                        docs: None,
                        ty: IdlType::String,
                    }
                ]
            },
            generics: None
        }
    );

    assert!(idl.events.is_none());
    assert!(idl.events.is_none());
    assert!(idl.metadata.is_none());
}

#[test]
fn events() {
    let src = r#"
contract caller {
    enum Color { Yellow, Blue, Green }

    event Event1(bool, string indexed, int8, Color);
    event Event2(bool a, uint128 indexed cc);

    function emitAll(bool a, string b, int8 d, uint128 e) public {
        emit Event1(a, b, d, Color.Blue);
        emit Event2(a, e);
    }
}
    "#;

    let mut ns = generate_namespace(src);
    // We need this to populate Contract.emit_events
    codegen::codegen(&mut ns, &Options::default());

    let idl = generate_anchor_idl(0, &ns, "0.1.0");

    assert_eq!(idl.instructions.len(), 2);

    // implicit constructor
    assert_eq!(idl.instructions[0].name, "new");
    assert!(idl.instructions[0].docs.is_none());
    assert_eq!(
        idl.instructions[0].accounts,
        vec![idl_account("dataAccount", true, false)]
    );
    assert!(idl.instructions[0].args.is_empty());
    assert!(idl.instructions[0].returns.is_none());

    assert_eq!(idl.instructions[1].name, "emitAll");
    assert!(idl.instructions[1].docs.is_none());
    assert!(idl.instructions[1].accounts.is_empty());
    assert_eq!(
        idl.instructions[1].args,
        vec![
            IdlField {
                name: "a".to_string(),
                docs: None,
                ty: IdlType::Bool,
            },
            IdlField {
                name: "b".to_string(),
                docs: None,
                ty: IdlType::String,
            },
            IdlField {
                name: "d".to_string(),
                docs: None,
                ty: IdlType::I8
            },
            IdlField {
                name: "e".to_string(),
                docs: None,
                ty: IdlType::U128,
            }
        ]
    );
    assert!(idl.instructions[1].returns.is_none());

    assert!(idl.accounts.is_empty());

    assert_eq!(idl.types.len(), 1);

    assert_eq!(
        idl.types[0],
        IdlTypeDefinition {
            name: "Color".to_string(),
            docs: None,
            ty: IdlTypeDefinitionTy::Enum {
                variants: vec![
                    IdlEnumVariant {
                        name: "Yellow".to_string(),
                        fields: None,
                    },
                    IdlEnumVariant {
                        name: "Blue".to_string(),
                        fields: None,
                    },
                    IdlEnumVariant {
                        name: "Green".to_string(),
                        fields: None,
                    }
                ]
            },
            generics: None
        }
    );

    assert_eq!(
        idl.events,
        Some(vec![
            IdlEvent {
                name: "Event1".to_string(),
                fields: vec![
                    IdlEventField {
                        name: "field_0".to_string(),
                        ty: IdlType::Bool,
                        index: false,
                    },
                    IdlEventField {
                        name: "field_1".to_string(),
                        ty: IdlType::String,
                        index: true,
                    },
                    IdlEventField {
                        name: "field_2".to_string(),
                        ty: IdlType::I8,
                        index: false,
                    },
                    IdlEventField {
                        name: "field_3".to_string(),
                        ty: IdlType::Defined("Color".to_string()),
                        index: false,
                    }
                ],
            },
            IdlEvent {
                name: "Event2".to_string(),
                fields: vec![
                    IdlEventField {
                        name: "a".to_string(),
                        ty: IdlType::Bool,
                        index: false,
                    },
                    IdlEventField {
                        name: "cc".to_string(),
                        ty: IdlType::U128,
                        index: true,
                    }
                ]
            }
        ])
    );

    assert!(idl.errors.is_none());
    assert!(idl.metadata.is_none());
}

#[test]
fn types() {
    let src = r#"
    contract caller {
    event Event1(int24, uint32);

    function myFunc(int24 a, uint32[2][] b, uint32[2][4] d, uint32[][2] e) public {
        emit Event1(a, b[0][1] + d[1][2] - e[1][0]);
    }
}
    "#;

    let mut ns = generate_namespace(src);
    codegen(&mut ns, &Options::default());
    let idl = generate_anchor_idl(0, &ns, "0.1.0");

    assert_eq!(idl.instructions.len(), 2);

    // implicit constructor
    assert_eq!(idl.instructions[0].name, "new");
    assert!(idl.instructions[0].docs.is_none());
    assert_eq!(
        idl.instructions[0].accounts,
        vec![idl_account("dataAccount", true, false)]
    );
    assert!(idl.instructions[0].args.is_empty());
    assert!(idl.instructions[0].returns.is_none());

    assert_eq!(idl.instructions[1].name, "myFunc");
    assert!(idl.instructions[1].accounts.is_empty());
    assert_eq!(
        idl.instructions[1].args,
        vec![
            IdlField {
                name: "a".to_string(),
                docs: None,
                ty: IdlType::I32,
            },
            IdlField {
                name: "b".to_string(),
                docs: None,
                ty: IdlType::Vec(IdlType::Array(IdlType::U32.into(), 2).into()),
            },
            IdlField {
                name: "d".to_string(),
                docs: None,
                ty: IdlType::Array(IdlType::Array(IdlType::U32.into(), 2).into(), 4),
            },
            IdlField {
                name: "e".to_string(),
                docs: None,
                ty: IdlType::Array(IdlType::Vec(IdlType::U32.into()).into(), 2)
            }
        ]
    );
    assert!(idl.instructions[1].returns.is_none());
    assert!(idl.accounts.is_empty());
    assert!(idl.types.is_empty());
    assert_eq!(
        idl.events.unwrap(),
        vec![IdlEvent {
            name: "Event1".to_string(),
            fields: vec![
                IdlEventField {
                    name: "field_0".to_string(),
                    ty: IdlType::I32,
                    index: false,
                },
                IdlEventField {
                    name: "field_1".to_string(),
                    ty: IdlType::U32,
                    index: false,
                }
            ]
        }]
    );
    assert!(idl.errors.is_none());
    assert!(idl.metadata.is_none());
}

#[test]
fn constructor() {
    let src = r#"
        contract caller {
    uint64 b;
    constructor(uint64 ff) {
        b = ff;
    }

    function getNum() public view returns (uint64) {
        return b;
    }
}
    "#;
    let mut ns = generate_namespace(src);
    codegen(&mut ns, &Options::default());
    let idl = generate_anchor_idl(0, &ns, "0.1.0");

    assert_eq!(idl.name, "caller");
    assert!(idl.docs.is_none());
    assert!(idl.constants.is_empty());

    assert_eq!(idl.instructions.len(), 2);
    assert_eq!(idl.instructions[0].name, "new");
    assert!(idl.instructions[0].docs.is_none());
    assert_eq!(
        idl.instructions[0].accounts,
        vec![idl_account("dataAccount", true, false)]
    );
    assert_eq!(
        idl.instructions[0].args,
        vec![IdlField {
            name: "ff".to_string(),
            docs: None,
            ty: IdlType::U64,
        },]
    );
    assert!(idl.instructions[0].returns.is_none());

    assert_eq!(idl.instructions[1].name, "getNum");
    assert!(idl.instructions[1].docs.is_none());
    assert_eq!(
        idl.instructions[1].accounts,
        vec![idl_account("dataAccount", false, false)]
    );
    assert!(idl.instructions[1].args.is_empty());
    assert_eq!(idl.instructions[1].returns, Some(IdlType::U64));

    assert!(idl.accounts.is_empty());
    assert!(idl.types.is_empty());
    assert!(idl.events.is_none());
    assert!(idl.errors.is_none());
    assert!(idl.metadata.is_none());
}

#[test]
fn named_returns() {
    let src = r#"
contract Testing {
    function getNum(uint64 a, uint64 b) public pure returns (uint64 ret1, uint64 ret2) {
        ret1 = a + b;
        ret2 = a/b;
    }
}    "#;

    let mut ns = generate_namespace(src);
    codegen(&mut ns, &Options::default());
    let idl = generate_anchor_idl(0, &ns, "0.1.0");

    assert_eq!(idl.instructions.len(), 2);

    assert_eq!(idl.instructions[0].name, "new");
    assert!(idl.instructions[0].returns.is_none());
    assert!(idl.instructions[0].args.is_empty());
    assert_eq!(
        idl.instructions[0].accounts,
        vec![idl_account("dataAccount", true, false)]
    );

    assert_eq!(idl.instructions[1].name, "getNum");
    assert_eq!(
        idl.instructions[1].returns,
        Some(IdlType::Defined("getNum_returns".to_string()))
    );
    assert_eq!(
        idl.instructions[1].args,
        vec![
            IdlField {
                name: "a".to_string(),
                docs: None,
                ty: IdlType::U64
            },
            IdlField {
                name: "b".to_string(),
                docs: None,
                ty: IdlType::U64
            },
        ]
    );

    assert_eq!(idl.types.len(), 1);
    assert_eq!(
        idl.types[0],
        IdlTypeDefinition {
            name: "getNum_returns".to_string(),
            docs: Some(vec![
                "Data structure to hold the multiple returns of function getNum".to_string()
            ]),
            ty: IdlTypeDefinitionTy::Struct {
                fields: vec![
                    IdlField {
                        name: "ret1".to_string(),
                        docs: None,
                        ty: IdlType::U64,
                    },
                    IdlField {
                        name: "ret2".to_string(),
                        docs: None,
                        ty: IdlType::U64,
                    }
                ],
            },
            generics: None
        }
    );
}

#[test]
fn mangled_names() {
    let src = r#"
contract Testing {
    function getNum(uint64 a, uint64 b) public pure returns (uint64 ret1, uint64 ret2) {
        ret1 = a + b;
        ret2 = a/b;
    }

    function getNum(int32 a, int32 b) public pure returns (int32 ret3, int32 ret4) {
        ret3 = a-b;
        ret4 = b/a;
    }
}
    "#;

    let mut ns = generate_namespace(src);
    codegen(&mut ns, &Options::default());
    let idl = generate_anchor_idl(0, &ns, "0.1.0");

    assert_eq!(idl.instructions.len(), 3);

    assert_eq!(idl.instructions[0].name, "new");

    assert_eq!(idl.instructions[1].name, "getNum_uint64_uint64");
    assert!(idl.instructions[1].docs.is_none());
    assert_eq!(idl.instructions[1].accounts, vec![]);
    assert_eq!(idl.instructions[1].args.len(), 2);
    assert_eq!(
        idl.instructions[1].args[0],
        IdlField {
            name: "a".to_string(),
            docs: None,
            ty: IdlType::U64
        }
    );
    assert_eq!(
        idl.instructions[1].args[1],
        IdlField {
            name: "b".to_string(),
            docs: None,
            ty: IdlType::U64
        }
    );
    assert_eq!(
        idl.instructions[1].returns,
        Some(IdlType::Defined("getNum_uint64_uint64_returns".to_string()))
    );

    assert_eq!(idl.instructions[2].name, "getNum_int32_int32");
    assert!(idl.instructions[2].docs.is_none());
    assert_eq!(idl.instructions[1].accounts, vec![]);

    assert_eq!(idl.instructions[2].args.len(), 2);
    assert_eq!(
        idl.instructions[2].args[0],
        IdlField {
            name: "a".to_string(),
            docs: None,
            ty: IdlType::I32,
        }
    );
    assert_eq!(
        idl.instructions[2].args[1],
        IdlField {
            name: "b".to_string(),
            docs: None,
            ty: IdlType::I32
        }
    );
    assert_eq!(
        idl.instructions[2].returns,
        Some(IdlType::Defined("getNum_int32_int32_returns".to_string()))
    );

    assert_eq!(idl.types.len(), 2);

    assert_eq!(
        idl.types[0],
        IdlTypeDefinition {
            name: "getNum_uint64_uint64_returns".to_string(),
            docs: Some(vec![
                "Data structure to hold the multiple returns of function getNum".to_string()
            ]),
            ty: IdlTypeDefinitionTy::Struct {
                fields: vec![
                    IdlField {
                        name: "ret1".to_string(),
                        docs: None,
                        ty: IdlType::U64
                    },
                    IdlField {
                        name: "ret2".to_string(),
                        docs: None,
                        ty: IdlType::U64
                    }
                ]
            },
            generics: None
        }
    );

    assert_eq!(
        idl.types[1],
        IdlTypeDefinition {
            name: "getNum_int32_int32_returns".to_string(),
            docs: Some(vec![
                "Data structure to hold the multiple returns of function getNum".to_string()
            ]),
            ty: IdlTypeDefinitionTy::Struct {
                fields: vec![
                    IdlField {
                        name: "ret3".to_string(),
                        docs: None,
                        ty: IdlType::I32
                    },
                    IdlField {
                        name: "ret4".to_string(),
                        docs: None,
                        ty: IdlType::I32
                    }
                ]
            },
            generics: None
        }
    );
}

#[test]
fn name_collision() {
    let str = r#"
    contract Testing {
    struct getNum_returns {
        string str;
    }

    function getNum(uint64 a, uint64 b, getNum_returns c) public pure returns (uint64 ret1, uint64 ret2) {
        ret1 = a + b;
        ret2 = a/b;
    }

    function doNotGetNum(int32 a, int32 b) public pure returns (int32 ret3, int32 ret4) {
        ret3 = a-b;
        ret4 = b/a;
    }
}
    "#;

    let mut ns = generate_namespace(str);
    codegen(&mut ns, &Options::default());
    let idl = generate_anchor_idl(0, &ns, "0.1.0");

    assert_eq!(idl.types.len(), 3);

    assert_eq!(
        idl.types[0],
        IdlTypeDefinition {
            name: "getNum_returns".to_string(),
            docs: None,
            ty: IdlTypeDefinitionTy::Struct {
                fields: vec![IdlField {
                    name: "str".to_string(),
                    docs: None,
                    ty: IdlType::String,
                }]
            },
            generics: None
        }
    );

    assert_eq!(
        idl.types[1],
        IdlTypeDefinition {
            name: "getNum_returns_1".to_string(),
            docs: Some(vec![
                "Data structure to hold the multiple returns of function getNum".to_string()
            ]),
            ty: IdlTypeDefinitionTy::Struct {
                fields: vec![
                    IdlField {
                        name: "ret1".to_string(),
                        docs: None,
                        ty: IdlType::U64
                    },
                    IdlField {
                        name: "ret2".to_string(),
                        docs: None,
                        ty: IdlType::U64
                    }
                ]
            },
            generics: None
        }
    );

    assert_eq!(
        idl.types[2],
        IdlTypeDefinition {
            name: "doNotGetNum_returns".to_string(),
            docs: Some(vec![
                "Data structure to hold the multiple returns of function doNotGetNum".to_string()
            ]),
            ty: IdlTypeDefinitionTy::Struct {
                fields: vec![
                    IdlField {
                        name: "ret3".to_string(),
                        docs: None,
                        ty: IdlType::I32
                    },
                    IdlField {
                        name: "ret4".to_string(),
                        docs: None,
                        ty: IdlType::I32
                    }
                ]
            },
            generics: None
        }
    );
}

#[test]
fn double_name_collision() {
    let str = r#"
    contract Testing {
    struct getNum_returns {
        string str;
    }

    struct getNum_returns_1 {
        bytes bt;
    }

    function getNum(uint64 a, uint64 b, getNum_returns c) public pure returns (uint64 ret1, uint64 ret2) {
        ret1 = a + b;
        ret2 = a/b;
    }

    function doNotGetNum(int32 a, int32 b, getNum_returns_1 c) public pure returns (int32 ret3, int32 ret4) {
        ret3 = a-b;
        ret4 = b/a;
    }
}
    "#;

    let mut ns = generate_namespace(str);
    codegen(&mut ns, &Options::default());
    let idl = generate_anchor_idl(0, &ns, "0.1.0");

    assert_eq!(idl.types.len(), 4);

    assert_eq!(
        idl.types[0],
        IdlTypeDefinition {
            name: "getNum_returns".to_string(),
            docs: None,
            ty: IdlTypeDefinitionTy::Struct {
                fields: vec![IdlField {
                    name: "str".to_string(),
                    docs: None,
                    ty: IdlType::String,
                }]
            },
            generics: None
        }
    );

    assert_eq!(
        idl.types[1],
        IdlTypeDefinition {
            name: "getNum_returns_1".to_string(),
            docs: None,
            ty: IdlTypeDefinitionTy::Struct {
                fields: vec![IdlField {
                    name: "bt".to_string(),
                    docs: None,
                    ty: IdlType::Bytes
                },]
            },
            generics: None
        }
    );

    assert_eq!(
        idl.types[2],
        IdlTypeDefinition {
            name: "getNum_returns_2".to_string(),
            docs: Some(vec![
                "Data structure to hold the multiple returns of function getNum".to_string()
            ]),
            ty: IdlTypeDefinitionTy::Struct {
                fields: vec![
                    IdlField {
                        name: "ret1".to_string(),
                        docs: None,
                        ty: IdlType::U64
                    },
                    IdlField {
                        name: "ret2".to_string(),
                        docs: None,
                        ty: IdlType::U64
                    }
                ]
            },
            generics: None
        }
    );

    assert_eq!(
        idl.types[3],
        IdlTypeDefinition {
            name: "doNotGetNum_returns".to_string(),
            docs: Some(vec![
                "Data structure to hold the multiple returns of function doNotGetNum".to_string()
            ]),
            ty: IdlTypeDefinitionTy::Struct {
                fields: vec![
                    IdlField {
                        name: "ret3".to_string(),
                        docs: None,
                        ty: IdlType::I32
                    },
                    IdlField {
                        name: "ret4".to_string(),
                        docs: None,
                        ty: IdlType::I32
                    }
                ]
            },
            generics: None
        }
    );
}

#[test]
fn deduplication() {
    let src = r#"
    contract a {
    event myEvent(uint32, uint32 field_0, uint32, int64 field_0_1, int64 field_1, uint128);

    function myFunc(address ff, string) public returns (address, address return_0) {
        emit myEvent(1, 2, 3, 4, 5, 6);

        return (address(this), ff);
    }
}
    "#;

    let mut ns = generate_namespace(src);
    // We need this to populate Contract.emit_events
    codegen::codegen(&mut ns, &Options::default());

    let idl = generate_anchor_idl(0, &ns, "0.1.0");

    assert_eq!(idl.instructions.len(), 2);
    assert_eq!(idl.instructions[0].name, "new");

    assert_eq!(idl.instructions[1].name, "myFunc");
    assert_eq!(
        idl.instructions[1].args,
        vec![
            IdlField {
                name: "ff".to_string(),
                docs: None,
                ty: IdlType::PublicKey,
            },
            IdlField {
                name: "arg0".to_string(),
                docs: None,
                ty: IdlType::String,
            }
        ]
    );

    assert_eq!(idl.types.len(), 1);
    assert_eq!(
        idl.types[0],
        IdlTypeDefinition {
            name: "myFunc_returns".to_string(),
            docs: Some(vec![
                "Data structure to hold the multiple returns of function myFunc".to_owned()
            ]),
            ty: IdlTypeDefinitionTy::Struct {
                fields: vec![
                    IdlField {
                        name: "return_0".to_string(),
                        docs: None,
                        ty: IdlType::PublicKey
                    },
                    IdlField {
                        name: "return_0_1".to_string(),
                        docs: None,
                        ty: IdlType::PublicKey
                    }
                ]
            },
            generics: None
        }
    );

    assert!(idl.events.is_some());
    assert_eq!(idl.events.as_ref().unwrap().len(), 1);
    assert_eq!(
        idl.events.as_ref().unwrap()[0],
        IdlEvent {
            name: "myEvent".to_string(),
            fields: vec![
                IdlEventField {
                    name: "field_0".to_string(),
                    ty: IdlType::U32,
                    index: false,
                },
                IdlEventField {
                    name: "field_0_1".to_string(),
                    ty: IdlType::U32,
                    index: false,
                },
                IdlEventField {
                    name: "field_1".to_string(),
                    ty: IdlType::U32,
                    index: false,
                },
                IdlEventField {
                    name: "field_0_1_1".to_string(),
                    ty: IdlType::I64,
                    index: false,
                },
                IdlEventField {
                    name: "field_1_1".to_string(),
                    ty: IdlType::I64,
                    index: false,
                },
                IdlEventField {
                    name: "field_2".to_string(),
                    ty: IdlType::U128,
                    index: false,
                }
            ]
        }
    );
}

#[test]
fn duplicate_named_custom_types() {
    // Other contract comes first
    let src = r#"
contract D {
	struct Foo { int64 f1; }
}
contract C {
	enum Foo { b1, b2, b3 }
        function f(D.Foo x, Foo y) public pure returns (int64) { return x.f1; }
}
    "#;

    let mut ns = generate_namespace(src);
    codegen(&mut ns, &Options::default());
    let idl = generate_anchor_idl(1, &ns, "0.1.0");

    assert_eq!(idl.types.len(), 2);
    assert_eq!(idl.types[0].name, "D_Foo");
    assert_eq!(idl.types[1].name, "Foo");

    // Current contract comes first
    let src = r#"
    contract D {
	struct Foo { int64 f1; }
}
contract C {
	enum Foo { b1, b2, b3 }
        function f(Foo y, D.Foo x) public pure returns (int64) { return x.f1; }
}
    "#;

    let mut ns = generate_namespace(src);
    codegen(&mut ns, &Options::default());
    let idl = generate_anchor_idl(1, &ns, "0.1.0");

    assert_eq!(idl.types.len(), 2);
    assert_eq!(idl.types[0].name, "Foo");
    assert_eq!(idl.types[1].name, "D_Foo");

    // Type outside a contract first
    let src = r#"
    contract D {
	struct Foo { int64 f1; }
}

enum Foo { b1, b2, b3 }

contract C {
        function f(Foo y, D.Foo x) public pure returns (int64) { return x.f1; }
}
    "#;

    let mut ns = generate_namespace(src);
    codegen(&mut ns, &Options::default());
    let idl = generate_anchor_idl(1, &ns, "0.1.0");
    assert_eq!(idl.types.len(), 2);
    assert_eq!(idl.types[0].name, "Foo");
    assert_eq!(idl.types[1].name, "D_Foo");

    // Type outside contract second
    let src = r#"
    contract D {
	struct Foo { int64 f1; }
}

enum Foo { b1, b2, b3 }

contract C {
        function f(D.Foo x, Foo y) public pure returns (int64) { return x.f1; }
}
    "#;

    let mut ns = generate_namespace(src);
    codegen(&mut ns, &Options::default());
    let idl = generate_anchor_idl(1, &ns, "0.1.0");

    assert_eq!(idl.types.len(), 2);
    assert_eq!(idl.types[0].name, "D_Foo");
    assert_eq!(idl.types[1].name, "Foo");

    // Name already exists before
    let src = r#"
    contract D {
	struct Foo { int64 f1; }
}

enum Foo { b1, b2, b3 }

contract C {
    struct D_Foo {
        int64 f2;
    }
        function f(Foo y, D_Foo z, D.Foo x) public pure returns (int64) { return x.f1 + z.f2; }
}
    "#;

    let mut ns = generate_namespace(src);
    codegen(&mut ns, &Options::default());
    let idl = generate_anchor_idl(1, &ns, "0.1.0");

    assert_eq!(idl.types.len(), 3);
    assert_eq!(idl.types[0].name, "Foo");
    assert_eq!(idl.types[1].name, "D_Foo");
    assert_eq!(idl.types[2].name, "D_Foo_1");

    // Name already exists after
    let src = r#"
    contract D {
	struct Foo { int64 f1; }
}

enum Foo { b1, b2, b3 }

contract C {
    struct D_Foo {
        int64 f2;
    }
        function f(Foo y,  D.Foo x, D_Foo z) public pure returns (int64) { return x.f1 + z.f2; }
}
    "#;
    let mut ns = generate_namespace(src);
    codegen(&mut ns, &Options::default());
    let idl = generate_anchor_idl(1, &ns, "0.1.0");

    assert_eq!(idl.types.len(), 3);
    assert_eq!(idl.types[0].name, "Foo");
    assert_eq!(idl.types[1].name, "D_Foo_1");
    assert_eq!(idl.types[2].name, "D_Foo");

    // Pathological name as first argument
    let src = r#"
    contract D {
	struct Foo { int64 f1; }
}

enum Foo { b1, b2, b3 }

contract C {
    struct D_Foo {
        int64 f2;
    }
        function f(D_Foo z, Foo y,  D.Foo x) public pure returns (int64) { return x.f1 + z.f2; }
}
    "#;
    let mut ns = generate_namespace(src);
    codegen(&mut ns, &Options::default());
    let idl = generate_anchor_idl(1, &ns, "0.1.0");

    assert_eq!(idl.types.len(), 3);
    assert_eq!(idl.types[0].name, "D_Foo");
    assert_eq!(idl.types[1].name, "Foo");
    assert_eq!(idl.types[2].name, "D_Foo_1");

    let src = r#"
contract D {
	struct Foo { int64 f1; }
}

enum Foo { b1, b2, b3 }

contract C {
    struct D_Foo {
        int64 f2;
    }

    struct D_Foo_1 {
       int64 f3;
    }

    function f(D_Foo z, D_Foo_1 k, Foo y,  D.Foo x) public pure returns (int64) { return x.f1 + z.f2 + k.f3; }
}
    "#;

    let mut ns = generate_namespace(src);
    codegen(&mut ns, &Options::default());
    let idl = generate_anchor_idl(1, &ns, "0.1.0");

    assert_eq!(idl.types.len(), 4);
    assert_eq!(idl.types[0].name, "D_Foo");
    assert_eq!(idl.types[1].name, "D_Foo_1");
    assert_eq!(idl.types[2].name, "Foo");
    assert_eq!(idl.types[3].name, "D_Foo_2");

    let src = r#"
contract D {
	struct Foo { int64 f1; }
}

enum Foo { b1, b2, b3 }

contract C {
    struct D_Foo {
        int64 f2;
    }

    struct D_Foo_1 {
       int64 f3;
    }

    function f(D_Foo_1 k, D_Foo z, Foo y,  D.Foo x) public pure returns (int64) { return x.f1 + z.f2 + k.f3; }
}
    "#;

    let mut ns = generate_namespace(src);
    codegen(&mut ns, &Options::default());
    let idl = generate_anchor_idl(1, &ns, "0.1.0");

    assert_eq!(idl.types.len(), 4);
    assert_eq!(idl.types[0].name, "D_Foo_1");
    assert_eq!(idl.types[1].name, "D_Foo");
    assert_eq!(idl.types[2].name, "Foo");
    assert_eq!(idl.types[3].name, "D_Foo_2");

    let src = r#"
contract D {
	struct Foo { int64 f1; }
}

enum Foo { b1, b2, b3 }

contract C {
    struct D_Foo {
        int64 f2;
    }

    struct D_Foo_1 {
       int64 f3;
    }

    function f(D_Foo z, Foo y, D_Foo_1 k, D.Foo x) public pure returns (int64) { return x.f1 + z.f2 + k.f3; }
}
    "#;

    let mut ns = generate_namespace(src);
    codegen(&mut ns, &Options::default());
    let idl = generate_anchor_idl(1, &ns, "0.1.0");

    assert_eq!(idl.types.len(), 4);
    assert_eq!(idl.types[0].name, "D_Foo");
    assert_eq!(idl.types[1].name, "Foo");
    assert_eq!(idl.types[2].name, "D_Foo_1");
    assert_eq!(idl.types[3].name, "D_Foo_2");

    let src = r#"
contract D {
	struct Foo { int64 f1; }
}

enum Foo { b1, b2, b3 }

contract C {
    struct D_Foo {
        int64 f2;
    }

    struct D_Foo_1 {
       int64 f3;
    }

    function f(D_Foo z, Foo y,  D.Foo x, D_Foo_1 k) public pure returns (int64) { return x.f1 + z.f2 + k.f3; }
}
    "#;

    let mut ns = generate_namespace(src);
    codegen(&mut ns, &Options::default());
    let idl = generate_anchor_idl(1, &ns, "0.1.0");

    assert_eq!(idl.types.len(), 4);
    assert_eq!(idl.types[0].name, "D_Foo");
    assert_eq!(idl.types[1].name, "Foo");
    assert_eq!(idl.types[2].name, "D_Foo_2");
    assert_eq!(idl.types[3].name, "D_Foo_1");
}

#[test]
fn program_id() {
    let src = r#"
    @program_id("Foo5mMfYo5RhRcWa4NZ2bwFn4Kdhe8rNK5jchxsKrivA")
contract C {
    struct D_Foo {
        int64 f2;
    }

    struct D_Foo_1 {
       int64 f3;
    }

    function f(D_Foo_1 k, D_Foo z) public pure returns (int64) { return z.f2 + k.f3; }
}
    "#;

    let mut ns = generate_namespace(src);
    codegen(&mut ns, &Options::default());
    let idl = generate_anchor_idl(0, &ns, "0.1.0");

    assert!(idl.metadata.is_some());
    assert_eq!(
        idl.metadata.unwrap(),
        json!({"address": "Foo5mMfYo5RhRcWa4NZ2bwFn4Kdhe8rNK5jchxsKrivA"})
    );
}

#[test]
fn data_account_signer() {
    let src = r#"
    contract caller {
        // signer required
        @payer(wallet)
        constructor() {}
    }"#;

    let mut ns = generate_namespace(src);
    codegen(&mut ns, &Options::default());
    let idl = generate_anchor_idl(0, &ns, "0.1.0");

    assert_eq!(idl.instructions.len(), 1);

    // implicit constructor
    assert_eq!(idl.instructions[0].name, "new");
    assert!(idl.instructions[0].docs.is_none());
    assert_eq!(
        idl.instructions[0].accounts,
        vec![
            idl_account("dataAccount", true, true),
            idl_account("wallet", true, true),
            idl_account("systemProgram", false, false),
        ]
    );
    assert!(idl.instructions[0].args.is_empty());
    assert!(idl.instructions[0].returns.is_none());

    let src = r#"
    contract caller {
        // pda, signer NOT required
        @payer(wallet)
        @seed("watchword")
        constructor(address wallet) {}
    }"#;

    let mut ns = generate_namespace(src);
    codegen(&mut ns, &Options::default());
    let idl = generate_anchor_idl(0, &ns, "0.1.0");

    assert_eq!(idl.instructions.len(), 1);

    assert_eq!(idl.instructions[0].name, "new");
    assert!(idl.instructions[0].docs.is_none());
    assert_eq!(
        idl.instructions[0].accounts,
        vec![
            idl_account("dataAccount", true, false),
            idl_account("wallet", true, true),
            idl_account("systemProgram", false, false)
        ]
    );
    assert!(idl.instructions[0].args.len() == 1);
    assert!(idl.instructions[0].returns.is_none());
}

#[test]
fn accounts_call_chain() {
    let src = r#"
    contract Test {
    function call_1() public view returns (uint64) {
        return call_2();
    }

    function call_2() public view returns (uint64) {
        return call_3();
    }

    function call_3() public view returns (uint64) {
        return block.timestamp;
    }
}
    "#;
    let mut ns = generate_namespace(src);
    codegen(&mut ns, &Options::default());
    let idl = generate_anchor_idl(0, &ns, "0.1.0");

    assert_eq!(idl.instructions[0].name, "new");
    assert_eq!(
        idl.instructions[0].accounts,
        vec![idl_account("dataAccount", true, false)]
    );

    assert_eq!(idl.instructions[1].name, "call_1");
    assert_eq!(
        idl.instructions[1].accounts,
        vec![idl_account("clock", false, false)]
    );

    assert_eq!(idl.instructions[2].name, "call_2");
    assert_eq!(idl.instructions[2].accounts, idl.instructions[1].accounts);

    assert_eq!(idl.instructions[3].name, "call_3");
    assert_eq!(idl.instructions[3].accounts, idl.instructions[1].accounts);
}

#[test]
fn accounts_on_recursion() {
    let src = r#"
        contract Test {
    address addr;
    bytes message;
    bytes signature;

    constructor(address addr_, bytes message_, bytes signature_) {
        addr = addr_;
        message = message_;
        signature = signature_;
    }


    function call_1() public view returns (bool, uint64) {
        return call_2();
    }

    function call_2() public view returns (bool, uint64) {
        (bool a, uint64 b) = call_3();
        return (signatureVerify(addr, message, signature), b);
    }

    function call_3() public view returns (bool, uint64) {
        (bool a, uint64 b) = call_1();
        return (a, block.timestamp);
    }
}
    "#;

    let mut ns = generate_namespace(src);
    codegen(&mut ns, &Options::default());
    let idl = generate_anchor_idl(0, &ns, "0.1.0");

    assert_eq!(idl.instructions[0].name, "new");
    assert_eq!(
        idl.instructions[0].accounts,
        vec![idl_account("dataAccount", true, false)]
    );

    assert_eq!(idl.instructions[1].name, "call_1");
    assert_eq!(
        idl.instructions[1].accounts,
        vec![
            idl_account("dataAccount", false, false),
            idl_account("SysvarInstruction", false, false),
            idl_account("clock", false, false),
        ]
    );

    assert_eq!(idl.instructions[2].name, "call_2");
    assert_eq!(idl.instructions[2].accounts, idl.instructions[1].accounts);

    assert_eq!(idl.instructions[3].name, "call_3");
    assert_eq!(
        idl.instructions[3].accounts[0],
        idl.instructions[1].accounts[0]
    );
    assert_eq!(
        idl.instructions[3].accounts[1],
        idl.instructions[1].accounts[2]
    );
    assert_eq!(
        idl.instructions[3].accounts[2],
        idl.instructions[1].accounts[1]
    );
}

#[test]
fn system_account_for_payer_annotation() {
    let src = r#"
    contract Test {
    address addr;
    bytes message;
    bytes signature;

    @payer(addr_)
    constructor(address addr_, bytes message_, bytes signature_) {
        addr = addr_;
        message = message_;
        signature = signature_;
    }

    function call_3() public view returns (bool, uint64) {
        return (true, block.timestamp);
    }
}
    "#;
    let mut ns = generate_namespace(src);
    codegen(&mut ns, &Options::default());
    let idl = generate_anchor_idl(0, &ns, "0.1.0");

    assert_eq!(idl.instructions[0].name, "new");
    assert_eq!(
        idl.instructions[0].accounts,
        vec![
            idl_account("dataAccount", true, true),
            idl_account("addr_", true, true),
            idl_account("systemProgram", false, false),
        ]
    );
}

#[test]
fn calling_system_program() {
    let src = r#"
        import 'solana';

library SystemInstruction {
    address constant systemAddress = address"11111111111111111111111111111111";
    enum Instruction {
        CreateAccount,
        Assign
    }

    function create_account(address from, address to, uint64 lamports, uint64 space, address owner) internal {
        AccountMeta[2] metas = [
            AccountMeta({pubkey: from, is_signer: true, is_writable: true}),
            AccountMeta({pubkey: to, is_signer: true, is_writable: true})
        ];
        bytes bincode = abi.encode(uint32(Instruction.CreateAccount), lamports, space, owner);
        systemAddress.call{accounts: metas}(bincode);
    }
}

contract Test {

    function call_1(address from, address to, uint64 lamports, uint64 space, address owner) public {
        SystemInstruction.create_account(from, to, lamports, space, owner);
    }
}
    "#;

    let mut ns = generate_namespace(src);
    codegen(&mut ns, &Options::default());
    let idl = generate_anchor_idl(1, &ns, "0.1.0");

    assert_eq!(idl.instructions[0].name, "new");

    assert_eq!(idl.instructions[1].name, "call_1");
    assert_eq!(
        idl.instructions[1].accounts,
        vec![idl_account("systemProgram", false, false)]
    );
}

#[test]
fn call_token_program() {
    let src = r#"
    import 'solana';

library TokenProgram {
    address constant systemAddress = address"TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
    enum Instruction {
        CreateAccount,
        Assign
    }

    function create_account(address from, address to, uint64 lamports, uint64 space, address owner) internal {
        AccountMeta[2] metas = [
            AccountMeta({pubkey: from, is_signer: true, is_writable: true}),
            AccountMeta({pubkey: to, is_signer: true, is_writable: true})
        ];
        bytes bincode = abi.encode(uint32(Instruction.CreateAccount), lamports, space, owner);
        systemAddress.call{accounts: metas}(bincode);
    }
}

contract Test {

    function call_1(address from, address to, uint64 lamports, uint64 space, address owner) public {
        TokenProgram.create_account(from, to, lamports, space, owner);
    }
}
    "#;
    let mut ns = generate_namespace(src);
    codegen(&mut ns, &Options::default());
    let idl = generate_anchor_idl(1, &ns, "0.1.0");

    assert_eq!(idl.instructions[0].name, "new");

    assert_eq!(idl.instructions[1].name, "call_1");
    assert_eq!(
        idl.instructions[1].accounts,
        vec![
            idl_account("tokenProgram", false, false),
            idl_account("systemProgram", false, false),
        ]
    );
}

#[test]
fn other_collected_public_keys() {
    let src = r#"
    import 'solana';

@program_id("SysvarRent111111111111111111111111111111111")
interface anchor {
	@selector([0xaf,0xaf,0x6d,0x1f,0x0d,0x98,0x9b,0xed])
	function initialize(bool data1) view external;
}

@program_id("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL")
interface associated {
	@selector([0xaf,0xaf,0x6d,0x1f,0x0d,0x98,0x9b,0xed])
	function initialize(bool data1) view external;
}

@program_id("SysvarC1ock11111111111111111111111111111111")
interface clock_interface {
	@selector([0xaf,0xaf,0x6d,0x1f,0x0d,0x98,0x9b,0xed])
	function initialize(bool data1) view external;
}

@program_id("z7FbDfQDfucxJz5o8jrGLgvSbdoeSqX5VrxBb5TVjHq")
interface other_interface {
	@selector([0xaf,0xaf,0x6d,0x1f,0x0d,0x98,0x9b,0xed])
	function initialize(bool data1) view external;
}

contract Test {
    function call_1() public {
        anchor.initialize{accounts: []}(true);
    }

    function call_2() public {
        associated.initialize{accounts: []}(false);
    }

    function call_3() public {
        clock_interface.initialize{accounts: []}(true);
    }

    function call_4() public {
        other_interface.initialize{accounts: []}(false);
    }
}
    "#;

    let mut ns = generate_namespace(src);
    codegen(&mut ns, &Options::default());
    let idl = generate_anchor_idl(4, &ns, "0.1.0");

    assert_eq!(idl.instructions[0].name, "new");
    assert_eq!(
        idl.instructions[0].accounts,
        vec![idl_account("dataAccount", true, false)]
    );

    assert_eq!(idl.instructions[1].name, "call_1");
    assert_eq!(
        idl.instructions[1].accounts,
        vec![
            idl_account("rent", false, false),
            idl_account("systemProgram", false, false),
        ]
    );

    assert_eq!(idl.instructions[2].name, "call_2");
    assert_eq!(
        idl.instructions[2].accounts,
        vec![
            idl_account("associatedTokenProgram", false, false),
            idl_account("systemProgram", false, false),
        ]
    );

    assert_eq!(idl.instructions[3].name, "call_3");
    assert_eq!(
        idl.instructions[3].accounts,
        vec![
            idl_account("clock", false, false),
            idl_account("systemProgram", false, false),
        ]
    );

    assert_eq!(idl.instructions[4].name, "call_4");
    assert_eq!(
        idl.instructions[4].accounts,
        vec![
            idl_account("systemProgram", false, false),
            idl_account("other_interface_programId", false, false),
        ]
    );
}

#[test]
fn multiple_contracts() {
    let src = r#"
    import 'solana';

contract creator {
    function create_child() external returns (uint64) {
        print("Going to create child");
        Child.new();
        return Child.say_hello();
    }
}

@program_id("Chi1d5XD6nTAp2EyaNGqMxZzUjh6NvhXRxbGHP3D1RaT")
contract Child {
    @payer(payer)
    @space(511 + 7)
    constructor() {
        print("In child constructor");
    }

    function say_hello() public view returns (uint64) {
        print("Hello there");
        return block.slot;
    }
}
    "#;

    let mut ns = generate_namespace(src);
    codegen(&mut ns, &Options::default());
    let idl = generate_anchor_idl(0, &ns, "0.1.0");

    assert_eq!(idl.instructions[0].name, "new");
    assert_eq!(idl.instructions[1].name, "create_child");

    assert_eq!(
        idl.instructions[1].accounts,
        vec![
            idl_account("systemProgram", false, false),
            idl_account("Child_programId", false, false),
            idl_account("payer", true, true),
            idl_account("Child_dataAccount", true, true),
            idl_account("clock", false, false),
        ]
    );
}

#[test]
fn constructor_double_payer() {
    let src = r#"
    import 'solana';

@program_id("SoLDxXQ9GMoa15i4NavZc61XGkas2aom4aNiWT6KUER")
contract Builder {
    @payer(payer_account)
    constructor() {
       BeingBuilt.new("abc");
    }
}


@program_id("SoLGijpEqEeXLEqa9ruh7a6Lu4wogd6rM8FNoR7e3wY")
contract BeingBuilt {
    @space(1024)
    @payer(other_account)
    constructor(@seed bytes my_seed) {}

    function say_this(string text) public pure {
        print(text);
    }
}
    "#;

    let mut ns = generate_namespace(src);
    codegen(&mut ns, &Options::default());
    let idl = generate_anchor_idl(0, &ns, "0.1.0");

    assert_eq!(idl.instructions[0].name, "new");
    assert_eq!(
        idl.instructions[0].accounts,
        vec![
            idl_account("dataAccount", true, true),
            idl_account("payer_account", true, true),
            idl_account("systemProgram", false, false),
            idl_account("BeingBuilt_programId", false, false),
            idl_account("other_account", true, true),
            idl_account("BeingBuilt_dataAccount", true, false),
        ]
    );
}

#[test]
fn modifier() {
    let src1 = r#"
import "solana";

@program_id("CU8sqXecq7pxweQnJq6CARonEApGN2DXcv9ukRBRgnRf")
contract starter {
    bool private value = true;

    modifier test_modifier() {
        print("modifier");
        _;
    }

    @payer(payer)
    constructor() {
        print("Hello, World!");
    }

    function flip() public test_modifier {
            value = !value;
    }

    function get() public view returns (bool) {
            return value;
    }
}
    "#;

    let src2 = r#"
import "solana";

@program_id("CU8sqXecq7pxweQnJq6CARonEApGN2DXcv9ukRBRgnRf")
contract starter {
    bool private value = true;

    @payer(payer)
    constructor() {
        print("Hello, World!");
    }

    function flip() public {
            value = !value;
    }

    function get() public view returns (bool) {
            return value;
    }
}
    "#;

    let mut ns1 = generate_namespace(src1);
    codegen(&mut ns1, &Options::default());
    let idl1 = generate_anchor_idl(0, &ns1, "0.1.0");

    let mut ns2 = generate_namespace(src2);
    codegen(&mut ns2, &Options::default());
    let idl2 = generate_anchor_idl(0, &ns2, "0.1.0");

    assert_eq!(idl1, idl2);
}

#[test]
fn account_transfer_recursive() {
    let src = r#"
contract CT3 {
    @payer(three_payer)
    constructor() {
        CT2.new();
    }
}

@program_id("Ha2EGxARbSYpqNZkkvZUUGEyx3pu7Mg9pvMsuEJuWNjH")
contract CT2 {
    @payer(two_payer)
    constructor() {
        CT1.new(block.timestamp);
    }
}


@program_id("5kQ3iJ43gHNDjqmSAtE1vDu18CiSAfNbRe4v5uoobh3U")
contract CT1 {
    @payer(one_payer)
    constructor(uint64 time) {
        print("{}".format(time));
    }
}
    "#;

    let mut ns = generate_namespace(src);
    codegen(&mut ns, &Options::default());
    let ct3 = generate_anchor_idl(0, &ns, "0.1.0");
    let ct2 = generate_anchor_idl(1, &ns, "0.1.0");
    let ct1 = generate_anchor_idl(2, &ns, "0.1.0");

    assert_eq!(ct1.instructions[0].name, "new");
    assert_eq!(
        ct1.instructions[0].accounts,
        vec![
            idl_account("dataAccount", true, true),
            idl_account("one_payer", true, true),
            idl_account("systemProgram", false, false),
        ]
    );

    assert_eq!(ct2.instructions[0].name, "new");
    assert_eq!(
        ct2.instructions[0].accounts,
        vec![
            idl_account("dataAccount", true, true),
            idl_account("two_payer", true, true),
            idl_account("clock", false, false),
            idl_account("systemProgram", false, false),
            idl_account("CT1_programId", false, false),
            idl_account("one_payer", true, true),
            idl_account("CT1_dataAccount", true, true),
        ]
    );

    assert_eq!(ct3.instructions[0].name, "new");
    assert_eq!(
        ct3.instructions[0].accounts,
        vec![
            idl_account("dataAccount", true, true),
            idl_account("three_payer", true, true),
            idl_account("systemProgram", false, false),
            idl_account("CT2_programId", false, false),
            idl_account("two_payer", true, true),
            idl_account("CT2_dataAccount", true, true),
            idl_account("clock", false, false),
            idl_account("CT1_programId", false, false),
            idl_account("one_payer", true, true),
            idl_account("CT1_dataAccount", true, true),
        ]
    );
}

#[test]
fn default_constructor() {
    let src = r#"
contract Foo {
    uint b;
    function get_b() public returns (uint) {
        return b;
    }
}

contract Other {
    @account(foo_pid)
    function call_foo() external {
        Foo.new{program_id: tx.accounts.foo_pid.key}();
    }
}
    "#;

    let mut ns = generate_namespace(src);
    codegen(&mut ns, &Options::default());
    let idl = generate_anchor_idl(1, &ns, "0.0.1");

    assert_eq!(idl.instructions[1].name, "call_foo");
    assert_eq!(
        idl.instructions[1].accounts,
        vec![
            idl_account("foo_pid", false, false),
            idl_account("Foo_dataAccount", true, false),
            idl_account("systemProgram", false, false)
        ]
    );
}

#[test]
fn function_annotations() {
    let src = r#"
contract Test1 {
    @account(foo)
    @mutableAccount(bar)
    @signer(signerFoo)
    @mutableSigner(signerBar)
    function doThis() external returns (uint64) {
        assert(tx.accounts.signerFoo.is_signer);
        assert(tx.accounts.signerBar.is_signer);

        return tx.accounts.foo.lamports;
    }
}

contract Test2 {
    @account(t1Id)
    function callThat() external returns (uint64) {
        uint64 res = Test1.doThis{program_id: tx.accounts.t1Id.key}();
        return res;
    }
}
    "#;

    let mut ns = generate_namespace(src);
    codegen(&mut ns, &Options::default());
    let idl_1 = generate_anchor_idl(0, &ns, "0.1.0");
    let idl_2 = generate_anchor_idl(1, &ns, "0.1.0");

    assert_eq!(idl_1.instructions[1].name, "doThis");
    assert_eq!(
        idl_1.instructions[1].accounts,
        vec![
            idl_account("foo", false, false),
            idl_account("bar", true, false),
            idl_account("signerFoo", false, true),
            idl_account("signerBar", true, true),
        ]
    );

    assert_eq!(idl_2.instructions[1].name, "callThat");
    assert_eq!(
        idl_2.instructions[1].accounts,
        vec![
            idl_account("t1Id", false, false),
            idl_account("systemProgram", false, false),
            idl_account("foo", false, false),
            idl_account("bar", true, false),
            idl_account("signerFoo", false, true),
            idl_account("signerBar", true, true),
        ]
    );
}

fn idl_account(name: &str, is_mut: bool, is_signer: bool) -> IdlAccountItem {
    IdlAccountItem::IdlAccount(IdlAccount {
        name: name.to_string(),
        is_mut,
        is_signer,
        is_optional: Some(false),
        docs: None,
        pda: None,
        relations: vec![],
    })
}
