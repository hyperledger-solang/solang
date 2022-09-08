Events
======

In Solidity, contracts can emit events that signal that changes have occurred. For example, a Solidity
contract could emit a `Deposit` event, or `BetPlaced` in a poker game. These events are stored
in the blockchain transaction log, so they become part of the permanent record. From Solidity's perspective,
you can emit events but you cannot access events on the chain.

Once those events are added to the chain, an off-chain application can listen for events. For example, the Web3.js
interface has a `subscribe()` function. Another is example is
`Hyperledger Burrow <https://hyperledger.github.io/burrow/#/reference/vent>`_
which has a vent command which listens to events and inserts them into a Postgres database.

An event has two parts. First, there is a limited set of topics. Usually there are no more than 3 topics,
and each of those has a fixed length of 32 bytes. They are there so that an application listening for events
can easily filter for particular types of events, without needing to do any decoding. There is also a data
section of variable length bytes, which is ABI encoded. To decode this part, the ABI for the event must be known.

From Solidity's perspective, an event has a name, and zero or more fields. The fields can either be ``indexed`` or
not. ``indexed`` fields are stored as topics, so there can only be a limited number of ``indexed`` fields. The other
fields are stored in the data section of the event. The event name does not need to be unique; just like
functions, they can be overloaded as long as the fields are of different types, or the event has
a different number of arguments.

.. warning::
    On Solana, writing ``indexed`` besides an event field has no impact when emitting events. The ``indexed``
    keyword serves only to generate metadata information for IDL files. All event attributes will be encoded as data to
    be passed for Solana's ``sol_log_data`` system call, regardless of the ``indexed`` keyword being present. This
    behavior follows what Solana's Anchor framework does.

In Parity Substrate, the topic fields are always the hash of the value of the field. Ethereum only hashes fields
which do not fit in the 32 bytes. Since a cryptographic hash is used, it is only possible to compare the topic against a
known value.

An event can be declared in a contract, or outside.

.. code-block:: solidity

    event CounterpartySigned (
        address indexed party,
        address counter_party,
        uint contract_no
    );

    contract Signer {
        funtion sign(address counter_party, uint contract_no) internal {
            emit CounterpartySigned(address(this), counter_party, contract_no);
        }
    }

Like function calls, the emit statement can have the fields specified by position, or by field name. Using
field names rather than position may be useful in case the event name is overloaded, since the field names
make it clearer which exact event is being emitted.


.. code-block:: solidity

    event UserModified(
        address user,
        string name
    ) anonymous;

    event UserModified(
        address user,
        uint64 groupid
    );

    contract user {
        function set_name(string name) public {
            emit UserModified({ user: msg.sender, name: name });
        }

        function set_groupid(uint64 id) public {
            emit UserModified({ user: msg.sender, groupid: id });
        }
    }

In the transaction log, the first topic of an event is the keccak256 hash of the signature of the
event. The signature is the event name, followed by the fields types in a comma separated list in parentheses. So
the first topic for the second UserModified event would be the keccak256 hash of ``UserModified(address,uint64)``.
You can leave this topic out by declaring the event ``anonymous``. This makes the event slightly smaller (32 bytes
less) and makes it possible to have 4 ``indexed`` fields rather than 3.