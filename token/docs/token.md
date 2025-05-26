# Autonomi Community Token

Autonomi token standard, that enables issuing own token by anyone – from scratch, or by wrapping existing ERC20 tokens.

Token is based on a DAG of transactions represented in the network's storage as instances of GraphEntry data. GraphEntries are connected by *parents* and *descendants* lists. The token comes in two flavors: *Native* and *Bridged*.

We begin with creating library and application for Autonomi, not including changes to API/node code.

## Native ACT

Each graph is a separate token.

Token ID is a xorname of the Chunk containing token info: *symbol*, *name*, *decimals* and address (PublicKey) of genesis spend. This token ID will be different for every new token, even with same symbol, name and decimals.

Genesis transaction GE:
* *content* is token ID
* Empty *parents* (indicating Native genesis transaction)
* Single *output* to issuer's key. This value is a *totalSupply*.

## Bridged (ERC20) ACT

A token consists of multiple graphs, each of them created from a EVM burn transaction. Token info can be read from EVM blockchain, currently Arbitrum.

Token ID is an EVM token contract address prepended by characters "ERC20", a zero byte (0x00) and first 6 bytes of SHA256 hash of minimal unsigned int (eg. `0x02AB34` for 174900) representation of Chain Id (eg. `sha256(0xA4B1)[0..6]` for Arbittrum One). Potential Native Token would be simply a bridged [ANT ERC20 from Arbitrum One](https://arbiscan.io/token/0xa78d8321b20c4ef90ecd72f2588aa985a4bdb684), so its token ID would be (0x)`4552433230 00 2CEA1CB4897D A78D8321B20C4EF90ECD72F2588AA985A4BDB684`.

Genesis transaction GE:
* *content* is token ID with zeros as token contract address (indicating Bridged genesis transaction)
* Single *parents* entry, being a burn TX id on EVM blockchain
* Single *output* to public key of secret created from EVM Private Key and burn TX id. This balance can be then spent by using the same secret.

## Common (Native / Bridged)

Ordinary GE:
* Parents
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
* Sum of inputs = sum of outputs 
* Genesis transaction

If an invalid transaction is detected during validation of an incoming transaction (someone sent us money), such transaction could be rejected by putting a "burn" GrephEntry (one with empty *outputs*) or publishing a marker.

## A Wallet

List of Secret Keys, which Public Keys are outputs of some transactions. Together with transaction pointers to include as parents, when user wants to spend the output.

?? Should wallet be kept on the network or locally? Two types of the wallet? Consider security and convenience.

## Use cases

* User A creates a new token
* User A sends amount of a token to User B
* Show User A's posessed amounts of each token on each address / public key
* Show token info of a transaction
* ...
