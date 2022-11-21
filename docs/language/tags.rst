
.. _tags:

Tags
====

Any contract, interface, library, event definition, struct definition, function, or contract variable
may have tags associated with them. These are used for generating documentation for the contracts,
when Solang is run with the ``--doc`` command line option. This option generates some html which
lists all the types, contracts, functions, and state variables along with their tags.

The tags use a special comment format. They can either be specified in block comments or single
line comments.

.. include:: ../examples/tags.sol
  :code: solidity

The tags which are allowed:

``@title``
    Headline for this unit

``@notice``
    General body for explaining what this unit does

``@dev``
    Any development notes

``@author``
    Field for the author of this code

``@param`` `name`
    Document a function parameter, field of struct or event. Requires a name of the field or parameter

``@return`` `name`
    Document a function return value. Requires a name of the field or parameter if the function returns
    more than one value.