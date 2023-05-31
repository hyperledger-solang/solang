contract foo {
    struct test_stru {
        uint256 a;
        uint256 b;
    }

    test_stru storage_struct;

    function bar(int256[] calldata vl) public view {
        test_stru storage tts = storage_struct;
        assembly {
            // 'a' contains vl memory address
            let a := vl.offset

            // 'b' contains vl length
            let b := vl.length

            // This will change the reference of vl
            vl.offset := 5
        }
        // Any usage of vl here may crash the program
    }
}
