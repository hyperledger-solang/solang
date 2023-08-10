contract Testing {
    string[] string_vec;

    function testLength() public returns (uint32, uint32, uint32) {
        string_vec.push("tea");
        string_vec.push("coffe");
        string_vec.push("sixsix");
        string[] memory rr = string_vec;
        return (rr[0].length, rr[1].length, rr[2].length);
    }

    function getString(uint32 index) public view returns (string memory) {
        string[] memory rr = string_vec;
        return rr[index];
    }
}
