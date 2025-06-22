/// SPDX-License-Identifier: Apache-2.0

contract storage_types {
            
    uint64 public temporary var = 1;
    uint64 public instance var1 = 1;
    uint64 public persistent var2 = 2;
    uint64 public var3 = 2;

    function inc() public {
        var++;
        var1++;
        var2++;
        var3++;
    }

    function dec() public {
        var--;
        var1--;
        var2--;
        var3--;
    }
}