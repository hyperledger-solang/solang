/// SPDX-License-Identifier: Apache-2.0

contract c {
    function get_num(address a) public returns (uint64) {
        a.requireAuth();
        return 2;
    }
}