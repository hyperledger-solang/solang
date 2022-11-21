contract b {
    struct user {
        bool exists;
        address addr;
    }
    mapping(string => user) users;

    function add(string name, address addr) public {
        // assigning to a storage variable creates a reference
        user storage s = users[name];

        s.exists = true;
        s.addr = addr;
    }

    function get(string name) public view returns (bool, address) {
        // assigning to a memory variable creates a copy
        user s = users[name];

        return (s.exists, s.addr);
    }

    function rm(string name) public {
        delete users[name];
    }
}
