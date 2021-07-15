use itertools::Itertools;
use solang::file_cache::FileCache;
use solang::sema::ast;
use solang::sema::ast::{Diagnostic, Level};
use solang::{parse_and_resolve, Target};

fn generic_target_parse(src: &'static str) -> ast::Namespace {
    let mut cache = FileCache::new();
    cache.set_file_contents("test.sol", src.to_string());

    parse_and_resolve("test.sol", &mut cache, Target::Generic)
}

fn generic_parse_two_files(src1: &'static str, src2: &'static str) -> ast::Namespace {
    let mut cache = FileCache::new();
    cache.set_file_contents("test.sol", src1.to_string());
    cache.set_file_contents("test2.sol", src2.to_string());

    parse_and_resolve("test.sol", &mut cache, Target::Generic)
}

fn count_warnings(diagnostics: &[Diagnostic]) -> usize {
    diagnostics
        .iter()
        .filter(|&x| x.level == Level::Warning)
        .count()
}

fn get_first_warning(diagnostics: &[Diagnostic]) -> &Diagnostic {
    diagnostics
        .iter()
        .find_or_first(|&x| x.level == Level::Warning)
        .unwrap()
}

fn get_warnings(diagnostics: &[Diagnostic]) -> Vec<&Diagnostic> {
    let mut res = Vec::new();
    for elem in diagnostics {
        if elem.level == Level::Warning {
            res.push(elem);
        }
    }

    res
}

fn assert_message_in_warnings(diagnostics: &[Diagnostic], message: &str) -> bool {
    let warnings = get_warnings(diagnostics);
    for warning in warnings {
        if warning.message == message {
            return true;
        }
    }

    false
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

    let ns = generic_target_parse(case_1);
    assert_eq!(count_warnings(&ns.diagnostics), 0);

    // Unused event
    let case_2 = r#"
    contract usedEvent {
        event Hey(uint8 n);
        event Hello(uint8 n);
        function emitEvent(uint8 n) public {
            emit Hey(n);
        }
    }
    "#;

    let ns = generic_target_parse(case_2);
    assert_eq!(count_warnings(&ns.diagnostics), 1);
    assert_eq!(
        get_first_warning(&ns.diagnostics).message,
        "event 'Hello' has never been emitted"
    );
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
    let ns = generic_parse_two_files(file_1, file_2);
    assert_eq!(count_warnings(&ns.diagnostics), 0);

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

    let ns = generic_parse_two_files(file_1, file_2);
    assert_eq!(count_warnings(&ns.diagnostics), 2);
    assert!(assert_message_in_warnings(
        &ns.diagnostics,
        "storage variable 'cte' has been assigned, but never read"
    ));
    assert!(assert_message_in_warnings(
        &ns.diagnostics,
        "global constant 'outside' has never been used"
    ));
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

    let ns = generic_target_parse(file);
    let warnings = get_warnings(&ns.diagnostics);
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

    let ns = generic_target_parse(file);
    assert_eq!(count_warnings(&ns.diagnostics), 1);
    assert_eq!(
        get_first_warning(&ns.diagnostics).message,
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

    let ns = generic_target_parse(file);
    assert_eq!(count_warnings(&ns.diagnostics), 0);
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

    let ns = generic_target_parse(file);
    assert_eq!(count_warnings(&ns.diagnostics), 3);
    assert!(assert_message_in_warnings(
        &ns.diagnostics,
        "local variable 'b' has been assigned, but never read"
    ));
    assert!(assert_message_in_warnings(
        &ns.diagnostics,
        "local variable 'a' has been assigned, but never read"
    ));
    assert!(assert_message_in_warnings(
        &ns.diagnostics,
        "local variable 'c' has never been read nor assigned"
    ));
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

    let ns = generic_target_parse(file);
    assert_eq!(count_warnings(&ns.diagnostics), 4);
    assert!(assert_message_in_warnings(
        &ns.diagnostics,
        "local variable 't2' has been assigned, but never read"
    ));
    assert!(assert_message_in_warnings(
        &ns.diagnostics,
        "storage variable 't1' has been assigned, but never read"
    ));
    assert!(assert_message_in_warnings(
        &ns.diagnostics,
        "local variable 't5' has never been read nor assigned"
    ));
    assert!(assert_message_in_warnings(
        &ns.diagnostics,
        "storage variable 't6' has never been used"
    ));
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

    let ns = generic_target_parse(file);
    assert_eq!(count_warnings(&ns.diagnostics), 5);
    assert!(assert_message_in_warnings(
        &ns.diagnostics,
        "local variable 'arr4' has been assigned, but never read"
    ));
    assert!(assert_message_in_warnings(
        &ns.diagnostics,
        "local variable 'arr5' has been assigned, but never read"
    ));
    assert!(assert_message_in_warnings(
        &ns.diagnostics,
        "storage variable 'arr1' has been assigned, but never read"
    ));
    assert!(assert_message_in_warnings(
        &ns.diagnostics,
        "storage variable 'arr2' has been assigned, but never read"
    ));
    assert!(assert_message_in_warnings(
        &ns.diagnostics,
        "storage variable 'byteArr' has been assigned, but never read"
    ));

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

    let ns = generic_target_parse(file);
    assert_eq!(count_warnings(&ns.diagnostics), 0);
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

    let ns = generic_target_parse(file);
    assert_eq!(count_warnings(&ns.diagnostics), 2);
    assert!(assert_message_in_warnings(
        &ns.diagnostics,
        "local variable 'b32' has never been assigned a value, but has been read"
    ));
    assert!(assert_message_in_warnings(
        &ns.diagnostics,
        "storage variable 'byteArr' has been assigned, but never read"
    ));
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

    let ns = generic_target_parse(file);
    assert_eq!(count_warnings(&ns.diagnostics), 0);
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

    let ns = generic_target_parse(file);
    assert_eq!(count_warnings(&ns.diagnostics), 0);
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
    let ns = generic_target_parse(file);
    assert_eq!(count_warnings(&ns.diagnostics), 2);
    assert!(assert_message_in_warnings(
        &ns.diagnostics,
        "function parameter 'a' has never been read"
    ));
    assert!(assert_message_in_warnings(
        &ns.diagnostics,
        "local variable 'ct' has been assigned, but never read",
    ));
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

    let ns = generic_target_parse(file);
    assert_eq!(count_warnings(&ns.diagnostics), 0);

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

    let ns = generic_target_parse(file);
    assert_eq!(count_warnings(&ns.diagnostics), 0);
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

    let ns = generic_target_parse(file);
    assert_eq!(count_warnings(&ns.diagnostics), 2);
    assert!(assert_message_in_warnings(
        &ns.diagnostics,
        "local variable 'vec2' has been assigned, but never read"
    ));
    assert!(assert_message_in_warnings(
        &ns.diagnostics,
        "storage variable 'vec1' has been assigned, but never read"
    ));

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

    let ns = generic_target_parse(file);
    assert_eq!(count_warnings(&ns.diagnostics), 0);
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

    let ns = generic_target_parse(file);
    assert_eq!(count_warnings(&ns.diagnostics), 3);
    assert!(assert_message_in_warnings(
        &ns.diagnostics,
        "destructure variable 'a' has never been used"
    ));
    assert!(assert_message_in_warnings(
        &ns.diagnostics,
        "return variable 'hey' has never been assigned"
    ));
    assert!(assert_message_in_warnings(
        &ns.diagnostics,
        "storage variable 'testing' has been assigned, but never read"
    ));
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

    let ns = generic_target_parse(file);
    assert_eq!(count_warnings(&ns.diagnostics), 3);
    assert!(assert_message_in_warnings(
        &ns.diagnostics,
        "try-catch error bytes 'returnData' has never been used"
    ));
    assert!(assert_message_in_warnings(
        &ns.diagnostics,
        "try-catch returns variable 'returnedInstance' has never been read"
    ));
    assert!(assert_message_in_warnings(
        &ns.diagnostics,
        "try-catch error string 'revertReason' has never been used"
    ));

    let file = r#"
    contract CalledContract {
        bool public ok = true;
        bool public notOk = false;
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

    let ns = generic_target_parse(file);
    assert_eq!(count_warnings(&ns.diagnostics), 1);
    assert!(assert_message_in_warnings(
        &ns.diagnostics,
        "storage variable 'notOk' has been assigned, but never read"
    ));
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

    let ns = generic_target_parse(file);
    assert_eq!(count_warnings(&ns.diagnostics), 0);
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

    let ns = generic_target_parse(file);
    assert_eq!(count_warnings(&ns.diagnostics), 0);
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

    let ns = generic_target_parse(file);
    assert_eq!(count_warnings(&ns.diagnostics), 1);
    assert!(assert_message_in_warnings(
        &ns.diagnostics,
        "storage variable 'choice' has been assigned, but never read"
    ));
}

#[test]
fn builtin_call_destructure() {
    let file = r#"
        contract Test {

        function test() public returns(bool p) {
            uint128 b = 1;
            uint64 g = 2;
            address payable ad = payable(address(this));
            bytes memory by;
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

    let ns = generic_target_parse(file);
    assert_eq!(count_warnings(&ns.diagnostics), 1);
    assert!(assert_message_in_warnings(
        &ns.diagnostics,
        "local variable 'by' has never been assigned a value, but has been read"
    ));
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

    let ns = generic_target_parse(file);
    assert_eq!(count_warnings(&ns.diagnostics), 0);
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

    let ns = generic_target_parse(file);
    assert_eq!(count_warnings(&ns.diagnostics), 0);
}
