# Autonomi Community Token

Autonomi token standard, that enables issuing own token by anyone – from scratch, or by wrapping existing ERC20 tokens.

Token is based on a DAG of transactions represented in the network's storage as instances of GraphEntry data. GraphEntries are connected by *parents* and *descendants* lists. The token comes in two flavors: *Native* and *Bridged*.

We begin with creating lib/application for Autonomi, not icluding changes to API/node code.

## Native

Each graph structure is a separate token.

Token ID is a xorname (public key?) of the structure (Chunk?) containing token info: *symbol*, *name*, *totalSupply*, *decimals*. Future Native Token could have token ID consisting of zeros and token info would be hardcoded.

Genesis transaction GE:
* *content* token ID, as in ordinary GE
* Empty *parents* (indicating a genesis transaction)

## Bridged (ERC20)

A token consists of multiple graphs, each of them created from a EVM burn transaction. Token info can be read from EVM blockchain, currently Arbitrum.

Token ID is a hash/derivation of EVM token address. TODO: or maybe first bridge user should create token info structure similar to Native mode?

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

## Validation

Validity of transactions are checked client-side by traversing DAG **backwards** from an output we want to use during a payment, and **forward** from a set of remembered past transactions in a spare time. Incentive for validation is an importance of the transaction. The higher the amount, the more time we are willing to spend on validating incoming transaction.

### Backwards, during payment

This validation is carried out on payee's own advantage, to make sure the output we want to use is not a descendant of some earlier invalid transaction.

1. Empty list of transactions to validate is created.
2. Validation starts from transactions, which outputs we want to use, they are added to the list.
3. Check if they all have same Genesis GE and token ID
4. Validate genesis transaction
5. Random transaction is taken from the list and validated.
6. Check if there is an **error marker** for this transaction.
7. This transaction's **parents** are added to the list.
8. Go to step 5, repeat until user stops the process.

### Forward, in spare time

This validation is carried out to other users' advantage, to make sure entire system is secure. When invalid transaction is detected, an error marker is created for one (or more?) descendant transaction to indicate, that it's not safe to use outputs from it (them).

The rationale for this process is, that a cheater could create such a long chain of valid transactions after an invalud one, so that payee would not have time to validate them all, thus effectively hiding it from *Backwards* validation.

1. List of starting transactions is read from storage.
2. If list is empty, some user's past transaction is added.
3. Transaction is taken from the top of the list and validated.
4. If it's invalid, the **marking process** starts, and validation ends.
5. If valid, transaction's **random child** is added to the top of the list.
6. If transaction does not have children, it's added to end of the list as a starting transaction, to start from when any of it's output is spent, and another transaction is taken from list.
7. When all transactions from list are processed, user's past transactions are added.
8. Go to step 3, repeat until user stops the process.

#### Marking Process

DAG is traversed from invalid transaction forward (by choosing random spent output), creating **error markers** for all transactions, that have unspent outputs, until user stops the process, or transaction with no spent outputs is encountered.

One could argue, that false error marker could be created for any transaction by a vicious user, but by the nature of this GE-based token system, and its high privacy coming from usage of one-time keys, it's hard to connect a key (hence transaction or unspent output) with particular person/user.


### Validation (common)

* Signature
* Parent's token ID is equal children's token ID
* Parent's Genesis GE is equal children's Genesis GE
* Graph acyclicity (TODO: check with theory/proofs)
  * Keep list/hashset of visited GEs, check that they are not visited twice
  * A "self-cycle" is allowed as a special marker of Bridged genesis transaction
* Sum of inputs = sum of outputs


## A Wallet

List of Private Keys, which Public Keys are outputs of some transactions. Together with transaction pointers to include as parents, when user wants to spend the output.

?? Should wallet be kept on the network or locally? Two types of the wallet? Consider security and convenience.

## Use cases

* User A creates a new token
* User A sends amount of a token to User B
* Show User A's posessed amounts of each token on each address / public key
* Show token info of a transaction
* ...
