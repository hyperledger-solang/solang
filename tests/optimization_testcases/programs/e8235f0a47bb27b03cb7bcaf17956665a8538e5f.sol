contract c {
    struct Struct {
        int field;
    }

    Struct mem;

    constructor() {
        mem = Struct(1);
    }

    function func() public view returns (int) {
        Struct bar = true ? mem : mem;
        Struct baz = bar;
        return baz.field;
    }
}
