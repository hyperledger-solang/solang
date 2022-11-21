contract s {
    struct user {
        address f1;
        int256[] list;
    }
    user[1000] users;

    function clear() public {
        // delete has to iterate over 1000 users, and for each of those clear the
        // f1 field, read the length of the list, and iterate over each of those
        delete users;
    }
}
