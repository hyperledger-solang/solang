contract CrowdProposal {
    address payable public immutable author;
    constructor(address uni_) public {}

}


contract CrowdProposalFactory {
    address public immutable uni;
    function createCrowdProposal() external {
        CrowdProposal proposal = new CrowdProposal(uni);
    }
}
// ---- Expect: diagnostics ----
// warning: 3:25-29: function parameter 'uni_' is unused
// warning: 3:31-37: 'public': visibility for constructors is ignored
// warning: 11:23-31: local variable 'proposal' is unused
