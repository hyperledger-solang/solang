contract ethereum {
    // As a public mapping, this creates accessor function called balance, which takes
    // an address as an argument, and returns an uint
    mapping(address => uint256) public balances;

    // A public array takes the index as an uint argument and returns the element,
    // in this case string.
    string[] users;
}
