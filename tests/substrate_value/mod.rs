use parity_scale_codec::Encode;
use parity_scale_codec_derive::{Decode, Encode};

use super::{build_solidity, first_error, no_errors};
use solang::{parse_and_resolve, Target};

#[test]
fn external_call_value() {
    let (_, errors) = parse_and_resolve(
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
        Target::Substrate,
    );

    assert_eq!(first_error(errors), "‘foo’ not a valid call parameter");

    let (_, errors) = parse_and_resolve(
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
        Target::Substrate,
    );

    assert_eq!(first_error(errors), "‘foo’ not a valid call parameter");

    let (_, errors) = parse_and_resolve(
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
        Target::Substrate,
    );

    assert_eq!(first_error(errors), "‘salt’ not valid for external calls");

    let (_, errors) = parse_and_resolve(
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
        Target::Substrate,
    );

    assert_eq!(first_error(errors), "‘value’ specified multiple times");

    let (_, errors) = parse_and_resolve(
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
        Target::Substrate,
    );

    assert_eq!(first_error(errors), "‘value’ specified multiple times");

    let (_, errors) = parse_and_resolve(
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
        Target::Substrate,
    );

    assert_eq!(
        first_error(errors),
        "code block found where list of call arguments expected, like ‘{gas: 5000}’"
    );

    let (_, errors) = parse_and_resolve(
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
        Target::Substrate,
    );

    assert_eq!(first_error(errors), "missing call arguments");

    let (_, errors) = parse_and_resolve(
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
        Target::Substrate,
    );

    assert_eq!(
        first_error(errors),
        "sending value to function ‘test’ which is not payable"
    );

    let (_, errors) = parse_and_resolve(
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
        Target::Substrate,
    );

    assert_eq!(
        first_error(errors),
        "sending value to function ‘test’ which is not payable"
    );

    let (_, errors) = parse_and_resolve(
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
        Target::Substrate,
    );

    no_errors(errors);

    let (_, errors) = parse_and_resolve(
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
        Target::Substrate,
    );

    no_errors(errors);

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
    let (_, errors) = parse_and_resolve(
        r##"
        contract b {
            function step1() public returns (address payable) {
                return payable(this);
            }
        }"##,
        Target::Substrate,
    );

    no_errors(errors);

    let (_, errors) = parse_and_resolve(
        r##"
        contract b {
            function step1() public returns (address) {
                return this;
            }
        }"##,
        Target::Substrate,
    );

    assert_eq!(
        first_error(errors),
        "implicit conversion to address from contract b not allowed"
    );

    let (_, errors) = parse_and_resolve(
        r##"
        contract b {
            function step1(b other) public {
                this = other;
            }
        }"##,
        Target::Substrate,
    );

    assert_eq!(first_error(errors), "expression is not assignable");

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

    assert_eq!(runtime.vm.scratch, runtime.vm.address);

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

    assert_eq!(runtime.vm.scratch, runtime.vm.address);

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

    assert_eq!(runtime.vm.scratch, Ret(102).encode());

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

    assert_eq!(runtime.vm.scratch, runtime.vm.address);

    let (_, errors) = parse_and_resolve(
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
        Target::Substrate,
    );

    assert_eq!(
        first_error(errors),
        "function ‘other’ is not ‘public’ or ‘extern’"
    );

    let (_, errors) = parse_and_resolve(
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
        Target::Substrate,
    );

    assert_eq!(
        first_error(errors),
        "function ‘other’ is not ‘public’ or ‘extern’"
    );
}

#[test]
fn balance() {
    let (_, errors) = parse_and_resolve(
        r##"
        contract b {
            function step1(address j) public returns (uint128) {
                return j.balance;
            }
        }"##,
        Target::Substrate,
    );

    assert_eq!(
        first_error(errors),
        "substrate can only retrieve balance of this, like ‘address(this).balance’"
    );

    let (_, errors) = parse_and_resolve(
        r##"
        contract b {
            function step1(b j) public returns (uint128) {
                return j.balance;
            }
        }"##,
        Target::Substrate,
    );

    assert_eq!(first_error(errors), "‘balance’ not found");

    let (_, errors) = parse_and_resolve(
        r##"
        contract b {
            function step1(address payable j) public returns (uint128) {
                return j.balance;
            }
        }"##,
        Target::Substrate,
    );

    assert_eq!(
        first_error(errors),
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

    assert_eq!(runtime.vm.scratch, 315u128.to_le_bytes());

    runtime.function("step2", Vec::new());

    assert_eq!(runtime.vm.scratch, 500u128.to_le_bytes());
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
            function goaway(address payable from) public returns (bool) {
                selfdestruct(from);
            }
        }"##,
    );
    runtime.constructor(0, Vec::new());

    runtime.function("step1", Vec::new());
    assert_eq!(runtime.accounts.get_mut(&runtime.vm.address).unwrap().1, 0);

    runtime.function_expect_return("step2", Vec::new(), 1);
    assert_eq!(
        runtime.accounts.get_mut(&runtime.vm.address).unwrap().1,
        511
    );
}
