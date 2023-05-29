contract foo {
    struct test_stru {
        uint256 a;
        uint256 b;
    }

    test_stru storage_struct;

    function bar() public view {
        test_stru storage tts = storage_struct;
        assembly {
            // The variables 'a' and 'b' contain zero
            let a := storage_struct.offset
            let b := tts.offset

            // This changes the reference slot of 'tts'
            tts.slot := 5
        }
    }
}
