contract Testing {
    address[] splitAddresses;

    function split(address addr1, address addr2) public {
        splitAddresses = [addr1, addr2];
    }

    function getIdx(uint32 idx) public view returns (address) {
        return splitAddresses[idx];
    }

    function getVec(uint32 a, uint32 b) public pure returns (uint32[] memory) {
        uint32[] memory vec;
        vec = [a, b];
        return vec;
    }
}
