 // ethereum solc wants his pragma
pragma abicoder v2;

contract structs {
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

    struct_foo foo1;
    struct_foo foo2;

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

    function pass_foo2(struct_foo f, string v) public pure returns (struct_foo) {
        set_in2(f, v);
        return f;
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
