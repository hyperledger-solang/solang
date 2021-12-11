contract destruct {
    function hello() public pure  returns (string) {
        return "Hello";
    }

    function selfterminate(address payable deposit) public {
        selfdestruct(deposit);
    }
}