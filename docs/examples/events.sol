event CounterpartySigned (
    address indexed party,
    address counter_party,
    uint contract_no
);

contract Signer {
    function sign(address counter_party, uint contract_no) public {
        emit CounterpartySigned(address(this), counter_party, contract_no);
    }
}
