/// Same as a.sol, but without a call to auth.authAsCurrContract
contract a_invalid {
    function call_b (address b, address c) public returns (uint64) {
        address addr = address(this);
        // authorize contract c to be called, with function name "get_num" and "a" as an arg.
        // get_num calls a.require_auth()
        //auth.authAsCurrContract(c, "get_num", addr);
        bytes payload = abi.encode("increment", addr, c);
        (bool suc, bytes returndata) = b.call(payload);
        uint64 result = abi.decode(returndata, (uint64));
        return result;
    }
}