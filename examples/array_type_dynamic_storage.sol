contract s {
    int64[] a;

    function test() public {
        // push takes a single argument with the item to be added
        a.push(128);
        // push with no arguments adds 0
        a.push();
        // now we have two elements in our array, 128 and 0
        assert(a.length == 2);
        a[0] |= 64;
        // pop removes the last element
        a.pop();
        // you can assign the return value of pop
        int64 v = a.pop();
        assert(v == 192);
    }
}
