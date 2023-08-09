contract Testing {
    string st;

    function setString(string input) public {
        st = input;
    }

    function getLength() public view returns (uint32) {
        return st.length;
    }
}
