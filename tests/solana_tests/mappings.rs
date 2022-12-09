// SPDX-License-Identifier: Apache-2.0

use crate::{account_new, build_solidity, BorshToken};
use num_bigint::BigInt;
use num_traits::{One, Zero};

#[test]
fn simple_mapping() {
    let mut vm = build_solidity(
        r#"
        contract foo {
            mapping (uint64 => uint64) map;

            function set(uint64 index, uint64 val) public {
                map[index] = val;
            }

            function get(uint64 index) public returns (uint64) {
                return map[index];
            }

            function rm(uint64 index) public {
                delete map[index];
            }
        }"#,
    );

    vm.constructor("foo", &[]);

    for i in 0..10 {
        vm.function(
            "set",
            &[
                BorshToken::Uint {
                    width: 64,
                    value: BigInt::from(102 + i),
                },
                BorshToken::Uint {
                    width: 64,
                    value: BigInt::from(300331 + i),
                },
            ],
            None,
        );
    }

    for i in 0..10 {
        let returns = vm.function(
            "get",
            &[BorshToken::Uint {
                width: 64,
                value: BigInt::from(102 + i),
            }],
            None,
        );

        assert_eq!(
            returns,
            vec![BorshToken::Uint {
                width: 64,
                value: BigInt::from(300331 + i)
            }]
        );
    }

    let returns = vm.function(
        "get",
        &[BorshToken::Uint {
            width: 64,
            value: BigInt::from(101u8),
        }],
        None,
    );

    assert_eq!(
        returns,
        vec![BorshToken::Uint {
            width: 64,
            value: BigInt::zero()
        }]
    );

    vm.function(
        "rm",
        &[BorshToken::Uint {
            width: 64,
            value: BigInt::from(104u8),
        }],
        None,
    );

    for i in 0..10 {
        let returns = vm.function(
            "get",
            &[BorshToken::Uint {
                width: 64,
                value: BigInt::from(102 + i),
            }],
            None,
        );

        if 102 + i != 104 {
            assert_eq!(
                returns,
                vec![BorshToken::Uint {
                    width: 64,
                    value: BigInt::from(300331 + i)
                }]
            );
        } else {
            assert_eq!(
                returns,
                vec![BorshToken::Uint {
                    width: 64,
                    value: BigInt::zero(),
                }]
            );
        }
    }
}

#[test]
fn less_simple_mapping() {
    let mut vm = build_solidity(
        r#"
        struct S {
            string f1;
            int64[] f2;
        }

        contract foo {
            mapping (uint => S) map;

            function set_string(uint index, string s) public {
                map[index].f1 = s;
            }

            function add_int(uint index, int64 n) public {
                map[index].f2.push(n);
            }

            function get(uint index) public returns (S) {
                return map[index];
            }

            function rm(uint index) public {
                delete map[index];
            }
        }"#,
    );

    vm.constructor("foo", &[]);

    vm.function(
        "set_string",
        &[
            BorshToken::Uint {
                width: 256,
                value: BigInt::from(12313132131321312311213131u128)
            },
            BorshToken::String(String::from("This is a string which should be a little longer than 32 bytes so we the the abi encoder")),
        ],  None
    );

    vm.function(
        "add_int",
        &[
            BorshToken::Uint {
                width: 256,
                value: BigInt::from(12313132131321312311213131u128),
            },
            BorshToken::Int {
                width: 64,
                value: BigInt::from(102u8),
            },
        ],
        None,
    );

    let returns = vm.function(
        "get",
        &[BorshToken::Uint {
            width: 256,
            value: BigInt::from(12313132131321312311213131u128),
        }],
        None,
    );

    assert_eq!(
        returns,
        vec![BorshToken::Tuple(vec![
            BorshToken::String(String::from("This is a string which should be a little longer than 32 bytes so we the the abi encoder")),
            BorshToken::Array(vec![
                BorshToken::Int{
                    width: 64,
                    value: BigInt::from(102u8)
                },
            ]),
        ])]
    );
}

#[test]
fn string_mapping() {
    let mut vm = build_solidity(
        r#"
        struct S {
            string f1;
            int64[] f2;
        }

        contract foo {
            mapping (string => S) map;

            function set_string(string index, string s) public {
                map[index].f1 = s;
            }

            function add_int(string index, int64 n) public {
                map[index].f2.push(n);
            }

            function get(string index) public returns (S) {
                return map[index];
            }

            function rm(string index) public {
                delete map[index];
            }
        }"#,
    );

    vm.constructor("foo", &[]);

    vm.function(
        "set_string",
        &[
            BorshToken::String(String::from("a")),
            BorshToken::String(String::from("This is a string which should be a little longer than 32 bytes so we the the abi encoder")),
        ], None
    );

    vm.function(
        "add_int",
        &[
            BorshToken::String(String::from("a")),
            BorshToken::Int {
                width: 64,
                value: BigInt::from(102u8),
            },
        ],
        None,
    );

    let returns = vm.function("get", &[BorshToken::String(String::from("a"))], None);

    assert_eq!(
        returns,
        vec![BorshToken::Tuple(vec![
            BorshToken::String(String::from("This is a string which should be a little longer than 32 bytes so we the the abi encoder")),
            BorshToken::Array(vec![
                BorshToken::Int{
                    width: 64,
                    value: BigInt::from(102u8)
                },
            ]),
        ])]
    );
}

#[test]
fn contract_mapping() {
    let mut vm = build_solidity(
        r#"
        interface I {}

        contract foo {
            mapping (I => string) public map;

            function set(I index, string s) public {
                map[index] = s;
            }

            function get(I index) public returns (string) {
                return map[index];
            }

            function rm(I index) public {
                delete map[index];
            }
        }"#,
    );

    vm.constructor("foo", &[]);

    let index = BorshToken::Address(account_new());

    vm.function(
        "set",
        &[
            index.clone(),
            BorshToken::String(String::from("This is a string which should be a little longer than 32 bytes so we the the abi encoder")),
        ], None
    );

    let returns = vm.function("get", &[index.clone()], None);

    assert_eq!(
        returns,
        vec![BorshToken::String(String::from("This is a string which should be a little longer than 32 bytes so we the the abi encoder"))]
    );

    vm.function("rm", &[index.clone()], None);

    let returns = vm.function("get", &[index], None);

    assert_eq!(returns, vec![BorshToken::String(String::from(""))]);
}

#[test]
fn mapping_in_mapping() {
    let mut vm = build_solidity(
        r#"
        contract foo {
            mapping (string => mapping(int64 => byte)) public map;

            function set(string s, int64 n, bytes1 v) public {
                map[s][n] = v;
            }
        }"#,
    );

    vm.constructor("foo", &[]);

    vm.function(
        "set",
        &[
            BorshToken::String(String::from("a")),
            BorshToken::Int {
                width: 64,
                value: BigInt::from(102u8),
            },
            BorshToken::FixedBytes(vec![0x98]),
        ],
        None,
    );

    let returns = vm.function(
        "map",
        &[
            BorshToken::String(String::from("a")),
            BorshToken::Int {
                width: 64,
                value: BigInt::from(102u8),
            },
        ],
        None,
    );

    assert_eq!(returns, vec![BorshToken::FixedBytes(vec![0x98])]);

    let returns = vm.function(
        "map",
        &[
            BorshToken::String(String::from("a")),
            BorshToken::Int {
                width: 64,
                value: BigInt::from(103u8),
            },
        ],
        None,
    );

    assert_eq!(returns, vec![BorshToken::FixedBytes(vec![0])]);

    let returns = vm.function(
        "map",
        &[
            BorshToken::String(String::from("b")),
            BorshToken::Int {
                width: 64,
                value: BigInt::from(102u8),
            },
        ],
        None,
    );

    assert_eq!(returns, vec![BorshToken::FixedBytes(vec![0])]);
}

#[test]
fn sparse_array() {
    let mut vm = build_solidity(
        r#"
        struct S {
            string f1;
            int64[] f2;
        }

        contract foo {
            S[1e9] map;

            function set_string(uint index, string s) public {
                map[index].f1 = s;
            }

            function add_int(uint index, int64 n) public {
                map[index].f2.push(n);
            }

            function get(uint index) public returns (S) {
                return map[index];
            }

            function rm(uint index) public {
                delete map[index];
            }
        }"#,
    );

    vm.constructor("foo", &[]);

    vm.function(
        "set_string",
        &[
            BorshToken::Uint{
                width: 256,
                value: BigInt::from(909090909u64)
            },
            BorshToken::String(String::from("This is a string which should be a little longer than 32 bytes so we the the abi encoder")),
        ], None
    );

    vm.function(
        "add_int",
        &[
            BorshToken::Uint {
                width: 256,
                value: BigInt::from(909090909u64),
            },
            BorshToken::Int {
                width: 64,
                value: BigInt::from(102u8),
            },
        ],
        None,
    );

    let returns = vm.function(
        "get",
        &[BorshToken::Uint {
            width: 256,
            value: BigInt::from(909090909u64),
        }],
        None,
    );

    assert_eq!(
        returns,
        vec![BorshToken::Tuple(vec![
            BorshToken::String(String::from("This is a string which should be a little longer than 32 bytes so we the the abi encoder")),
            BorshToken::Array(vec![
                BorshToken::Int{
                    width: 64,
                    value: BigInt::from(102u8)
                },
            ]),
        ])]
    );
}

#[test]
fn massive_sparse_array() {
    let mut vm = build_solidity(
        r#"
        struct S {
            string f1;
            int64[] f2;
        }

        contract foo {
            S[1e24] map;

            function set_string(uint index, string s) public {
                map[index].f1 = s;
            }

            function add_int(uint index, int64 n) public {
                map[index].f2.push(n);
            }

            function get(uint index) public returns (S) {
                return map[index];
            }

            function rm(uint index) public {
                delete map[index];
            }
        }"#,
    );

    vm.constructor("foo", &[]);

    vm.function(
        "set_string",
        &[
            BorshToken::Uint {
                width: 256,
                value: BigInt::from(786868768768678687686877u128)
            },
            BorshToken::String(String::from("This is a string which should be a little longer than 32 bytes so we the the abi encoder")),
        ], None
    );

    vm.function(
        "add_int",
        &[
            BorshToken::Uint {
                width: 256,
                value: BigInt::from(786868768768678687686877u128),
            },
            BorshToken::Int {
                width: 64,
                value: BigInt::from(102u8),
            },
        ],
        None,
    );

    let returns = vm.function(
        "get",
        &[BorshToken::Uint {
            width: 256,
            value: BigInt::from(786868768768678687686877u128),
        }],
        None,
    );

    assert_eq!(
        returns,
        vec![BorshToken::Tuple(vec![
            BorshToken::String(String::from("This is a string which should be a little longer than 32 bytes so we the the abi encoder")),
            BorshToken::Array(vec![
                BorshToken::Int {
                    width: 64,
                    value: BigInt::from(102u8)
                },
            ]),
        ])]
    );
}

#[test]
fn mapping_in_dynamic_array() {
    let mut vm = build_solidity(
        r#"
        contract foo {
            mapping (uint64 => uint64)[] public map;
            int64 public number;

            function set(uint64 array_no, uint64 index, uint64 val) public {
                map[array_no][index] = val;
            }

            function rm(uint64 array_no, uint64 index) public {
                delete map[array_no][index];
            }

            function push() public {
                map.push();
            }

            function pop() public {
                map.pop();
            }

            function setNumber(int64 x) public {
                number = x;
            }

            function length() public returns (uint64) {
                return map.length;
            }
        }"#,
    );

    vm.constructor("foo", &[]);

    vm.function(
        "setNumber",
        &[BorshToken::Int {
            width: 64,
            value: BigInt::from(2147483647),
        }],
        None,
    );

    vm.function("push", &[], None);
    vm.function("push", &[], None);

    for array_no in 0..2 {
        for i in 0..10 {
            vm.function(
                "set",
                &[
                    BorshToken::Uint {
                        width: 64,
                        value: BigInt::from(array_no),
                    },
                    BorshToken::Uint {
                        width: 64,
                        value: BigInt::from(102 + i + array_no * 500),
                    },
                    BorshToken::Uint {
                        width: 64,
                        value: BigInt::from(300331 + i),
                    },
                ],
                None,
            );
        }
    }

    for array_no in 0..2 {
        for i in 0..10 {
            let returns = vm.function(
                "map",
                &[
                    BorshToken::Uint {
                        width: 256,
                        value: BigInt::from(array_no),
                    },
                    BorshToken::Uint {
                        width: 64,
                        value: BigInt::from(102 + i + array_no * 500),
                    },
                ],
                None,
            );

            assert_eq!(
                returns,
                vec![BorshToken::Uint {
                    width: 64,
                    value: BigInt::from(300331 + i)
                },]
            );
        }
    }

    let returns = vm.function(
        "map",
        &[
            BorshToken::Uint {
                width: 256,
                value: BigInt::zero(),
            },
            BorshToken::Uint {
                width: 64,
                value: BigInt::from(101u8),
            },
        ],
        None,
    );

    assert_eq!(
        returns,
        vec![BorshToken::Uint {
            width: 64,
            value: BigInt::zero()
        }]
    );

    vm.function(
        "rm",
        &[
            BorshToken::Uint {
                width: 64,
                value: BigInt::zero(),
            },
            BorshToken::Uint {
                width: 64,
                value: BigInt::from(104u8),
            },
        ],
        None,
    );

    for i in 0..10 {
        let returns = vm.function(
            "map",
            &[
                BorshToken::Uint {
                    width: 256,
                    value: BigInt::zero(),
                },
                BorshToken::Uint {
                    width: 64,
                    value: BigInt::from(102 + i),
                },
            ],
            None,
        );

        if 102 + i != 104 {
            assert_eq!(
                returns,
                vec![BorshToken::Uint {
                    width: 64,
                    value: BigInt::from(300331 + i)
                },]
            );
        } else {
            assert_eq!(
                returns,
                vec![BorshToken::Uint {
                    width: 64,
                    value: BigInt::zero()
                }]
            );
        }
    }

    let returns = vm.function("length", &[], None);
    assert_eq!(
        returns,
        vec![BorshToken::Uint {
            width: 64,
            value: BigInt::from(2u8)
        }]
    );

    vm.function("pop", &[], None);

    let returns = vm.function("length", &[], None);
    assert_eq!(
        returns,
        vec![BorshToken::Uint {
            width: 64,
            value: BigInt::one()
        }]
    );

    vm.function("pop", &[], None);

    let returns = vm.function("length", &[], None);
    assert_eq!(
        returns,
        vec![BorshToken::Uint {
            width: 64,
            value: BigInt::zero()
        }]
    );

    let returns = vm.function("number", &[], None);

    assert_eq!(
        returns,
        vec![BorshToken::Int {
            width: 64,
            value: BigInt::from(2147483647u64)
        }]
    );
}

#[test]
fn mapping_in_struct_in_dynamic_array() {
    let mut vm = build_solidity(
        r#"
        contract foo {
            struct A {
                mapping(uint256 => uint256) a;
            }

            A[] private map;
            int64 public number;

            function set(uint64 array_no, uint64 index, uint64 val) public {
                map[array_no].a[index] = val;
            }

            function get(uint64 array_no, uint64 index) public returns (uint256) {
                return map[array_no].a[index];
            }

            function rm(uint64 array_no, uint64 index) public {
                delete map[array_no].a[index];
            }

            function push() public {
                map.push();
            }

            function pop() public {
                map.pop();
            }

            function setNumber(int64 x) public {
                number = x;
            }
        }"#,
    );

    vm.constructor("foo", &[]);

    vm.function(
        "setNumber",
        &[BorshToken::Int {
            width: 64,
            value: BigInt::from(2147483647),
        }],
        None,
    );

    vm.function("push", &[], None);
    vm.function("push", &[], None);

    for array_no in 0..2 {
        for i in 0..10 {
            vm.function(
                "set",
                &[
                    BorshToken::Uint {
                        width: 64,
                        value: BigInt::from(array_no),
                    },
                    BorshToken::Uint {
                        width: 64,
                        value: BigInt::from(102 + i + array_no * 500),
                    },
                    BorshToken::Uint {
                        width: 64,
                        value: BigInt::from(300331 + i),
                    },
                ],
                None,
            );
        }
    }

    for array_no in 0..2 {
        for i in 0..10 {
            let returns = vm.function(
                "get",
                &[
                    BorshToken::Uint {
                        width: 64,
                        value: BigInt::from(array_no),
                    },
                    BorshToken::Uint {
                        width: 64,
                        value: BigInt::from(102 + i + array_no * 500),
                    },
                ],
                None,
            );

            assert_eq!(
                returns,
                vec![BorshToken::Uint {
                    width: 256,
                    value: BigInt::from(300331 + i)
                },]
            );
        }
    }

    let returns = vm.function(
        "get",
        &[
            BorshToken::Uint {
                width: 64,
                value: BigInt::zero(),
            },
            BorshToken::Uint {
                width: 64,
                value: BigInt::from(101u8),
            },
        ],
        None,
    );

    assert_eq!(
        returns,
        vec![BorshToken::Uint {
            width: 256,
            value: BigInt::zero(),
        },]
    );

    vm.function(
        "rm",
        &[
            BorshToken::Uint {
                width: 64,
                value: BigInt::zero(),
            },
            BorshToken::Uint {
                width: 64,
                value: BigInt::from(104u8),
            },
        ],
        None,
    );

    for i in 0..10 {
        let returns = vm.function(
            "get",
            &[
                BorshToken::Uint {
                    width: 64,
                    value: BigInt::zero(),
                },
                BorshToken::Uint {
                    width: 64,
                    value: BigInt::from(102 + i),
                },
            ],
            None,
        );

        if 102 + i != 104 {
            assert_eq!(
                returns,
                vec![BorshToken::Uint {
                    width: 256,
                    value: BigInt::from(300331 + i)
                }]
            );
        } else {
            assert_eq!(
                returns,
                vec![BorshToken::Uint {
                    width: 256,
                    value: BigInt::zero()
                }]
            );
        }
    }

    vm.function("pop", &[], None);
    vm.function("pop", &[], None);

    let returns = vm.function("number", &[], None);

    assert_eq!(
        returns,
        vec![BorshToken::Int {
            width: 64,
            value: BigInt::from(2147483647u64),
        }]
    );
}

#[test]
fn mapping_delete() {
    let mut vm = build_solidity(
        r#"
contract DeleteTest {

    struct data_struct  {
        address addr1;
	    address addr2;
    }

    mapping(uint => data_struct) example;

    function addData() public  {
        data_struct dt = data_struct({addr1: address(this), addr2: msg.sender});
        uint id = 1;
        example[id] = dt;
    }

    function deltest() external {
        uint id = 1;
        delete example[id];
    }

    function get() public view returns (data_struct calldata) {
        uint id = 1;
        return example[id];
    }
}
        "#,
    );

    vm.constructor("DeleteTest", &[]);
    let _ = vm.function("addData", &[], None);
    let _ = vm.function("deltest", &[], None);
    let returns = vm.function("get", &[], None);
    assert_eq!(
        returns,
        vec![BorshToken::Tuple(vec![
            BorshToken::FixedBytes(vec![
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0
            ]),
            BorshToken::FixedBytes(vec![
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0
            ])
        ])],
    );
}

#[test]
fn mapping_within_struct() {
    let mut vm = build_solidity(
        r#"
contract CrowdFunding {
    struct Funder {
        address addr;
        uint amount;
    }

    struct Campaign {
        mapping(uint => Funder)[2] arr_mp;
        mapping (uint => Funder) funders;
    }

    uint numCampaigns;
    mapping (uint => Campaign) campaigns;


function newCampaign() public returns (uint campaignID) {
    campaignID = numCampaigns++;
    Campaign storage _campaign = campaigns[campaignID];
    _campaign.funders[0] = Funder(msg.sender, 100);
    _campaign.arr_mp[1][0] = Funder(msg.sender, 105);
}

function getAmt() public view returns (uint) {
    Campaign storage _campaign = campaigns[numCampaigns - 1];
    return _campaign.funders[0].amount;
}

function getArrAmt() public view returns (uint) {
    Campaign storage _campaign = campaigns[numCampaigns - 1];
    return _campaign.arr_mp[1][0].amount;
}

}
        "#,
    );

    vm.constructor("CrowdFunding", &[]);

    let ret = vm.function("newCampaign", &[], None);

    assert_eq!(
        ret,
        vec![BorshToken::Uint {
            width: 256,
            value: BigInt::zero(),
        }]
    );

    let ret = vm.function("getAmt", &[], None);
    assert_eq!(
        ret,
        vec![BorshToken::Uint {
            width: 256,
            value: BigInt::from(100u8),
        }]
    );

    let ret = vm.function("getArrAmt", &[], None);
    assert_eq!(
        ret,
        vec![BorshToken::Uint {
            width: 256,
            value: BigInt::from(105u8),
        }]
    );
}
