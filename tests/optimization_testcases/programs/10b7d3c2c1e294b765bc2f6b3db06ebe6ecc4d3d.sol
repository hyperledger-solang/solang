contract foo {
    mapping(uint64 => uint64)[] public map;
    int64 public number;

    function set(uint64 array_no, uint64 index, uint64 val) public {
        map[array_no][index] = val;
    }

    function rm(uint64 array_no, uint64 index) public {
        delete map[array_no][index];
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

    function length() public returns (uint64) {
        return map.length;
    }
}
