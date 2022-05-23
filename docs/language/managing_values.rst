Managing values
===============

Sending and receiving value
___________________________

Value in Solidity is represented by ``uint128``.

.. note::

    Parity Substrate can be compiled with a different type for ``T::Balance``. If you
    need support for a different type, please raise an
    `issue <https://github.com/hyperledger-labs/solang/issues>`_.

Checking your balance
_____________________

The balance of a contract can be checked with `address` ``.balance``, so your own balance
is ``address(this).balance``.

.. note::
    Parity Substrate cannot check the balance for contracts other than the current
    one. If you need to check the balance of another contract, then add a balance
    function to that contract like the one below, and call that function instead.

.. code-block:: solidity

    function balance() public returns (uint128) {
        return address(this).balance;
    }

Creating contracts with an initial value
________________________________________

You can specify the value you want to be deposited in the new contract by
specifying ``{value: 100 ether}`` before the constructor arguments. This is
explained in :ref:`sending value to the new contract <sending_values>`.

Sending value with an external call
___________________________________

You can specify the value you want to be sent along with the function call by
specifying ``{value: 100 ether}`` before the function arguments. This is
explained in :ref:`passing value and gas with external calls <passing_value_gas>`.

.. _send_transfer:

Sending value using ``send()`` and ``transfer()``
_________________________________________________

The ``send()`` and ``transfer()`` functions are available as method on a
``address payable`` variable. The single arguments is the amount of value you
would like to send. The difference between the two functions is what happens
in the failure case: ``transfer()`` will revert the current call, ``send()``
returns a ``bool`` which will be ``false``.

In order for the receiving contract to receive the value, it needs a ``receive()``
function, see :ref:`fallback() and receive() function <fallback_receive>`.

Here is an example:

.. code-block:: solidity

    contract A {
        B other;

        constructor() {
            other = new B();

            bool complete = payable(other).transfer(100);

            if (!complete) {
                // oops
            }

            // if the following fails, our transaction will fail
            other.send(100);
        }
    }

    contract B {
        receive() payable external {
            // ..
        }
    }

.. note::
    On Subtrate, this uses the ``seal_transfer()`` mechanism rather than ``seal_call()``, since this
    does not come with gas overhead. This means the ``receive()`` function is not required in the
    receiving contract, and it will not be called if it is present. If you want the ``receive()``
    function to be called, use ``address.call{value: 100}("")`` instead.
