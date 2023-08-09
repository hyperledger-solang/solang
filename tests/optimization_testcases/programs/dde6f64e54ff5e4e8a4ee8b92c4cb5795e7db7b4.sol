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
