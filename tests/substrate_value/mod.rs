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
