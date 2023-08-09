contract foo {
    mapping(uint64 => uint64) map;

    function set(uint64 index, uint64 val) public {
        map[index] = val;
    }

    function get(uint64 index) public returns (uint64) {
        return map[index];
    }

    function rm(uint64 index) public {
        delete map[index];
    }
}
