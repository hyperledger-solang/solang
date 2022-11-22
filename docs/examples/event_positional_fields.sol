event UserModified(
    address user,
    string name
) anonymous;

event UserModified(
    address user,
    uint64 groupid
);

contract user {
    function set_name(string name) public {
        emit UserModified({ user: msg.sender, name: name });
    }

    function set_groupid(uint64 id) public {
        emit UserModified({ user: msg.sender, groupid: id });
    }
}
