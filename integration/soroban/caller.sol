contract caller {
    function add (address addr, uint64 a, uint64 b, uint64 c) public returns (uint64) {
        bytes payload = abi.encode("add", a, b, c);
        (bool suc, bytes returndata) = addr.call(payload);
        uint64 result = abi.decode(returndata, (uint64));
        return result;
    }
}