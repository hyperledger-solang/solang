use parity_scale_codec::Encode;
use parity_scale_codec_derive::Decode;

use crate::{build_solidity, first_error, no_errors, parse_and_resolve};
use solang::Target;

#[test]
fn external_call_value() {
    let ns = parse_and_resolve(
        r##"
        contract a {
            function test(b t) public {
                t.test{foo: 1}(102);
            }
        }

        contract b {
            int x;

            function test(int32 l) public {
                a f = new a();
            }
        }"##,
        Target::default_substrate(),
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "‘foo’ not a valid call parameter"
    );

    let ns = parse_and_resolve(
        r##"
        contract a {
            function test(b t) public {
                t.test{foo: 1}({l: 102});
            }
        }

        contract b {
            int x;

            function test(int32 l) public {
                a f = new a();
            }
        }"##,
        Target::default_substrate(),
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "‘foo’ not a valid call parameter"
    );

    let ns = parse_and_resolve(
        r##"
        contract a {
            function test(b t) public {
                t.test{salt: 1}({l: 102});
            }
        }

        contract b {
            int x;

            function test(int32 l) public {
                a f = new a();
            }
        }"##,
        Target::default_substrate(),
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "‘salt’ not valid for external calls"
    );

    let ns = parse_and_resolve(
        r##"
        contract a {
            function test(b t) public {
                t.test{value: 1, value: 2}({l: 102});
            }
        }

        contract b {
            int x;

            function test(int32 l) public {
                a f = new a();
            }
        }"##,
        Target::default_substrate(),
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "‘value’ specified multiple times"
    );

    let ns = parse_and_resolve(
        r##"
        contract a {
            function test(b t) public {
                t.test{value: 1}{value: 2}({l: 102});
            }
        }

        contract b {
            int x;

            function test(int32 l) public {
                a f = new a();
            }
        }"##,
        Target::default_substrate(),
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "‘value’ specified multiple times"
    );

    let ns = parse_and_resolve(
        r##"
        contract a {
            function test(b t) public {
                t.test{value: 1}{value = 2;}({l: 102});
            }
        }

        contract b {
            int x;

            function test(int32 l) public {
                a f = new a();
            }
        }"##,
        Target::default_substrate(),
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "code block found where list of call arguments expected, like ‘{gas: 5000}’"
    );

    let ns = parse_and_resolve(
        r##"
        contract a {
            function test(b t) public {
                t.test{value: 1}{}({l: 102});
            }
        }

        contract b {
            int x;

            function test(int32 l) public {
                a f = new a();
            }
        }"##,
        Target::default_substrate(),
    );

    assert_eq!(first_error(ns.diagnostics), "missing call arguments");

    let ns = parse_and_resolve(
        r##"
        contract a {
            function test(int32 l) public {
            }
        }

        contract b {
            int x;

            function test() public {
                a f = new a();
                f.test{value: 1023}(501);
            }
        }"##,
        Target::default_substrate(),
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "sending value to function ‘test’ which is not payable"
    );

    let ns = parse_and_resolve(
        r##"
        contract a {
            function test(int32 l) public {
            }
        }

        contract b {
            int x;

            function test() public {
                a f = new a();
                f.test{value: 1023}({l: 501});
            }
        }"##,
        Target::default_substrate(),
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "sending value to function ‘test’ which is not payable"
    );

    let ns = parse_and_resolve(
        r##"
        contract a {
            function test(int32 l) public payable {
            }
        }

        contract b {
            int x;

            function test() public {
                uint256 x = 500;
                a f = new a();
                f.test{value: x}({l: 501});
            }
        }"##,
        Target::default_substrate(),
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "conversion truncates uint256 to uint128, as value is type uint128 on target substrate"
    );

    let ns = parse_and_resolve(
        r##"
        contract a {
            function test(int32 l) public {
            }
        }

        contract b {
            int x;

            function test() public {
                a f = new a();
                f.test{value: 2-2}({l: 501});
            }
        }"##,
        Target::default_substrate(),
    );

    no_errors(ns.diagnostics);

    let ns = parse_and_resolve(
        r##"
        contract a {
            function test(int32 l) public {
            }
        }

        contract b {
            int x;

            function test() public {
                a f = new a();
                f.test{value: 0*10}(501);
            }
        }"##,
        Target::default_substrate(),
    );

    no_errors(ns.diagnostics);

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

    for (address, account) in runtime.accounts {
        if address == runtime.vm.address {
            continue;
        }

        assert_eq!(account.1, 1523);
    }

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

    for (address, account) in runtime.accounts {
        if address == runtime.vm.address {
            continue;
        }

        assert_eq!(account.1, 1523);
    }
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

    for (address, account) in runtime.accounts {
        if address == runtime.vm.address {
            continue;
        }

        assert_eq!(account.1, 500);
    }

    let mut runtime = build_solidity(
        r##"
        contract b {
            function step1() public {
                a f = new a{value: 0}();
            }
        }

        contract a {
            function test(int32 l) public payable {
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());

    runtime.function("step1", Vec::new());

    for (address, account) in runtime.accounts {
        if address == runtime.vm.address {
            continue;
        }

        assert_eq!(account.1, 0);
    }

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

    for (address, account) in runtime.accounts {
        if address == runtime.vm.address {
            continue;
        }

        assert_eq!(account.1, 499);
    }

    let mut runtime = build_solidity(
        r##"
        contract b {
            function step1() public {
                try new a{value: 511}() {
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

    for (address, account) in runtime.accounts {
        if address == runtime.vm.address {
            continue;
        }

        assert_eq!(account.1, 511);
    }

    let mut runtime = build_solidity(
        r##"
        contract b {
            function step1() public {
                try new a{value: 511}() returns (a) {
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

    for (address, account) in runtime.accounts {
        if address == runtime.vm.address {
            continue;
        }

        assert_eq!(account.1, 511);
    }
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
    let ns = parse_and_resolve(
        r##"
        contract b {
            function step1() public returns (address payable) {
                return payable(this);
            }
        }"##,
        Target::default_substrate(),
    );

    no_errors(ns.diagnostics);

    let ns = parse_and_resolve(
        r##"
        contract b {
            function step1() public returns (address) {
                return this;
            }
        }"##,
        Target::default_substrate(),
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "implicit conversion to address from contract b not allowed"
    );

    let ns = parse_and_resolve(
        r##"
        contract b {
            function step1(b other) public {
                this = other;
            }
        }"##,
        Target::default_substrate(),
    );

    assert_eq!(first_error(ns.diagnostics), "expression is not assignable");

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

    assert_eq!(runtime.vm.output, runtime.vm.address);

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

    assert_eq!(runtime.vm.output, runtime.vm.address);

    #[derive(Debug, PartialEq, Encode, Decode)]
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

    assert_eq!(runtime.vm.output, Ret(102).encode());

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

    assert_eq!(runtime.vm.output, runtime.vm.address);

    let ns = parse_and_resolve(
        r##"
        contract b {
            int32 s;

            function step1() public returns (int32) {
                this.other(102);
                return s;
            }

            function other(int32 n) private {
                s = n;
            }
        }"##,
        Target::default_substrate(),
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "function ‘other’ is not ‘public’ or ‘external’"
    );

    let ns = parse_and_resolve(
        r##"
        contract b {
            int32 s;

            function step1() public returns (int32) {
                this.other({n: 102});
                return s;
            }

            function other(int32 n) private {
                s = n;
            }
        }"##,
        Target::default_substrate(),
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "function ‘other’ is not ‘public’ or ‘external’"
    );
}

#[test]
fn balance() {
    let ns = parse_and_resolve(
        r##"
        contract b {
            function step1(address j) public returns (uint128) {
                return j.balance;
            }
        }"##,
        Target::default_substrate(),
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "substrate can only retrieve balance of this, like ‘address(this).balance’"
    );

    let ns = parse_and_resolve(
        r##"
        contract b {
            function step1(b j) public returns (uint128) {
                return j.balance;
            }
        }"##,
        Target::default_substrate(),
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "contract ‘b’ has no public function ‘balance’"
    );

    let ns = parse_and_resolve(
        r##"
        contract b {
            function step1(address payable j) public returns (uint128) {
                return j.balance;
            }
        }"##,
        Target::default_substrate(),
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "substrate can only retrieve balance of this, like ‘address(this).balance’"
    );

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

    runtime.accounts.get_mut(&runtime.vm.address).unwrap().1 = 315;

    runtime.function("step1", Vec::new());

    assert_eq!(runtime.vm.output, 315u128.to_le_bytes());

    runtime.function("step2", Vec::new());

    assert_eq!(runtime.vm.output, 500u128.to_le_bytes());
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
    assert_eq!(runtime.accounts.get_mut(&runtime.vm.address).unwrap().1, 0);

    runtime.function_expect_failure("step2", Vec::new());
    assert_eq!(
        runtime.accounts.get_mut(&runtime.vm.address).unwrap().1,
        511
    );
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
    assert_eq!(runtime.vm.output, true.encode());

    for (address, account) in runtime.accounts {
        if address == runtime.vm.address {
            continue;
        }

        assert_eq!(account.1, 1011);
    }

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

    assert_eq!(runtime.vm.output, true.encode());

    for (address, account) in runtime.accounts {
        if address == runtime.vm.address {
            continue;
        }

        assert_eq!(account.1, 1011);
    }

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

    for (address, account) in runtime.accounts {
        if address == runtime.vm.address {
            continue;
        }

        assert_eq!(account.1, 1011);
    }

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

    for (address, account) in runtime.accounts {
        if address == runtime.vm.address {
            continue;
        }

        assert_eq!(account.1, 1011);
    }
}
