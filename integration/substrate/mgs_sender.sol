contract mytoken {
    function test(address account, bool sender) public view returns (address) {
        if (sender) {
            return msg.sender;
        }
        return msg.sender;
    }
}