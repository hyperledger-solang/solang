# Hyperledger Solang Solidity parser

This crate is part of [Hyperledger Solang](https://solang.readthedocs.io/). It contains the
parser for Solidity, including the dialects used by Solang for Solana and Substrate.

This parser is compatible with Ethereum Solidity v0.8.20.

`solang-parser` is still `0.*.*`, so breaking changes [may occur at any time](https://semver.org/#spec-item-4). If you must depend on `solang-parser`, we recommend pinning to a specific version, i.e., `=0.y.z`.

```rust
use solang_parser::{pt::{SourceUnitPart, ContractPart}, parse};

let (tree, comments) = parse(r#"
contract flipper {
    bool private value;

    /// Constructor that initializes the `bool` value to the given `init_value`.
    constructor(bool initvalue) {
        value = initvalue;
    }

    /// A message that can be called on instantiated contracts.
    /// This one flips the value of the stored `bool` from `true`
    /// to `false` and vice versa.
    function flip() public {
        value = !value;
    }

    /// Simply returns the current value of our `bool`.
    function get() public view returns (bool) {
        return value;
    }
}
"#, 0).unwrap();

for part in &tree.0 {
    match part {
        SourceUnitPart::ContractDefinition(def) => {
            println!("found contract {:?}", def.name);
            for part in &def.parts {
                match part {
                    ContractPart::VariableDefinition(def) => {
                        println!("variable {:?}", def.name);
                    }
                    ContractPart::FunctionDefinition(def) => {
                        println!("function {:?}", def.name);
                    }
                    _ => (),
                }
            }
        }
        _ => (),
    }
}
```
