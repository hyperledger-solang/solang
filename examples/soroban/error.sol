/// SPDX-License-Identifier: Apache-2.0

contract error {
    uint64 public count = 1;

    function decrement() public returns (uint64) {
        print("Second call will FAIL!");
        count -= 1;
        return count;
    }
}
