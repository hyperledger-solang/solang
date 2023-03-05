contract DebugBuffer {
    function multiple_prints() public {
        print("Hello!");
        print("I call seal_debug_message under the hood!");
    }

    function multiple_prints_then_revert() public {
        print("Hello!");
        print("I call seal_debug_message under the hood!");
        revert("sesa!!!");
    }
}
