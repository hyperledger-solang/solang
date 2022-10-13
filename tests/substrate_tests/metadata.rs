// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity;
use parity_scale_codec::{Decode, Encode};

/// The `mother` contract from the `ink!` examples, represented in solidity.
#[test]
fn mother_contract() {
    let _runtime = build_solidity(
        r##"
        struct Bids {
            address addr;
            uint128 size;
        }

        struct Status {
            uint32 blocknumber;
            AuctionStatus status;
            Outline outline;
        }

        enum Outline {
            NoWinner, WinnerDetected, PayoutCompleted
        }

        enum AuctionStatus {
            NotStarted, OpeningPeried, EndingPeriod, Ended, RfDelay
        }

        struct Auction {
            string name;
            uint8[32] subject;
            Bids[][] bids;
            uint32[3] terms;
            Status status;
            bool finalized;
            uint8[] vector;
        }

        contract Mother {
            Auction auction;
            mapping(address => uint128) balances;
        
            constructor(Auction a) {
                auction = a;
            }
        
            // constructor default() {} 
        
            function echo_auction(Auction a) public pure returns (Auction) {
                return a;
            }
        
            function revert_or_trap(bool revert, bool trap) public pure {
                if (revert) {
                
                }
                assert(!trap);
            }
        
            function debug_log(string message) public pure {
                print(message);
            }
        }"##,
    );
}
