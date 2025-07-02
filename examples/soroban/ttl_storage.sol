/// SPDX-License-Identifier: Apache-2.0

contract ttl_storage {
    uint64 public persistent pCount = 11;
    uint64 temporary tCount = 7;
    uint64 instance iCount = 3;

    function extend_persistent_ttl() public view returns (int64) {
        return pCount.extendTtl(1000, 5000);
    }

    function extend_temp_ttl() public view returns (int64) {
        return tCount.extendTtl(3000, 7000);
    }

    function extendInstanceTtl() public view returns (int64) {
        return extendInstanceTtl(2000, 10000);
    }
}