uint64 constant VAL = 42;

contract contract1 {
    function foo() internal virtual returns (uint64) {
        uint32 value = 42;
        if (true) {
            uint64 first = 1; 
            return value + first;
        } else {
            uint64 second = 2;
            return value + second + VAL;
        }     
    }
    function foo2(struct1 example) internal virtual returns (uint64) {
        struct1 s = example;
            uint64 first = s.field1;
            uint64 second = s.field2;
        if(s.field3 == enum1.one) {
            struct2 vall = s.field4;
        }
        return first + second; 
    }
}

struct struct1 {
    uint64 field1;
    uint64 field2;
    enum1 field3;
    struct2 field4;
}
struct struct2 {
    uint64 aaa;
    uint64 bbbb;
}
event event1(uint32 a);
enum enum1 {
    one,
    two,
    three
}
