contract array_struct_mapping_storage {
    struct A {
        mapping(uint256 => uint256) a;
    }

    A[] private map;
    // canary here to ensure we don't overwrite it at any point
    int64 public number;

    function set(uint64 array_no, uint64 index, uint64 val) public {
        map[array_no].a[index] = val;
    }

    function get(uint64 array_no, uint64 index) public view returns (uint256) {
        return map[array_no].a[index];
    }

    function rm(uint64 array_no, uint64 index) public {
        delete map[array_no].a[index];
    }

    function push() public {
        map.push();
    }

    function pop() public {
        map.pop();
    }

    function setNumber(int64 x) public {
        number = x;
    }
}