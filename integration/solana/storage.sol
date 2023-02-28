 // ethereum solc wants his pragma
pragma abicoder v2;

contract store {
    enum enum_bar { bar1, bar2, bar3, bar4 }
    struct struct_foo {
        enum_bar f1;
        bytes f2;
        int64 f3;
        bytes3 f4;
        string f5;
        inner_foo f6;
    }

    struct inner_foo {
        bool in1;
        string in2;
    }

    uint64 u64;
    uint32 u32;
    int16 i16;
    int256 i256;
    uint256 u256;
    string str;
    bytes bs = hex"b00b1e";
    bytes4 fixedbytes;
    enum_bar bar;
    struct_foo foo1;
    struct_foo foo2;

    function set_values() public {
        u64 = type(uint64).max;
        u32 = 0xdad0feef;
        i16 = 0x7ffe;
        i256 = type(int256).max;
        u256 = 102;
        str = "the course of true love never did run smooth";
        fixedbytes = "ABCD";
        bar = enum_bar.bar2;
    }

    function get_values1() public view returns (uint64, uint32, int16, int256) {
        return (u64, u32, i16, i256);
    }

    function get_values2() public view returns (uint256, string memory, bytes memory, bytes4, enum_bar) {
        return (u256, str, bs, fixedbytes, bar);
    }

    function do_ops() public {
        unchecked {
        // u64 will overflow to 1
        u64 += 2;
        u32 &= 0xffff;
        // another overflow
        i16 += 1;
        i256 ^= 1;
        u256 *= 600;
        str = "";
        bs[1] = 0xff;
        // make upper case
        fixedbytes |= 0x20202020;
        bar = enum_bar.bar4;
        }
    }

    function push_zero() public {
        bs.push();
    }

    function push(bytes1 b) public {
        bs.push(b);
    }

    function pop() public returns (byte) {
        // note: ethereum solidity bytes.pop() does not return a value
        return bs.pop();
    }

    function get_bs() public view returns (bytes memory) {
        return bs;
    }

    // function for setting the in2 field in either contract storage or memory
    function set_storage_in2(struct_foo storage f, string memory v) internal {
        f.f6.in2 = v;
    }

    // A memory struct is passed by memory reference (pointer)
    function set_in2(struct_foo memory f, string memory v) pure internal {
        f.f6.in2 = v;
    }

    function get_both_foos() public view returns (struct_foo memory, struct_foo memory) {
        return (foo1, foo2);
    }

    function get_foo(bool first) public view returns (struct_foo memory) {
        struct_foo storage f;

        if (first) {
            f = foo1;
        } else {
            f = foo2;
        }

        return f;
    }

    function set_foo2(struct_foo f, string v) public {
        set_in2(f, v);
        foo2 = f;
    }

    function set_foo1() public {
        foo1.f1 = enum_bar.bar2;
        foo1.f2 = "Don't count your chickens before they hatch";
        foo1.f3 = -102;
        foo1.f4 = hex"edaeda";
        foo1.f5 = "You can't have your cake and eat it too";
        foo1.f6.in1 = true;

        set_storage_in2(foo1, 'There are other fish in the sea');
    }

    function delete_foo(bool first) public {
        struct_foo storage f;

        if (first) {
            f = foo1;
        } else {
            f = foo2;
        }

        delete f;
    }

    function struct_literal() public {
        // declare a struct literal with fields. There is an
        // inner struct literal which uses positions
        struct_foo literal = struct_foo({
            f1: enum_bar.bar4,
            f2: "Supercalifragilisticexpialidocious",
            f3: 0xeffedead1234,
            f4: unicode'€',
            f5: "Antidisestablishmentarianism",
            f6: inner_foo(true, "Pseudopseudohypoparathyroidism")
        });

        // a literal is just a regular memory struct which can be modified
        literal.f3 = 0xfd9f;

        // now assign it to a storage variable; it will be copied to contract storage
        foo1 = literal;
    }
}
