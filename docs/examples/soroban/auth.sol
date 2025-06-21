/// SPDX-License-Identifier: Apache-2.0

contract auth {
    // Only this address can call the increment function
    address public owner = address"GDRIX624OGPQEX264NY72UKOJQUASHU3PYKL6DDPGSTWXWJSBOTR6N7W";

 
    uint64 public instance counter = 20;

    function increment() public returns (uint64) {

        owner.requireAuth();

        counter = counter + 1;

        return counter;
       
    }
}