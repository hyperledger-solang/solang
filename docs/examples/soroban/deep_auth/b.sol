/// SPDX-License-Identifier: Apache-2.0

contract b {
 
    uint64 public instance counter = 20;

    function increment(address a, address c) public returns (uint64) {

        a.requireAuth();
        bytes payload = abi.encode("get_num", a);
        (bool suc, bytes returndata) = c.call(payload);
        uint64 result = abi.decode(returndata, (uint64));

        counter = counter + 2;

        return counter;
       
    }
} 