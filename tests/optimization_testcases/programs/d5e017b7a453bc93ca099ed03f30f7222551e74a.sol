contract foo {
    function test() public {
        uint32[] bar1 = new uint32[](0);
        uint32[] bar2 = new uint32[](0);

        // each time we call a system call, the heap is checked
        // for consistency. So do a print() after each operation
        for (uint64 i = 1; i < 160; i++) {
            if ((i % 10) == 0) {
                bar1.pop();
                print("bar1.pop");
                bar2.pop();
                print("bar2.pop");
            } else {
                uint32 v = bar1.length;
                bar1.push(v);
                print("bar1.push");
                bar2.push(v);
                print("bar2.push");
            }
        }

        assert(bar1.length == bar2.length);

        for (uint32 i = 0; i < bar1.length; i++) {
            assert(bar1[i] == i);
            assert(bar2[i] == i);
        }
    }
}
