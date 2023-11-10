// SPDX-License-Identifier: Apache-2.0

use solang::file_resolver::FileResolver;
use solang::sema::ast;
use solang::{parse_and_resolve, Target};
use std::ffi::OsStr;

fn parse(src: &'static str) -> ast::Namespace {
    let mut cache = FileResolver::default();
    cache.set_file_contents("test.sol", src.to_string());

    parse_and_resolve(OsStr::new("test.sol"), &mut cache, Target::EVM)
}

fn parse_two_files(src1: &'static str, src2: &'static str) -> ast::Namespace {
    let mut cache = FileResolver::default();
    cache.set_file_contents("test.sol", src1.to_string());
    cache.set_file_contents("test2.sol", src2.to_string());

    parse_and_resolve(OsStr::new("test.sol"), &mut cache, Target::EVM)
}

#[test]
fn emit_event() {
    //Used event
    let case_1 = r#"
    contract usedEvent {
        event Hey(uint8 n);
        function emitEvent(uint8 n) public {
            emit Hey(n);
        }
    }
    "#;

    let ns = parse(case_1);
    assert_eq!(ns.diagnostics.count_warnings(), 0);

    // Unused event
    let case_2 = r#"
    event Hey(uint8 n);
    contract usedEvent {
        event Hey(bool);
        event Hello(uint8 n);
        function emitEvent(uint8 n) public {
            emit Hey(n);
        }
    }
    "#;

    let ns = parse(case_2);
    let warnings = ns.diagnostics.warnings();
    assert_eq!(warnings.len(), 2);
    assert_eq!(warnings[0].message, "event 'Hey' has never been emitted");
    assert_eq!(warnings[1].message, "event 'Hello' has never been emitted");

    // Unused event
    let case_2 = r#"
    contract F {
        event Hey(bool);
        event Hello(uint8 n);
    }
    contract usedEvent is F {
        event Hey(uint8 n);
        function emitEvent(uint8 n) public {
            emit Hey(n);
        }
    }
    "#;

    let ns = parse(case_2);
    let warnings = ns.diagnostics.warnings();
    assert_eq!(warnings.len(), 2);
    assert_eq!(warnings[0].message, "event 'Hey' has never been emitted");
    assert_eq!(warnings[1].message, "event 'Hello' has never been emitted");

    // Unused event
    let case_2 = r#"
    contract F {
        event Hey(uint8 n);
    }
    contract usedEvent is F {
        event Hey(uint8 n);
        function emitEvent(uint8 n) public {
            // reference event in contract F, so our event decl is not used
            emit F.Hey(n);
        }
    }
    "#;

    let ns = parse(case_2);
    assert_eq!(ns.diagnostics.count_warnings(), 1);
    assert_eq!(
        ns.diagnostics.first_warning().message,
        "event 'Hey' has never been emitted"
    );

    // make sure we don't complain about interfaces or abstract contracts
    let case_3 = r#"
    abstract contract F {
        event Hey(uint8 n);
    }
    interface G {
        event Hey(uint8 n);
    }
    "#;

    let ns = parse(case_3);
    assert_eq!(ns.diagnostics.count_warnings(), 0);
}

#[test]
fn constant_variable() {
    let file_2 = r#"
        uint32 constant outside = 2;
    "#;

    let file_1 = r#"
        import "test2.sol";
        contract Testing {
            uint32 test;
            uint32 constant cte = 5;
            constructor() {
                test = outside;
                test = cte;
            }

            function get() public view returns (uint32) {
                return test;
            }
        }
    "#;

    //Constant properly read
    let ns = parse_two_files(file_1, file_2);
    assert_eq!(ns.diagnostics.count_warnings(), 0);

    let file_1 = r#"
        import "test2.sol";
        contract Testing {
            uint32 test;
            uint32 constant cte = 5;
            constructor() {
                test = 45;
            }

            function get() public view returns (uint32) {
                return test;
            }
        }
    "#;

    let ns = parse_two_files(file_1, file_2);
    assert_eq!(ns.diagnostics.count_warnings(), 2);
    assert!(ns
        .diagnostics
        .warning_contains("storage variable 'cte' has been assigned, but never read"));
    assert!(ns
        .diagnostics
        .warning_contains("global constant 'outside' has never been used"));
}

#[test]
fn storage_variable() {
    let file = r#"
        contract Test {
            string str = "This is a test";
            string str2;
            constructor() {
                str = "This is another test";
            }
        }
    "#;

    let ns = parse(file);
    let warnings = ns.diagnostics.warnings();
    assert_eq!(warnings.len(), 2);
    assert_eq!(
        warnings[0].message,
        "storage variable 'str' has been assigned, but never read"
    );
    assert_eq!(
        warnings[1].message,
        "storage variable 'str2' has never been used"
    );

    let file = r#"
        contract Test {
            string str = "This is a test";
            string str2;
            constructor() {
                str = "This is another test";
                str2 = str;
            }
        }
    "#;

    let ns = parse(file);
    assert_eq!(ns.diagnostics.count_warnings(), 1);
    assert_eq!(
        ns.diagnostics.first_warning().message,
        "storage variable 'str2' has been assigned, but never read"
    );

    let file = r#"
        contract Test {
            string str = "This is a test";
            constructor() {
                str = "This is another test";
            }
        }

        contract Test2 is Test {
            function get() public view returns (string) {
                return str;
            }
        }
    "#;

    let ns = parse(file);
    assert_eq!(ns.diagnostics.count_warnings(), 0);
}

#[test]
fn state_variable() {
    let file = r#"
        contract Test {
            function get() public pure {
                uint32 a = 1;
                uint32 b;
                b = 1;
                uint32 c;

                uint32 d;
                d = 1;
                uint32 e;
                e = d*5;
                d = e/5;
            }
        }
    "#;

    let ns = parse(file);
    assert_eq!(ns.diagnostics.count_warnings(), 3);
    assert!(ns
        .diagnostics
        .warning_contains("local variable 'b' has been assigned, but never read"));
    assert!(ns
        .diagnostics
        .warning_contains("local variable 'a' is unused"));
    assert!(ns
        .diagnostics
        .warning_contains("local variable 'c' is unused"));
}

#[test]
fn struct_usage() {
    let file = r#"
        struct testing {
            uint8 it;
            bool tf;
        }

        contract Test {
            testing t1;
            testing t4;
            testing t6;
            constructor() {
                t1 = testing(8, false);
            }

            function modify() public returns (uint8) {
                testing memory t2;
                t2.it = 4;


                t4 = testing(1, true);
                testing storage t3 = t4;
                uint8 k = 2*4/t3.it;
                testing t5;

               return k;
            }
        }
    "#;

    let ns = parse(file);
    assert_eq!(ns.diagnostics.count_warnings(), 4);
    assert!(ns
        .diagnostics
        .warning_contains("local variable 't2' has been assigned, but never read"));
    assert!(ns
        .diagnostics
        .warning_contains("storage variable 't1' has been assigned, but never read"));
    assert!(ns
        .diagnostics
        .warning_contains("local variable 't5' is unused"));
    assert!(ns
        .diagnostics
        .warning_contains("storage variable 't6' has never been used"));
}

#[test]
fn subscript() {
    let file = r#"
        contract Test {
            int[] arr1;
            int[4] arr2;
            int[4] arr3;
            bytes byteArr;

            uint constant e = 1;

            function get() public {
                uint8 a = 1;
                uint8 b = 2;

                arr1[a] = 1;
                arr2[a+b] = 2;

                uint8 c = 1;
                uint8 d = 1;
                int[] memory arr4;
                arr4[0] = 1;
                int[4] storage arr5 = arr3;
                arr5[c*d] = 1;

                byteArr[e] = 0x05;
            }
        }
    "#;

    let ns = parse(file);
    assert_eq!(ns.diagnostics.count_warnings(), 4);
    assert!(ns
        .diagnostics
        .warning_contains("local variable 'arr4' has been assigned, but never read"));
    assert!(ns
        .diagnostics
        .warning_contains("storage variable 'arr1' has been assigned, but never read"));
    assert!(ns
        .diagnostics
        .warning_contains("storage variable 'arr2' has been assigned, but never read"));
    assert!(ns
        .diagnostics
        .warning_contains("storage variable 'byteArr' has been assigned, but never read"));

    let file = r#"
        contract Test {
            int[] arr1;
            int[4] arr2;
            int[4] arr3;
            bytes byteArr;

            uint constant e = 1;

            function get() public {
                uint8 a = 1;
                uint8 b = 2;

                arr1[a] = 1;
                arr2[a+b] = 2;
                assert(arr1[a] == arr2[b]);

                uint8 c = 1;
                uint8 d = 1;
                int[] memory arr4;
                arr4[0] = 1;
                int[4] storage arr5 = arr3;
                arr5[c*d] = 1;
                assert(arr4[c] == arr5[d]);
                assert(arr3[c] == arr5[d]);

                byteArr[e] = 0x05;
                assert(byteArr[e] == byteArr[e]);
            }
        }
    "#;

    let ns = parse(file);
    assert_eq!(ns.diagnostics.count_warnings(), 0);
}

#[test]
fn assign_trunc_cast() {
    //This covers ZeroExt as well
    let file = r#"
    contract Test {
        bytes byteArr;
        bytes32 baRR;

        function get() public  {
            string memory s = "Test";
            byteArr = bytes(s);
            uint16 a = 1;
            uint8 b;
            b = uint8(a);

            uint256 c;
            c = b;
            bytes32 b32;
            bytes memory char = bytes(bytes32(uint(a) * 2 ** (8 * b)));
            baRR = bytes32(c);
            bytes32 cdr = bytes32(char);
            assert(b32 == baRR);
            if(b32 != cdr) {

            }
        }
    }
"#;

    let ns = parse(file);
    assert_eq!(ns.diagnostics.count_warnings(), 1);
    assert!(ns
        .diagnostics
        .warning_contains("storage variable 'byteArr' has been assigned, but never read"));
}

#[test]
fn array_length() {
    let file = r#"
        contract Test {
        int[5] arr1;
        int[] arr2;

        function get() public view returns (bool) {
            int[5] memory arr3;
            int[] memory arr4;

            bool test = false;
            if(arr1.length == arr2.length) {
                test = true;
            }
            else if(arr3.length != arr4.length) {
                test = false;
            }

            return test;
        }
    }
    "#;

    let ns = parse(file);
    assert_eq!(ns.diagnostics.count_warnings(), 0);
}

#[test]
fn sign_ext_storage_load() {
    let file = r#"
        contract Test {
        bytes a;

        function use(bytes memory b) pure public {
            assert(b[0] == b[1]);
        }

        function get() public pure returns (int16 ret) {
            use(a);

            int8 b = 1;
            int16 c = 1;
            int16 d;
            d = c << b;
            ret = d;
        }
    }
    "#;

    let ns = parse(file);
    assert_eq!(ns.diagnostics.count_warnings(), 0);
}

#[test]
fn statements() {
    let file = r#"
    contract AddNumbers { function add(uint256 a, uint256 b) external pure returns (uint256 c) {c = b;} }
    contract Example {
        AddNumbers addContract;
        event StringFailure(string stringFailure);
        event BytesFailure(bytes bytesFailure);

        function exampleFunction(uint256 _a, uint256 _b) public returns (uint256 _c) {

            try addContract.add(_a, _b) returns (uint256 _value) {
                return (_value);
            } catch Error(string memory _err) {
                // This may occur if there is an overflow with the two numbers and the `AddNumbers` contract explicitly fails with a `revert()`
                emit StringFailure(_err);
            } catch (bytes memory _err) {
                emit BytesFailure(_err);
            }
        }

        function testFunction() pure public {
            int three = 3;
             {
                 int test = 2;
                 int c = test*3;

                 while(c != test) {
                     c -= three;
                 }
             }

             int four = 4;
             int test = 3;
             do {
                int ct = 2;
             } while(four > test);

        }

         function bytesToUInt(uint v) public pure returns (uint ret) {
        if (v == 0) {
            ret = 0;
        }
        else {
            while (v > 0) {
                ret = uint(uint(ret) / (2 ** 8));
                ret |= uint(((v % 10) + 48) * 2 ** (8 * 31));
                v /= 10;
            }
        }
        return ret;
    }

        function stringToUint(string s) public pure returns (uint result) {
            bytes memory b = bytes(s);
            uint i;
            result = 0;
            for (i = 0; i < b.length; i++) {
                uint c = uint(b[i]);
                if (c >= 48 && c <= 57) {
                    result = result * 10 + (c - 48);
                }
            }
        }
    }
    "#;
    let ns = parse(file);
    assert_eq!(ns.diagnostics.count_warnings(), 2);
    assert!(ns
        .diagnostics
        .warning_contains("function parameter 'a' is unused"));
    assert!(ns
        .diagnostics
        .warning_contains("local variable 'ct' is unused"));
}

#[test]
fn function_call() {
    let file = r#"
    contract Test1 {
        uint32 public a;

        constructor(uint32 b) {
            a = b;
        }

    }

    contract Test2{
        function test(uint32 v1, uint32 v2) private returns (uint32) {
            uint32 v = 1;
            Test1 t = new Test1(v);
            uint32[2] memory vec = [v2, v1];

            return vec[0] + t.a();
        }

        function callTest() public {
            uint32 ta = 1;
            uint32 tb = 2;

            ta = test(ta, tb);
        }
    }

    contract C {
        uint public data;
        function x() public returns (uint) {
            data = 3;
            return this.data();
        }
    }
    "#;

    let ns = parse(file);
    assert_eq!(ns.diagnostics.count_warnings(), 0);

    let file = r#"
    contract Test1 {
        uint32 public a;

        constructor(uint32 b) {
            a = b;
        }

    }

    contract Test2 is Test1{

        constructor(uint32 val) Test1(val) {}

        function test(uint32 v1, uint32 v2) private returns (uint32) {
            uint32 v = 1;
            Test1 t = new Test1(v);
            uint32[2] memory vec = [v2, v1];

            return vec[0] + t.a();
        }

        function callTest() public {
            uint32 ta = 1;
            uint32 tb = 2;

            ta = test(ta, tb);
        }
    }
    "#;

    let ns = parse(file);
    assert_eq!(ns.diagnostics.count_warnings(), 0);
}

#[test]
fn array_push_pop() {
    let file = r#"
        contract Test1 {
        uint32[] vec1;

        function testVec() public {
            uint32 a = 1;
            uint32 b = 2;
            uint32[] memory vec2;

            vec1.push(a);
            vec2.push(b);
        }
    }
    "#;

    let ns = parse(file);
    assert_eq!(ns.diagnostics.count_warnings(), 2);
    assert!(ns
        .diagnostics
        .warning_contains("local variable 'vec2' has been assigned, but never read"));
    assert!(ns
        .diagnostics
        .warning_contains("storage variable 'vec1' has been assigned, but never read"));

    let file = r#"
        contract Test1 {
        uint32[] vec1;

        function testVec() public {
            uint32 a = 1;
            uint32 b = 2;
            uint32[] memory vec2;

            vec1.push(a);
            vec2.push(b);
            vec1.pop();
            vec2.pop();
        }
    }
    "#;

    let ns = parse(file);
    assert_eq!(ns.diagnostics.count_warnings(), 2);
    assert!(ns
        .diagnostics
        .warning_contains("storage variable 'vec1' has been assigned, but never read"));
    assert!(ns
        .diagnostics
        .warning_contains("local variable 'vec2' has been assigned, but never read"));

    let file = r#"
      contract Test1 {
        function test_storage(uint64[] storage arr1, uint128[] storage arr2) private {
            arr1.push(32);
            arr2.pop();
        }

        function arg_ptr(uint64[] memory arr1, uint16[] memory arr2) private pure {
            arr1.push(422);
            arr2.pop();
        }
    }
    "#;
    let ns = parse(file);
    assert_eq!(ns.diagnostics.count_warnings(), 0);

    let file = r#"
      contract Test1 {
        function test_storage(uint64[] storage arr1) private pure {

        }

        function arg_ptr(uint64[] memory arr2) private pure {
        }
    }
    "#;
    let ns = parse(file);
    assert_eq!(ns.diagnostics.count_warnings(), 2);
    assert!(ns
        .diagnostics
        .warning_contains("function parameter 'arr1' is unused"));
    assert!(ns
        .diagnostics
        .warning_contains("function parameter 'arr2' is unused"));
}

#[test]
fn return_variable() {
    let file = r#"
    contract Test1 {
    string testing;

    function test1() public pure returns (uint32 ret, string memory ret2) {
        return (2, "Testing is fun");
    }

    function test2() public returns (uint32 hey) {

        (uint32 a, string memory t) = test1();
        testing = t;

    }
    }
    "#;

    let ns = parse(file);
    assert_eq!(ns.diagnostics.count_warnings(), 3);
    assert!(ns
        .diagnostics
        .warning_contains("destructure variable 'a' has never been used"));
    assert!(ns
        .diagnostics
        .warning_contains("return variable 'hey' has never been assigned"));
    assert!(ns
        .diagnostics
        .warning_contains("storage variable 'testing' has been assigned, but never read"));
}

#[test]
fn try_catch() {
    let file = r#"
    contract CalledContract {}

    contract TryCatcher {

       event SuccessEvent(bool t);
       event CatchEvent(bool t);

        function execute() public {

            try new CalledContract() returns(CalledContract returnedInstance) {
                emit SuccessEvent(true);
            }  catch Error(string memory revertReason) {
                emit CatchEvent(true);
            } catch (bytes memory returnData) {
                emit CatchEvent(false);
            }
        }
    }
    "#;

    let ns = parse(file);
    assert_eq!(ns.diagnostics.count_warnings(), 3);
    assert!(ns
        .diagnostics
        .warning_contains("try-catch error bytes 'returnData' has never been used"));
    assert!(ns
        .diagnostics
        .warning_contains("try-catch returns variable 'returnedInstance' has never been read"));
    assert!(ns
        .diagnostics
        .warning_contains("try-catch error string 'revertReason' has never been used"));

    let file = r#"
    contract CalledContract {
        bool public ok = true;
        bool private notOk = false;
    }

    contract TryCatcher {

       event SuccessEvent(bool t);
       event CatchEvent(string t);
       event CatchBytes(bytes t);

        function execute() public {

            try new CalledContract() returns(CalledContract returnedInstance) {
                // returnedInstance can be used to obtain the address of the newly deployed contract
                emit SuccessEvent(returnedInstance.ok());
            }  catch Error(string memory revertReason) {
                emit CatchEvent(revertReason);
            } catch (bytes memory returnData) {
                emit CatchBytes(returnData);
            }
        }
      }
    "#;

    let ns = parse(file);
    assert_eq!(ns.diagnostics.count_warnings(), 1);
    assert!(ns
        .diagnostics
        .warning_contains("storage variable 'notOk' has been assigned, but never read"));

    let file = r#"
    contract CalledContract {
        bool public ok;
    }
    "#;

    let ns = parse(file);
    assert_eq!(ns.diagnostics.count_warnings(), 0);
}

#[test]
fn destructure() {
    let file = r#"
       contract Test2{

            function callTest() public view returns (uint32 ret) {
                uint32 ta = 1;
                uint32 tb = 2;
                uint32 te = 3;

                string memory tc = "hey";
                bytes memory td = bytes(tc);
                address nameReg = address(this);
                (bool tf,) = nameReg.call(td);


                ta = tf? tb : te;
                uint8 tg = 1;
                uint8 th = 2;
                (tg, th) = (th, tg);
                return ta;
            }
    }
    "#;

    let ns = parse(file);
    assert_eq!(ns.diagnostics.count_warnings(), 0);
}

#[test]
fn struct_initialization() {
    let file = r#"
        contract Test1{
        struct Test2{
            uint8 a;
            uint8 b;
        }

        function callTest() public pure returns (uint32 ret) {
            uint8 tg = 1;
            uint8 th = 2;

            Test2 memory t2;
            t2 = Test2(tg, th);
            ret = t2.a;
        }
    }
    "#;

    let ns = parse(file);
    assert_eq!(ns.diagnostics.count_warnings(), 0);
}

#[test]
fn subarray_mapping_struct_literal() {
    let file = r#"
        contract T {
        int p;
        constructor(int b) {
            p = b;
        }

        function sum(int a, int b) virtual public returns (int){
            uint8 v1 = 1;
            uint8 v2 = 2;
            uint8 v3 = 3;
            uint8 v4 = 4;
            uint8[2][2] memory v = [[v1, v2], [v3, v4]];
            return a + b * p/v[0][1];
        }
    }

    contract Test is T(2){

        struct fooStruct {
            int foo;
            int figther;
        }
        mapping(string => int) public mp;
        enum FreshJuiceSize{ SMALL, MEDIUM, LARGE }
        FreshJuiceSize choice;

        function sum(int a, int b) override public returns (int) {
            choice = FreshJuiceSize.LARGE;
            return a*b;
        }

        function test() public returns (int){
            int a = 1;
            int b = 2;
            int c = super.sum(a, b);
            int d = 3;
            fooStruct memory myStruct = fooStruct({foo: c, figther: d});
            string memory t = "Do some tests";
            mp[t] = myStruct.figther;
            return mp[t];
        }
    }
    "#;

    let ns = parse(file);
    assert_eq!(ns.diagnostics.count_warnings(), 1);
    assert!(ns
        .diagnostics
        .warning_contains("storage variable 'choice' has been assigned, but never read"));
}

#[test]
fn builtin_call_destructure() {
    let file = r#"
        contract Test {

        function test() public returns(bool p) {
            uint128 b = 1;
            uint64 g = 2;
            address payable ad = payable(address(this));
            bytes memory by = hex"AB2";
            (p, ) = ad.call{value: b, gas: g}(by);
            uint c = 1;
            abi.encodeWithSignature("hey", c);

            uint128 amount = 2;
            ad.send(amount);
            uint128 amount2 = 1;
            ad.transfer(amount2);
        }
    }
    "#;

    let ns = parse(file);
    assert_eq!(ns.diagnostics.count_warnings(), 0);
}

#[test]
fn delete_statement() {
    let file = r#"
    pragma solidity 0;

    contract Test1{
        int test8var;
        function test8() public {
            delete test8var;
        test8var = 2;
        }
    }
    "#;

    let ns = parse(file);
    assert_eq!(ns.diagnostics.count_warnings(), 0);
}

#[test]
fn load_length() {
    let file = r#"
        contract foo {
        function f(uint i1) public pure returns (int) {
            int[8] bar = [ int(10), 20, 30, 4, 5, 6, 7, 8 ];

            bar[2] = 0x7_f;

            return bar[i1];
        }

        function barfunc() public pure returns (uint) {
            uint[2][3][4] array;

            return array.length;
        }
    }
    "#;

    let ns = parse(file);
    assert_eq!(ns.diagnostics.count_warnings(), 0);
}

#[test]
fn address_selector() {
    let file = r#"
    contract ctc {
        function foo(int32 a) public pure returns (bool) {
            return a==1;
        }

        function test() public view {
               function(int32) external returns (bool) func = this.foo;

            assert(address(this) == func.address);
            assert(func.selector == hex"42761137");
        }
    }
    "#;

    let ns = parse(file);
    assert_eq!(ns.diagnostics.count_warnings(), 0);
}

#[test]
fn load_storage_load() {
    let file = r#"
        struct X {
            uint32 f1;
            bool f2;
        }

        contract foo {
            function get() public pure returns (X[4] f) {
                f[1].f1 = 102;
                f[1].f2 = true;
            }
        }
    "#;

    let ns = parse(file);
    assert_eq!(ns.diagnostics.count_warnings(), 0);
}

#[test]
fn variable_function() {
    let file = r#"
    contract ft is Arith {
            function test(bool action, int32 a, int32 b) public returns (int32) {
                function(int32,int32) internal returns (int32) func;

                if (action) {
                    func = Arith.mul;
                } else {
                    func = Arith.add;
                }

                return func(a, b);
            }
        }

        contract Arith {
            function mul(int32 a, int32 b) internal pure returns (int32) {
                return a * b;
            }

            function add(int32 a, int32 b) internal pure returns (int32) {
                return a + b;
            }
        }
    "#;

    let ns = parse(file);
    assert_eq!(ns.diagnostics.count_warnings(), 0);

    let file = r#"
    contract ft {
            function test() public {
                function(int32) external returns (uint64) func = this.foo;

                assert(func(102) == 0xabbaabba);
            }

            function foo(int32) public pure returns (uint64) {
                return 0xabbaabba;
            }
        }
    "#;

    let ns = parse(file);
    assert_eq!(ns.diagnostics.count_warnings(), 0);

    let file = r#"
    contract ft {
            function(int32,int32) internal returns (int32) func;

            function mul(int32 a, int32 b) internal pure returns (int32) {
                return a * b;
            }

            function add(int32 a, int32 b) internal pure returns (int32) {
                return a + b;
            }

            function set_op(bool action) public {
                if (action) {
                    func = mul;
                } else {
                    func = add;
                }
            }

            function test(int32 a, int32 b) public returns (int32) {
                return func(a, b);
            }
        }
    "#;

    let ns = parse(file);
    assert_eq!(ns.diagnostics.count_warnings(), 0);

    let file = r#"
    contract ft {
            function mul(int32 a, int32 b) internal pure returns (int32) {
                return a * b;
            }

            function add(int32 a, int32 b) internal pure returns (int32) {
                return a + b;
            }

            function test(bool action, int32 a, int32 b) public returns (int32) {
                function(int32,int32) internal returns (int32) func;

                if (action) {
                    func = mul;
                } else {
                    func = add;
                }

                return func(a, b);
            }
        }
    "#;

    let ns = parse(file);
    assert_eq!(ns.diagnostics.count_warnings(), 0);

    let file = r#"
    contract ft is Arith {
            function mul(int32 a, int32 b) internal pure override returns (int32) {
                return a * b * 10;
            }

            function add(int32 a, int32 b) internal pure override returns (int32) {
                return a + b + 10;
            }
        }

        contract Arith {
            function test(bool action, int32 a, int32 b) public returns (int32) {
                function(int32,int32) internal returns (int32) func;

                if (action) {
                    func = mul;
                } else {
                    func = add;
                }

                return func(a, b);
            }

            function mul(int32 a, int32 b) internal virtual returns (int32) {
                return a * b;
            }

            function add(int32 a, int32 b) internal virtual returns (int32) {
                return a + b;
            }
        }
    "#;
    let ns = parse(file);
    assert_eq!(ns.diagnostics.count_warnings(), 0);

    let file = r#"
    function global_function() pure returns (uint32) {
            return 102;
        }

        function global_function2() pure returns (uint32) {
            return global_function() + 5;
        }

        contract c {
            function test() public {
                function() internal returns (uint32) ftype = global_function2;

                uint64 x = ftype();

                assert(x == 107);
            }
        }
    "#;
    let ns = parse(file);
    assert_eq!(ns.diagnostics.count_warnings(), 0);
}

#[test]
fn format_string() {
    let file = r#"
    contract foo {
            constructor() {
                int x = 21847450052839212624230656502990235142567050104912751880812823948662932355201;

                print("x = {}".format(x));
            }
        }
    "#;

    let ns = parse(file);
    assert_eq!(ns.diagnostics.count_warnings(), 0);
}

#[test]
fn balance() {
    let file = r#"
    contract foo {


    function test(address payable addr) public pure returns (bool) {
        bool p;
        p = addr.balance == 2;
        return p;
    }

    }
    "#;
    let ns = parse(file);
    assert_eq!(ns.diagnostics.count_warnings(), 0);
}
