contract Events {
    /// Ladida tada
    event foo1(int64 id, string s);

    /// @title Event Foo2
    /// @notice Just a test
    /// @author them is me
    event foo2(int64 id, string s2, address a);

    event ThisEventTopicShouldGetHashed(address indexed caller);
    event Event(bool indexed something);

    function emit_event() public {
        emit foo1(254, "hello there");

        emit foo2(type(int64).max, "minor", address(this));

        emit ThisEventTopicShouldGetHashed(msg.sender);

        emit Event(true);
    }
}
