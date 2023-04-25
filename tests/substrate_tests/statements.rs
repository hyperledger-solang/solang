// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity;
use parity_scale_codec::Encode;

#[test]
fn destruct_from_array() {
    #[derive(Encode)]
    struct Arg {
        values: Vec<u32>,
        fakes: Vec<bool>,
        id: Vec<u8>,
    }

    let mut runtime = build_solidity(
        "
        contract BlindAuction {
            function reveal(
                uint32[] values,
                bool[] fakes,
                bytes memory id
            ) public pure returns (bool) {
                (uint32 value, bool fake) = (values[0], fakes[0]);
                return id == abi.encodePacked(value, fake);
            }
        }",
    );

    let values = vec![1];
    let fakes = vec![true];
    let id = vec![1, 0, 0, 0, 1];
    runtime.function("reveal", Arg { values, fakes, id }.encode());
    assert_eq!(runtime.output(), true.encode());
}

#[test]
fn destruct_from_struct() {
    #[derive(Encode)]
    struct S1(Vec<u32>);

    #[derive(Encode)]
    struct S2(Vec<bool>);

    #[derive(Encode)]
    struct Arg {
        values: S1,
        fakes: S2,
        id: Vec<u8>,
    }

    let mut runtime = build_solidity(
        "
        contract BlindAuction {
            struct S1 { uint32[] u; }
            struct S2 { bool[] b; }
        
            function reveal(
                S1 values,
                S2 fakes,
                bytes memory id
            ) external pure returns (bool) {
                (uint32 value, bool fake) = (values.u[0], fakes.b[0]);
                return id == abi.encodePacked(value, fake);
            }
        }",
    );

    let values = S1(vec![1]);
    let fakes = S2(vec![true]);
    let id = vec![1, 0, 0, 0, 1];
    runtime.function("reveal", Arg { values, fakes, id }.encode());
    assert_eq!(runtime.output(), true.encode());
}
