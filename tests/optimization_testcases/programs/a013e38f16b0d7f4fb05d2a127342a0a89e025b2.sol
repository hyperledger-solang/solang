contract c1 {
    function test() public returns (string) {
        string ast = "Hello!";
        string bst = "from Solang";

        while (ast == bst) {
            ast = ast + "a";
        }

        return ast;
    }
}
