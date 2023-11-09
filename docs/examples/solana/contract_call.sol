contract Polymath {
    function call_math() external returns (uint) {
        return Math.sum(1, 2);
    }
    function call_english(address english_id) external returns (string) {
        return English.concatenate{program_id: english_id}("Hello", "world");
    }
}

@program_id("5afzkvPkrshqu4onwBCsJccb1swrt4JdAjnpzK8N4BzZ")
contract Math {
    function sum(uint a, uint b) external returns (uint) {
        return a + b;
    }
}

contract English {
    function concatenate(string a, string b) external returns (string) {
        return string.concat(a, b);
    }
}
