struct user {
    string name;
    bool active;
}

contract auth {
    function authenticate(string name, db.users users) public {
        // ...
    }
}

abstract contract db {
    struct users {
        user[] field1;
        int32 count;
    }
}
