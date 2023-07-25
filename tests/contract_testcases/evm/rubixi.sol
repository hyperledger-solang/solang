 // rubixi.sol
 pragma solidity ^0.8.11;

 contract Rubixi {

         //Declare variables for storage critical to contract
         uint private balance = 0;
         uint private collectedFees = 0;
         uint private feePercent = 10;
         uint private pyramidMultiplier = 300;
         uint private payoutOrder = 0;

         address private creator;

         //Sets creator
         // <yes> <report> ACCESS_CONTROL
         function DynamicPyramid() public {
                 creator = msg.sender; //anyone can call this
         }

         modifier onlyowner {
                 if (msg.sender == creator) _;
         }

         struct Participant {
                 address etherAddress;
                 uint payout;
         }

         Participant[] private participants;

         //Fallback function
         constructor () {
                 init();
         }

         //init function run on fallback
         function init() private {
                 //Ensures only tx with value of 1 ether or greater are processed and added to pyramid
                 if (msg.value < 1 ether) {
                         collectedFees += msg.value;
                         return;
                 }

                 uint _fee = feePercent;
                 //50% fee rebate on any ether value of 50 or greater
                 if (msg.value >= 50 ether) _fee /= 2;

                 addPayout(_fee);
         }

         //Function called for valid tx to the contract
         function addPayout(uint _fee) private {
                 //Adds new address to participant array
                 participants.push(Participant(msg.sender, (msg.value * pyramidMultiplier) / 100));

                 //These statements ensure a quicker payout system to later pyramid entrants, so the pyramid has a longer lifespan
                 if (participants.length == 10) pyramidMultiplier = 200;
                 else if (participants.length == 25) pyramidMultiplier = 150;

                 // collect fees and update contract balance
                 balance += (msg.value * (100 - _fee)) / 100;
                 collectedFees += (msg.value * _fee) / 100;

                 //Pays earlier participiants if balance sufficient
                 while (balance > participants[payoutOrder].payout) {
                         uint payoutToSend = participants[payoutOrder].payout;
                         //payable(participants[payoutOrder].etherAddress).transfer(uint128(payoutToSend));

                         balance -= participants[payoutOrder].payout;
                         payoutOrder += 1;
                 }
         }

         //Fee functions for creator
         function collectAllFees() public onlyowner {
                 if (collectedFees == 0) revert();

                 payable(creator).transfer(uint128(collectedFees));
                 collectedFees = 0;
         }

         function collectFeesInEther(uint _amt) public onlyowner {
                 _amt *= 1 ether;
                 if (_amt > collectedFees) collectAllFees();

                 if (collectedFees == 0) revert();

                 payable(creator).transfer(uint128(_amt));
                 collectedFees -= _amt;
         }

         function collectPercentOfFees(uint _pcent) public onlyowner {
                 if (collectedFees == 0 || _pcent > 100) revert();

                 uint feesToCollect = collectedFees / 100 * _pcent;
                 payable(creator).transfer(uint128(feesToCollect));
                 collectedFees -= feesToCollect;
         }

         //Functions for changing variables related to the contract
         function changeOwner(address _owner) public onlyowner {
                 creator = _owner;
         }

         function changeMultiplier(uint _mult) public onlyowner {
                 if (_mult > 300 || _mult < 120) revert();

                 pyramidMultiplier = _mult;
         }

         function changeFeePercentage(uint _fee) public onlyowner {
                 if (_fee > 10) revert();

                 feePercent = _fee;
         }

         //Functions to provide information to end-user using JSON interface or other interfaces
         function currentMultiplier() view public returns(uint multiplier, string memory info) {
                 multiplier = pyramidMultiplier;
                 info = 'This multiplier applies to you as soon as transaction is received, may be lowered to hasten payouts or increased if payouts are fast enough. Due to no float or decimals, multiplier is x100 for a fractional multiplier e.g. 250 is actually a 2.5x multiplier. Capped at 3x max and 1.2x min.';
         }

         function currentFeePercentage() view public returns(uint fee, string memory info) {
                 fee = feePercent;
                 info = 'Shown in % form. Fee is halved(50%) for amounts equal or greater than 50 ethers. (Fee may change, but is capped to a maximum of 10%)';
         }

         function currentPyramidBalanceApproximately() view public returns(uint pyramidBalance, string memory info) {
                 pyramidBalance = balance / 1 ether;
                 info = 'All balance values are measured in Ethers, note that due to no decimal placing, these values show up as integers only, within the contract itself you will get the exact decimal value you are supposed to';
         }

         function nextPayoutWhenPyramidBalanceTotalsApproximately() view public returns(uint balancePayout) {
                 balancePayout = participants[payoutOrder].payout / 1 ether;
         }

         function feesSeperateFromBalanceApproximately() view public returns(uint fees) {
                 fees = collectedFees / 1 ether;
         }

         function totalParticipants() view public returns(uint count) {
                 count = participants.length;
         }

         function numberOfParticipantsWaitingForPayout() view public returns(uint count) {
                 count = participants.length - payoutOrder;
         }

         function participantDetails(uint orderInPyramid) view public returns(address Address, uint Payout) {
                 if (orderInPyramid <= participants.length) {
                         //Address = participants[orderInPyramid].etherAddress;
                         Payout = participants[orderInPyramid].payout / 1 ether;
                 }
         }
 }

// ---- Expect: diagnostics ----
// warning: 67:31-43: local variable 'payoutToSend' is unused
// warning: 150:87-94: return variable 'Address' has never been assigned
