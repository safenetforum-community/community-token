# Autonomi Community Token

Autonomi token standard, that enables issuing own token by anyone – from scratch, or by wrapping existing ERC20 tokens.

Token is based on a DAG of transactions represented in the network's storage as instances of GraphEntry data. GraphEntries are connected by *parents* (inputs) and *children* (outputs) lists. The token comes in two flavors: *Native* and *Bridged / ERC-20*.

We begin with creating library and application for Autonomi, not including changes to API/node code.

## Common (Native and Bridged)

Token ID is a xorname of the Chunk containing token info: *symbol*, *name*, *decimals*, *type*, and additional fields dependent on the type.

Ordinary GE:
* Parents
  * Input 1
  * Input 2
  * ...
* Content: token ID
* Descendants: transaction outputs with amounts

## Native ACT

Each graph is a separate token.

Token info chunk contains address (PublicKey) of genesis spend. This token ID will be different for every new token, even with same symbol, name and decimals.

Genesis transaction GE:
* Empty *parents* (indicating Native genesis transaction)
* Single *output* to issuer's key. This value is a *totalSupply*.

## Bridged (ERC20) ACT

A token consists of multiple graphs, each of them created from a EVM burn transaction.

Token info chunk is redundant copy of info existing on the blockchain in token's smart contract. The additional info contains chain id of token's blockchain (currently only 42161, Arbitrum One's id is allowed) and address of token's contract.

Potential Native Token would be simply a bridged [ANT ERC20 from Arbitrum One](https://arbiscan.io/token/0xa78d8321b20c4ef90ecd72f2588aa985a4bdb684).

Genesis transaction GE:
* Single *parents* entry, being a burn TX id on EVM blockchain, does not exist on Autonomi.
* Single *output* to public key of secret created from EVM Private Key and burn TX id. This balance can be then spent by using the same secret.

## Verification

Validity of transactions are checked client-side by traversing DAG backwards. Incentive for validation is an importance of the transaction. The higher the amount, the more time we are willing to spend on validating incoming transaction.

1. List of transactions to validate is created.
2. Validation starts from transaction we want to validate, it is added to the list.
3. Random transaction is taken from the list and validated.
4. This transaction's parents are added to the list.
5. Go to step 3, repeat until user stops the process.

Validation
* Validate signature
* It has parents (or it can be a Native Genesis)
* All parents exist (or it can be a ERC20 Genesis)
* Parents' token IDs are equal child's token ID (or is child an Exchange GE? See exchange description)
* Graph acyclicity (TODO: check with theory/proofs)
  * Keep list/hashset of visited GEs, check that they are not visited twice
* Sum of inputs = sum of outputs

If an invalid transaction is detected during validation of an incoming transaction (someone sent us money), such transaction could be rejected by putting a "burn" GrephEntry (one with empty *outputs*) or publishing a marker.

(note) If parent GE has multiple outputs to a child GE, it's ok if child GE has one input referencing parent, with amount treated like it was a sum of parent outputs.

## Payment process

User has no fixed address, like in Ethereum or Bitcoin. The tokens, that user holds, are unspent outputs of some transactions/GEs created by payers, pointing to same address. When user wants to use some of these, new GE has to be created, so all these tokens have to be used. This situation is resolved by creating a new address, called *rest*, and new GE will have 2 outputs: the actual payee's pk/address and new, own *rest* address.

## A Wallet

Users have to keep track of GEs published with tokens, that they can spend, for example in a wallet structure, that keeps track of unspent outputs of graph entries, like a list of Secret Keys, which Public Keys are outputs of some transactions. Together with transaction pointers to include as parents, when user wants to spend the output.

On `request`, wallet returns (and creates if it does not exist) public key, that is associated with given token, so that spend can be created with output pointing at this key by the payer. All such spends are collected to the same key.

And if in turn, this user wants to spend, a new GE is created at this key(address), with those outputs as its inputs, and with two outputs: one being a PK given (in request process) by the payee, and second - a rest spent back to a new key that is created in user’s wallet for that token (old one is removed).

A wallet can be saved to a Scratchpad on every operation. But it can be kept somewhere else, like local storage, because it’s not referenced by any of token graph’s elements.

Wallet's structure is not part of ACT specification, although an example implementation is provided.

## Use cases

* User A creates a new token
* User A sends amount of a token to User B
* Show User A's posessed amounts of each token on each address / public key
* Show token info of a transaction
* ...

## Dictionary

Spend = Transaction = Graph Entry == GE

Public Key = Address

Secret Key = Private Key

Input = Parent

Output = Child

