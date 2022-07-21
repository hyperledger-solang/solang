use crate::build_solidity;
use borsh::BorshDeserialize;
use ethabi::Token;

#[test]
fn integers_bool_enum() {
    #[derive(BorshDeserialize, PartialEq, Debug)]
    enum WeekDay {
        Sunday,
        Monday,
        Tuesday,
        Wednesday,
        Thursday,
        Friday,
        Saturday,
    }

    #[derive(BorshDeserialize, Debug)]
    struct Res1 {
        a: u8,
        b: u64,
        c: u128,
        d: i16,
        e: i32,
        day: WeekDay,
        h: bool,
    }

    #[derive(BorshDeserialize, Debug)]
    struct Res2 {
        sunday: WeekDay,
        elem: WeekDay,
        vec_2: WeekDay,
    }

    let mut vm = build_solidity(
        r#"
contract Testing {
    enum weekday{
        sunday, monday, tuesday, wednesday, thursday, friday, saturday
    }

    function getThis() public pure returns (bytes memory) {
        uint8 a = 45;
        uint64 b = 9965956609890;
        uint128 c = 88;

        int16 d = -29;
        int32 e = -88;

        weekday f = weekday.wednesday;
        bool h = false;
        bytes memory g = abi.encode(a, b, c, d, e, f, h);
        return g;
    }

    function encodeEnum() public pure returns (bytes memory) {
        weekday[3] memory vec = [weekday.sunday, weekday.tuesday, weekday.friday];
        weekday elem = weekday.saturday;
        bytes memory b = abi.encode(weekday.sunday, elem, vec[2]);
        return b;
    }
}

        "#,
    );

    vm.constructor("Testing", &[]);
    let returns = vm.function("getThis", &[], &[], None);
    let encoded = returns[0].clone().into_bytes().unwrap();
    let decoded = Res1::try_from_slice(&encoded).unwrap();

    assert_eq!(decoded.a, 45);
    assert_eq!(decoded.b, 9965956609890);
    assert_eq!(decoded.c, 88);
    assert_eq!(decoded.d, -29);
    assert_eq!(decoded.e, -88);
    assert_eq!(decoded.day, WeekDay::Wednesday);
    assert!(!decoded.h);

    let returns = vm.function("encodeEnum", &[], &[], None);
    let encoded = returns[0].clone().into_bytes().unwrap();
    let decoded = Res2::try_from_slice(&encoded).unwrap();

    assert_eq!(decoded.sunday, WeekDay::Sunday);
    assert_eq!(decoded.elem, WeekDay::Saturday);
    assert_eq!(decoded.vec_2, WeekDay::Friday);
}

#[test]
fn encode_address() {
    #[derive(BorshDeserialize, Debug)]
    struct Response {
        address: [u8; 32],
        this: [u8; 32],
    }

    let mut vm = build_solidity(
        r#"
contract Testing {

    function getThis() public view returns (bytes memory) {
        bytes memory b = abi.encode(address(this), this);
        return b;
    }
}
        "#,
    );
    vm.constructor("Testing", &[]);
    let returns = vm.function("getThis", &[], &[], None);
    let encoded = returns[0].clone().into_bytes().unwrap();
    let decoded = Response::try_from_slice(&encoded).unwrap();
    assert_eq!(decoded.address, vm.programs[0].data);
    assert_eq!(decoded.this, vm.programs[0].data);
}

#[test]
fn string_and_bytes() {
    #[derive(BorshDeserialize, Debug)]
    struct MyStruct {
        a: String,
        b: Vec<u8>,
    }

    let mut vm = build_solidity(
        r#"
contract Testing {

    function getThis() public pure returns (bytes memory) {
        string memory a = "coffe";
        bytes memory b = "tea";
        bytes memory c = abi.encode(a, b);
        return c;
    }
}
      "#,
    );

    vm.constructor("Testing", &[]);
    let returns = vm.function("getThis", &[], &[], None);
    let encoded = returns[0].clone().into_bytes().unwrap();
    let decoded = MyStruct::try_from_slice(&encoded).unwrap();
    assert_eq!(decoded.a, "coffe");
    assert_eq!(decoded.b, b"tea");
}

#[test]
fn primitive_structs() {
    #[derive(Debug, BorshDeserialize)]
    struct NoPadStruct {
        a: u32,
        b: u32,
    }

    #[derive(Debug, BorshDeserialize)]
    struct PaddedStruct {
        a: u128,
        b: u8,
        c: [u8; 32],
    }

    let mut vm = build_solidity(
        r#"
        contract Testing {

    struct noPadStruct {
        uint32 a;
        uint32 b;
    }

    struct PaddedStruct {
        uint128 a;
        uint8 b;
        bytes32 c;
    }

    function getThis() public pure returns (bytes memory) {
        noPadStruct memory a = noPadStruct(1238, 87123);
        bytes memory b = abi.encode(a);
        return b;
    }

    function getThat() public pure returns (bytes memory) {
        PaddedStruct memory a = PaddedStruct(12998, 240, "tea_is_good");
        bytes memory b = abi.encode(a);
        return b;
    }
}
        "#,
    );
    vm.constructor("Testing", &[]);
    let returns = vm.function("getThis", &[], &[], None);
    let encoded = returns[0].clone().into_bytes().unwrap();
    let decoded = NoPadStruct::try_from_slice(&encoded).unwrap();
    assert_eq!(decoded.a, 1238);
    assert_eq!(decoded.b, 87123);

    let returns = vm.function("getThat", &[], &[], None);
    let encoded = returns[0].clone().into_bytes().unwrap();
    let decoded = PaddedStruct::try_from_slice(&encoded).unwrap();
    assert_eq!(decoded.a, 12998);
    assert_eq!(decoded.b, 240);
    let mut b: [u8; 11] = b"tea_is_good".to_owned();
    b.reverse();
    assert_eq!(&decoded.c[21..32], b);
}

#[test]
fn argument_string() {
    #[derive(Debug, BorshDeserialize)]
    struct Response {
        rr: String,
    }

    let mut vm = build_solidity(
        r#"
contract Testing {

    function testStruct(string memory rr) public pure returns (bytes memory) {
        bytes memory b1 = abi.encode(rr);
        return b1;
    }
}
      "#,
    );

    vm.constructor("Testing", &[]);
    let returns = vm.function(
        "testStruct",
        &[Token::String("nihao".to_string())],
        &[],
        None,
    );
    let encoded = returns[0].clone().into_bytes().unwrap();
    let decoded = Response::try_from_slice(&encoded).unwrap();
    assert_eq!(decoded.rr, "nihao");
}

#[test]
fn test_string_array() {
    #[derive(Debug, BorshDeserialize)]
    struct Response {
        a: Vec<String>,
    }

    let mut vm = build_solidity(
        r#"
        contract Testing {
            string[] string_vec;
            function encode() public view returns (bytes memory) {
                string[] memory mem_vec = string_vec;
                bytes memory b = abi.encode(mem_vec);
                return b;
            }

            function insertStrings() public {
                string_vec.push("tea");
                string_vec.push("coffee");
            }
        }
        "#,
    );

    vm.constructor("Testing", &[]);
    let returns = vm.function("encode", &[], &[], None);
    let encoded = returns[0].clone().into_bytes().unwrap();
    let decoded = Response::try_from_slice(&encoded).unwrap();
    assert_eq!(decoded.a.len(), 0);

    let _ = vm.function("insertStrings", &[], &[], None);
    let returns = vm.function("encode", &[], &[], None);
    let encoded = returns[0].clone().into_bytes().unwrap();
    let decoded = Response::try_from_slice(&encoded).unwrap();
    assert_eq!(decoded.a.len(), 2);
    assert_eq!(decoded.a[0], "tea");
    assert_eq!(decoded.a[1], "coffee");
}

#[test]
fn struct_within_struct() {
    #[derive(Debug, BorshDeserialize)]
    struct NoPadStruct {
        a: u32,
        b: u32,
    }

    #[derive(Debug, BorshDeserialize)]
    struct PaddedStruct {
        a: u128,
        b: u8,
        c: [u8; 32],
    }

    #[derive(Debug, BorshDeserialize)]
    struct NonConstantStruct {
        a: u64,
        b: Vec<String>,
        no_pad: NoPadStruct,
        pad: PaddedStruct,
    }

    let mut vm = build_solidity(
        r#"
contract Testing {

  struct noPadStruct {
        uint32 a;
        uint32 b;
    }

    struct PaddedStruct {
        uint128 a;
        uint8 b;
        bytes32 c;
    }

    struct NonConstantStruct {
        uint64 a;
        string[] b;
        noPadStruct noPad;
        PaddedStruct pad;
    }

    string[] string_vec;
    NonConstantStruct to_encode;
    function testStruct() public returns (bytes memory) {
        noPadStruct memory noPad = noPadStruct(89123, 12354);
        PaddedStruct memory padded = PaddedStruct(988834, 129, "tea_is_good");
        string_vec.push("tea");
        string_vec.push("coffee");

        to_encode = NonConstantStruct(890234, string_vec, noPad, padded);

        bytes memory b1 = abi.encode(to_encode);
        return b1;
    }
}
        "#,
    );

    vm.constructor("Testing", &[]);
    let returns = vm.function("testStruct", &[], &[], None);
    let encoded = returns[0].clone().into_bytes().unwrap();
    let decoded = NonConstantStruct::try_from_slice(&encoded).unwrap();

    assert_eq!(decoded.a, 890234);
    assert_eq!(decoded.b.len(), 2);
    assert_eq!(decoded.b[0], "tea");
    assert_eq!(decoded.b[1], "coffee");
    assert_eq!(decoded.no_pad.a, 89123);
    assert_eq!(decoded.no_pad.b, 12354);
    assert_eq!(decoded.pad.a, 988834);
    assert_eq!(decoded.pad.b, 129);
    let mut b: [u8; 11] = b"tea_is_good".to_owned();
    b.reverse();
    assert_eq!(&decoded.pad.c[21..32], b);
}

#[test]
fn struct_in_array() {
    #[derive(Debug, BorshDeserialize, PartialEq, Copy, Default, Clone)]
    struct NoPadStruct {
        a: u32,
        b: u32,
    }

    #[derive(Debug, BorshDeserialize)]
    struct PaddedStruct {
        a: u128,
        b: u8,
        c: [u8; 32],
    }

    #[derive(Debug, BorshDeserialize)]
    struct Res1 {
        item_1: NoPadStruct,
        item_2: PaddedStruct,
    }

    #[derive(Debug, BorshDeserialize)]
    struct Res2 {
        item_1: Vec<NoPadStruct>,
        item_2: [i32; 4],
        item_3: [NoPadStruct; 2],
    }

    let mut vm = build_solidity(
        r#"
    contract Testing {

        struct noPadStruct {
            uint32 a;
            uint32 b;
        }

        struct PaddedStruct {
            uint128 a;
            uint8 b;
            bytes32 c;
        }

        noPadStruct[] test_vec_1;

        function addData() public  {
            noPadStruct memory mm = noPadStruct(1623, 43279);
            test_vec_1.push(mm);
            mm.a = 41234;
            mm.b = 98375;
            test_vec_1.push(mm);
            mm.a = 945;
            mm.b = 7453;
            test_vec_1.push(mm);
        }


        function encodeStruct() public view returns (bytes memory) {
            PaddedStruct memory ss = PaddedStruct(1, 3, "there_is_padding_here");
            bytes memory b = abi.encode(test_vec_1[2], ss);
            return b;
        }

        function primitiveStruct() public view returns (bytes memory) {
            int32[4] memory mem_vec = [int32(1), -298, 3, -434];
            noPadStruct[2] memory str_vec = [noPadStruct(1,2), noPadStruct(3, 4)];
            bytes memory b1 = abi.encode(test_vec_1, mem_vec, str_vec);
            return b1;
        }
    }
        "#,
    );

    vm.constructor("Testing", &[]);
    let _ = vm.function("addData", &[], &[], None);
    let returns = vm.function("encodeStruct", &[], &[], None);
    let encoded = returns[0].clone().into_bytes().unwrap();
    let decoded = Res1::try_from_slice(&encoded).unwrap();

    assert_eq!(decoded.item_1.a, 945);
    assert_eq!(decoded.item_1.b, 7453);
    assert_eq!(decoded.item_2.a, 1);
    assert_eq!(decoded.item_2.b, 3);
    let mut b: [u8; 21] = b"there_is_padding_here".to_owned();
    b.reverse();
    assert_eq!(&decoded.item_2.c[11..32], b);

    let returns = vm.function("primitiveStruct", &[], &[], None);
    let encoded = returns[0].clone().into_bytes().unwrap();
    let decoded = Res2::try_from_slice(&encoded).unwrap();

    assert_eq!(decoded.item_1.len(), 3);
    assert_eq!(decoded.item_1[0], NoPadStruct { a: 1623, b: 43279 });
    assert_eq!(decoded.item_1[1], NoPadStruct { a: 41234, b: 98375 });
    assert_eq!(decoded.item_1[2], NoPadStruct { a: 945, b: 7453 });
    assert_eq!(decoded.item_2, [1, -298, 3, -434]);
    assert_eq!(decoded.item_3[0], NoPadStruct { a: 1, b: 2 });
    assert_eq!(decoded.item_3[1], NoPadStruct { a: 3, b: 4 });
}

#[test]
fn arrays() {
    #[derive(Debug, BorshDeserialize)]
    struct Res1 {
        vec_1: Vec<i16>,
    }

    #[derive(Debug, BorshDeserialize, Default, Clone)]
    struct NonConstantStruct {
        a: u64,
        b: Vec<String>,
    }

    #[derive(Debug, BorshDeserialize)]
    struct Res2 {
        complex_array: Vec<NonConstantStruct>,
    }

    #[derive(Debug, BorshDeserialize)]
    struct Res3 {
        multi_dim: [[i8; 2]; 3],
    }

    let mut vm = build_solidity(
        r#"
    contract Testing {
        int16[] vec_1;
        function addData() public {
            vec_1.push(-90);
            vec_1.push(5523);
            vec_1.push(-89);
        }

        struct NonConstantStruct {
            uint64 a;
            string[] b;
        }

        function encodeComplex() public returns (bytes memory) {
            string[] vec_2 = new string[](2);
            vec_2[0] = "tea";
            vec_2[1] = "coffee";
            NonConstantStruct[] arr = new NonConstantStruct[](2);
            arr[0] = NonConstantStruct(897, vec_2);

            string[] vec_3 = new string[](2);
            vec_3[0] = "cortado";
            vec_3[1] = "cappuccino";
            arr[1] = NonConstantStruct(74123, vec_3);
            return abi.encode(arr);
        }

        function encodeArray() public view returns (bytes memory) {
            bytes memory b = abi.encode(vec_1);
            return b;
        }

        function multiDimArrays() public pure returns (bytes memory) {
            int8[2][3] memory vec = [[int8(1), 2], [int8(4), 5], [int8(6), 7]];
            bytes memory b = abi.encode(vec);
            return b;
        }
    }
      "#,
    );

    vm.constructor("Testing", &[]);
    let _ = vm.function("addData", &[], &[], None);
    let returns = vm.function("encodeArray", &[], &[], None);
    let encoded = returns[0].clone().into_bytes().unwrap();
    let decoded = Res1::try_from_slice(&encoded).unwrap();

    assert_eq!(decoded.vec_1.len(), 3);
    assert_eq!(decoded.vec_1[0], -90);
    assert_eq!(decoded.vec_1[1], 5523);
    assert_eq!(decoded.vec_1[2], -89);

    let returns = vm.function("encodeComplex", &[], &[], None);
    let encoded = returns[0].clone().into_bytes().unwrap();
    let decoded = Res2::try_from_slice(&encoded).unwrap();

    assert_eq!(decoded.complex_array.len(), 2);
    assert_eq!(decoded.complex_array[0].a, 897);
    assert_eq!(
        decoded.complex_array[0].b,
        vec!["tea".to_string(), "coffee".to_string()]
    );
    assert_eq!(decoded.complex_array[1].a, 74123);
    assert_eq!(
        decoded.complex_array[1].b,
        vec!["cortado".to_string(), "cappuccino".to_string()]
    );

    let returns = vm.function("multiDimArrays", &[], &[], None);
    let encoded = returns[0].clone().into_bytes().unwrap();
    let decoded = Res3::try_from_slice(&encoded).unwrap();

    assert_eq!(decoded.multi_dim[0], [1, 2]);
    assert_eq!(decoded.multi_dim[1], [4, 5]);
    assert_eq!(decoded.multi_dim[2], [6, 7]);
}

#[test]
fn multi_dimensional_array() {
    #[derive(Debug, BorshDeserialize, Default, Copy, Clone, PartialEq)]
    struct PaddedStruct {
        a: u128,
        b: u8,
        c: [u8; 32],
    }

    #[derive(Debug, BorshDeserialize)]
    struct Res1 {
        item_1: Vec<[[PaddedStruct; 2]; 3]>,
        item_2: u16,
    }

    #[derive(Debug, BorshDeserialize)]
    struct Res2 {
        item: Vec<[[u16; 4]; 2]>,
    }

    #[derive(Debug, BorshDeserialize)]
    struct Res3 {
        item: Vec<u16>,
    }

    let mut vm = build_solidity(
        r#"
contract Testing {

    struct PaddedStruct {
        uint128 a;
        uint8 b;
        bytes32 c;
    }

    function getThis() public pure returns (bytes memory) {
        PaddedStruct memory a = PaddedStruct(56, 1, "oi");
        PaddedStruct memory b = PaddedStruct(78, 6, "bc");
        PaddedStruct memory c = PaddedStruct(89, 4, "sn");
        PaddedStruct memory d = PaddedStruct(42, 56, "cn");
        PaddedStruct memory e = PaddedStruct(23, 78, "fr");
        PaddedStruct memory f = PaddedStruct(445, 46, "br");

        PaddedStruct[2][3] memory vec = [[a, b], [c, d], [e, f]];

        PaddedStruct[2][3][] memory arr2 = new PaddedStruct[2][3][](1);
        arr2[0] = vec;

        uint16 g = 5;
        bytes memory b1 = abi.encode(arr2, g);
        return b1;
    }

    function multiDim() public pure returns (bytes memory) {
        uint16[4][2] memory vec = [[uint16(1), 2, 3, 4], [uint16(5), 6, 7, 8]];

        uint16[4][2][] memory simple_arr = new uint16[4][2][](1);
        simple_arr[0] = vec;

        bytes memory b = abi.encode(simple_arr);
        return b;
    }

    function uniqueDim() public pure returns (bytes memory) {
        uint16[] memory vec = new uint16[](5);
        vec[0] = 9;
        vec[1] = 3;
        vec[2] = 4;
        vec[3] = 90;
        vec[4] = 834;
        bytes memory b = abi.encode(vec);
        return b;
    }
}
        "#,
    );

    vm.constructor("Testing", &[]);
    let returns = vm.function("getThis", &[], &[], None);
    let encoded = returns[0].clone().into_bytes().unwrap();
    let decoded = Res1::try_from_slice(&encoded).unwrap();

    assert_eq!(decoded.item_1.len(), 1);
    let mut res1_c: Vec<u8> = Vec::new();
    res1_c.resize(32, 0);

    assert_eq!(
        decoded.item_1[0][0][0],
        PaddedStruct {
            a: 56,
            b: 1,
            c: create_response(&mut res1_c, b"oi")
        }
    );
    assert_eq!(
        decoded.item_1[0][0][1],
        PaddedStruct {
            a: 78,
            b: 6,
            c: create_response(&mut res1_c, b"bc")
        }
    );
    assert_eq!(
        decoded.item_1[0][1][0],
        PaddedStruct {
            a: 89,
            b: 4,
            c: create_response(&mut res1_c, b"sn")
        }
    );
    assert_eq!(
        decoded.item_1[0][1][1],
        PaddedStruct {
            a: 42,
            b: 56,
            c: create_response(&mut res1_c, b"cn")
        }
    );
    assert_eq!(
        decoded.item_1[0][2][0],
        PaddedStruct {
            a: 23,
            b: 78,
            c: create_response(&mut res1_c, b"fr")
        }
    );
    assert_eq!(
        decoded.item_1[0][2][1],
        PaddedStruct {
            a: 445,
            b: 46,
            c: create_response(&mut res1_c, b"br")
        }
    );
    assert_eq!(decoded.item_2, 5);

    let returns = vm.function("multiDim", &[], &[], None);
    let encoded = returns[0].clone().into_bytes().unwrap();
    let decoded = Res2::try_from_slice(&encoded).unwrap();

    assert_eq!(decoded.item.len(), 1);
    assert_eq!(decoded.item[0][0], [1, 2, 3, 4]);
    assert_eq!(decoded.item[0][1], [5, 6, 7, 8]);

    let returns = vm.function("uniqueDim", &[], &[], None);
    let encoded = returns[0].clone().into_bytes().unwrap();
    let decoded = Res3::try_from_slice(&encoded).unwrap();

    assert_eq!(decoded.item.len(), 5);
    assert_eq!(decoded.item, vec![9, 3, 4, 90, 834]);
}

fn create_response(vec: &mut [u8], string: &[u8; 2]) -> [u8; 32] {
    vec[30] = string[1];
    vec[31] = string[0];
    <[u8; 32]>::try_from(vec.to_owned()).unwrap()
}

#[test]
fn null_pointer() {
    #[derive(Debug, BorshDeserialize)]
    struct S {
        f1: i64,
        f2: String,
    }

    #[derive(Debug, BorshDeserialize)]
    struct Res1 {
        item: Vec<S>,
    }

    #[derive(Debug, BorshDeserialize)]
    struct Res2 {
        item: Vec<String>,
    }

    let mut vm = build_solidity(
        r#"
    contract Testing {

        struct S {
            int64 f1;
            string f2;
        }

        function test1() public pure returns (bytes memory) {
            S[] memory s = new S[](5);
            return abi.encode(s);
        }

        function test2() public pure returns (bytes memory) {
            string[] memory x = new string[](5);
            return abi.encode(x);
        }
    }
        "#,
    );

    vm.constructor("Testing", &[]);
    let returns = vm.function("test1", &[], &[], None);
    let encoded = returns[0].clone().into_bytes().unwrap();
    let decoded = Res1::try_from_slice(&encoded).unwrap();

    assert_eq!(decoded.item.len(), 5);
    for i in 0..5 {
        assert_eq!(decoded.item[i].f1, 0);
        assert!(decoded.item[i].f2.is_empty())
    }

    let returns = vm.function("test2", &[], &[], None);
    let encoded = returns[0].clone().into_bytes().unwrap();
    let decoded = Res2::try_from_slice(&encoded).unwrap();

    assert_eq!(decoded.item.len(), 5);

    for i in 0..5 {
        assert!(decoded.item[i].is_empty());
    }
}

#[test]
fn external_function() {
    #[derive(Debug, BorshDeserialize)]
    struct Res {
        item_1: [u8; 4],
        item_2: [u8; 32],
    }

    let mut vm = build_solidity(
        r#"
    contract Testing {
        function doThis(int64 a, int64 b) public pure returns (int64) {
            return a+b;
        }

        function doThat() public view returns (bytes4, address, bytes memory) {
            function (int64, int64) external returns (int64) fPtr = this.doThis;

            bytes memory b = abi.encode(fPtr);
            return (fPtr.selector, fPtr.address, b);
        }
    }
        "#,
    );

    vm.constructor("Testing", &[]);

    let returns = vm.function("doThat", &[], &[], None);
    let encoded = returns[2].clone().into_bytes().unwrap();
    let decoded = Res::try_from_slice(&encoded).unwrap();

    let mut selector = returns[0].clone().into_fixed_bytes().unwrap();
    selector.reverse();
    let address = returns[1].clone().into_fixed_bytes().unwrap();

    assert_eq!(decoded.item_1, &selector[..]);
    assert_eq!(decoded.item_2, &address[..]);
}
