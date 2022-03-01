contract mytokenEvent {
    event Debugging(address b);

    function test() public {
        emit Debugging(msg.sender);
    }
}