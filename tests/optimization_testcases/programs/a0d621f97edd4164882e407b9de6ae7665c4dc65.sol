contract c {
    event myevent(int32 indexed a, int32 b);

    function go() public {
        emit myevent(1, -2);
    }
}
