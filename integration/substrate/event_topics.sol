contract event_topics {
    event happens(string indexed name, string indexed what, int64 id);

    function foo(string memory name, int64 id) public {
        emit happens(name, "foo", id);
    }

    function bar(string memory name, int64 id) public {
        emit happens(name, "bar", id);
    }
}
