contract contract1 {
    function foo() internal virtual returns (uint64) {
        uint32 value = 42;
        if (true) {
            uint64 first = 1; 
            return value + first;
        } else {
            uint64 second = 2;
            return value + second;
        }     
    }
    function foo2(struct1 example) internal virtual returns (uint64) {
        struct1 s = example;
        uint64 first = s.field1;
        uint64 second = s.field2;
        return first + second; 
    }
}

struct struct1 {
    uint64 field1;
    uint64 field2;
}
