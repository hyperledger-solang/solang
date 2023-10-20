library Library {
    function set(
        int32[100] storage data,
        uint256 index,
        int32 value
    ) internal  {
        data[index] = value;
    }
}

contract TestContract {
    using Library for int32[100];

    int32[100] public dataArray;

    function setElement(uint256 index, int32 value) public {
        dataArray.set(index, value);
    }
}
