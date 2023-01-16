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