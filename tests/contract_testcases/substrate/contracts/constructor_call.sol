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
// ----
// warning (94-98): function parameter 'uni_' has never been read
// warning (100-106): 'public': visibility for constructors is ignored
// warning (249-257): local variable 'proposal' has been assigned, but never read
