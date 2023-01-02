contract b {
    struct user {
        bool exists;
        address addr;
    }
    mapping(string => user) users;

    function add(string name, address addr) public {
        // This construction is not recommended, because it requires two hash calculations.
        // See the tip below.
        users[name].exists = true;
        users[name].addr = addr;
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
