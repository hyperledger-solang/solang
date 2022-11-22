contract foo {
    struct test_stru {
        uint256 a;
        uint256 b;
    }

    function bar(uint64 a) public pure returns (uint64 ret) {
        uint64 b = 6;
        uint64[] memory vec;
        vec.push(4);
        string str = "cafe";
        test_stru tts = test_stru({a: 1, b: 2});
        assembly {
            // The following statements modify variables directly
            a := add(a, 3)
            b := mul(b, 2)
            ret := sub(a, b)

            // The following modify the reference address
            str := 5
            vec := 6
            tts := 7
        }

        // Any access to 'str', 'vec' or 'tts' here may crash the program.
    }
}
