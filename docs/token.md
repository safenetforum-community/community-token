# Autonomi Community Token

Autonomi token standard, that enables issuing own token by anyone – from scratch, or by wrapping existing ERC20 tokens.

Token is based on a DAG of transactions represented in the network's storage as instances of GraphEntry data. GraphEntries are connected by *parents* and *descendants* lists. The token comes in two flavors: *Native* and *Bridged*.

We begin with creating lib/application for Autonomi, not icluding changes to API/node code.

## Native

Each graph is a separate token.

Token ID is a xorname (public key?) of the structure (Chunk?) containing token info: *symbol*, *name*, *totalSupply*, *decimals*. Future Native Token could have token ID consisting of zeros and token info would be hardcoded.

Genesis transaction GE:
* *content* token ID, as in ordinary GE
* Empty *parents* (indicating a genesis transaction)

## Bridged (ERC20)

A token consists of multiple graphs, each of them created from a EVM burn transaction. Token info can be read from EVM blockchain, currently Arbitrum.

Token ID is a hash/derivation of EVM token address. TODO: or maybe first bridge user should create token info structure?

Genesis transaction GE:
* *content* is a burn transaction ID on the blockchain
* Single *parents* entry, pointing to the GE itself, as a marker of Bridged genesis transaction.

## Common (Native / Bridged)

Ordinary GE:
* Parents
  * Genesis GE
  * Input 1
  * Input 2
  * ...
* Content: token ID
* Descendants: transaction outputs with amounts

## Verification

Validity of transactions are checked client-side by traversing DAG backwards. Incentive for validation is an importance of the transaction. The higher the amount, the more time we are willing to spend on validating incoming transaction.

1. List of transactions to validate is created.
2. Validation starts from transaction we want to validate, it is added to the list.
3. Random transaction is taken from the list and validated.
4. This transaction's parents are added to the list.
5. Go to step 3, repeat until user stops the process.

Validation
* Signature
* Parent's token ID is equal children's token ID
* Parent's Genesis GE is equal children's Genesis GE
* Graph acyclicity (TODO: check with theory/proofs)
  * Keep list/hashset of visited GEs, check that they are not visited twice
  * A "self-cycle" is allowed as a special marker of Bridged genesis transaction
* Sum of inputs = sum of outputs 
* Genesis transaction

## A Wallet

List of Private Keys, which Public Keys are outputs of some transactions. Together with transaction pointers to include as parents, when user wants to spend the output.

?? Should wallet be kept on the network or locally? Two types of the wallet? Consider security and convenience.

## Use cases

* User A creates a new token
* User A sends amount of a token to User B
* Show User A's posessed amounts of each token on each address / public key
* Show token info of a transaction
* ...
