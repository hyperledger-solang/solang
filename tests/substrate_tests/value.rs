// SPDX-License-Identifier: Apache-2.0

use parity_scale_codec::{Decode, Encode};

use crate::build_solidity;

#[test]
fn external_call_value() {
    let mut runtime = build_solidity(
        r##"
        contract b {
            a f;

            function step1() public {
                f = new a();
            }

            function step2() public {
                f.test{value: 1023}(501);
            }
        }

        contract a {
            function test(int32 l) public payable {
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());

    runtime.function("step1", Vec::new());
    runtime.function("step2", Vec::new());

    // Minimum balance + transferred value = 500 + 1023
    assert_eq!(runtime.balance(2), 1523);

    let mut runtime = build_solidity(
        r##"
        contract b {
            function step1() public {
                a f = new a();
                try f.test{value: 1023}(501) {
                    //
                }
                catch (bytes) {
                    //
                }
            }
        }

        contract a {
            function test(int32 l) public payable {
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());

    runtime.function("step1", Vec::new());

    // Minimum balance + transferred value = 500 + 1023
    assert_eq!(runtime.balance(2), 1523);
}

#[test]
fn constructor_value() {
    let mut runtime = build_solidity(
        r##"
        contract b {
            function step1() public {
                a f = new a();
            }
        }

        contract a {
            function test(int32 l) public payable {
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());

    runtime.function("step1", Vec::new());

    assert_eq!(runtime.balance(2), 500);

    let mut runtime = build_solidity(
        r##"
        contract b {
            function step1() public {
                a f = (new a){value: 0}();
            }
        }

        contract a {
            function test(int32 l) public payable {
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());

    runtime.function("step1", Vec::new());

    assert_eq!(runtime.balance(2), 0);

    let mut runtime = build_solidity(
        r##"
        contract b {
            function step1() public {
                a f = new a{value: 499}();
            }
        }

        contract a {
            function test(int32 l) public payable {
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());

    runtime.function("step1", Vec::new());

    assert_eq!(runtime.balance(2), 499);

    let mut runtime = build_solidity(
        r##"
        contract b {
            function step1() public {
                try (new a{value: 511})() {
                    //
                }
                catch (bytes) {
                    //
                }
            }
        }

        contract a {
            function test(int32 l) public payable {
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());

    runtime.function("step1", Vec::new());

    assert_eq!(runtime.balance(2), 511);

    let mut runtime = build_solidity(
        r##"
        contract b {
            function step1() public {
                try (new a){value: 511}() returns (a) {
                    //
                }
                catch (bytes) {
                    //
                }
            }
        }

        contract a {
            function test(int32 l) public payable {
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());

    runtime.function("step1", Vec::new());

    assert_eq!(runtime.balance(2), 511)
}

#[test]
fn constructor_salt() {
    let mut runtime = build_solidity(
        r##"
        contract b {
            function step1() public {
                a f = new a{salt: 0}();
            }
        }

        contract a {
            function test(int32 l) public payable {
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());

    runtime.function("step1", Vec::new());

    let mut runtime = build_solidity(
        r##"
        contract b {
            function step1() public {
                a f = new a{salt: 1}();
            }
        }

        contract a {
            function test(int32 l) public payable {
            }
        }"##,
    );
    runtime.constructor(0, Vec::new());

    runtime.function("step1", Vec::new());

    // we can instantiate the same contract if we provide a different contract
    let mut runtime = build_solidity(
        r##"
        contract b {
            function step1() public {
                a f = new a{salt: 1}();
                f = new a{salt: 2}();
            }
        }

        contract a {
            function test(int32 l) public payable {
            }
        }"##,
    );
    runtime.constructor(0, Vec::new());

    runtime.function("step1", Vec::new());
}

#[test]
fn this_address() {
    let mut runtime = build_solidity(
        r##"
        contract b {
            function step1() public returns (address) {
                return address(this);
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());

    runtime.function("step1", Vec::new());

    assert_eq!(runtime.output(), runtime.caller());

    let mut runtime = build_solidity(
        r##"
        contract b {
            function step1() public returns (b) {
                return this;
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());

    runtime.function("step1", Vec::new());

    assert_eq!(runtime.output(), runtime.caller());

    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct Ret(u32);

    let mut runtime = build_solidity(
        r##"
        contract b {
            int32 s;

            function step1() public returns (int32) {
                this.other(102);
                return s;
            }

            function other(int32 n) public {
                s = n;
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());

    runtime.function("step1", Vec::new());

    assert_eq!(runtime.output(), Ret(102).encode());

    let mut runtime = build_solidity(
        r##"
        contract b {
            function step1() public returns (b) {
                return this;
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());

    runtime.function("step1", Vec::new());

    assert_eq!(runtime.output(), runtime.caller());
}

#[test]
fn balance() {
    let mut runtime = build_solidity(
        r##"
        contract b {
            other o;

            constructor() public {
                o = new other();
            }

            function step1() public returns (uint128) {
                return address(this).balance;
            }

            function step2() public returns (uint128) {
                return o.balance();
            }
        }

        contract other {
            function balance() public returns (uint128) {
                return address(this).balance;
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());
    runtime.function("step1", Vec::new());
    // Constructor received 20000 by default, however 500 were sent to "o"
    assert_eq!(runtime.output(), 19500u128.to_le_bytes());

    runtime.function("step2", Vec::new());

    assert_eq!(runtime.output(), 500u128.to_le_bytes());
}

#[test]
fn selfdestruct() {
    let mut runtime = build_solidity(
        r##"
        contract c {
            other o;
            function step1() public {
                o = new other{value: 511}();
            }

            function step2() public {
                o.goaway(payable(address(this)));
            }
        }

        contract other {
            function goaway(address payable recipient) public returns (bool) {
                selfdestruct(recipient);
            }
        }"##,
    );
    runtime.constructor(0, Vec::new());

    runtime.function("step1", Vec::new());
    assert_eq!(runtime.balance(0), 20000 - 511);

    runtime.function("step2", Vec::new());
    assert_eq!(runtime.balance(0), 20000);
}

#[test]
fn send_and_transfer() {
    let mut runtime = build_solidity(
        r##"
        contract c {
            other o;

            constructor() public {
                o = new other();
            }

            function step1() public returns (bool) {
                return payable(o).send(511);
            }
        }

        contract other {
            function giveme() public payable {
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());

    runtime.function("step1", Vec::new());

    // no receive() required for send/transfer
    assert_eq!(runtime.output(), true.encode());
    assert_eq!(runtime.balance(2), 1011);

    let mut runtime = build_solidity(
        r##"
        contract c {
            other o;

            constructor() public {
                o = new other();
            }

            function step1() public returns (bool) {
                return payable(o).send(511);
            }
        }

        contract other {
            receive() external payable {
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());

    runtime.function("step1", Vec::new());

    assert_eq!(runtime.output(), true.encode());
    assert_eq!(runtime.balance(2), 1011);

    let mut runtime = build_solidity(
        r##"
        contract c {
            other o;

            constructor() public {
                o = new other();
            }

            function step1() public {
                payable(o).transfer(511);
            }
        }

        contract other {
            function giveme() public {
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());

    runtime.function("step1", Vec::new());
    assert_eq!(runtime.balance(2), 1011);

    let mut runtime = build_solidity(
        r##"
        contract c {
            other o;

            constructor() public {
                o = new other();
            }

            function step1() public {
                payable(o).transfer(511);
            }
        }

        contract other {
            receive() external payable {
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());

    runtime.function("step1", Vec::new());

    assert_eq!(runtime.balance(2), 1011);
}
