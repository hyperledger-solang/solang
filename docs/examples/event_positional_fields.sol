event UserModified(
    address user,
    string name
) anonymous;

event UserModified(
    address user,
    uint64 groupid
);

contract user {
    function set_name(address user, string name) public {
        emit UserModified({ user: user, name: name });
    }

    function set_groupid(address user, uint64 id) public {
        emit UserModified({ user: user, groupid: id });
    }
}
