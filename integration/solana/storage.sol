contract store {
    enum enum_bar { bar1, bar2, bar3, bar4 }

    uint64 u64;
    uint32 u32;
    int16 i16;
    int256 i256;
    uint256 u256;
    string str;
    bytes bs = hex"b00b1e";
    bytes4 fixedbytes;
    enum_bar bar;

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

    function get_values2() public view returns (uint256, string, bytes, bytes4, enum_bar) {
        return (u256, str, bs, fixedbytes, bar);
    }

    function do_ops() public {
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