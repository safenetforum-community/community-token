- non-deterministic magnitude pointers
	- **problem**: Cheater could generate big number of valid transactions **after a fabricated one (a "double spend")**, to make validation harder and to increase chance, that payee would not want to waste time to validate so many transactions.
	  **solution**: The bigger amount we receive, the longer it's worth spending on validating the transaction.  
	- **problem**: This would basically require a payee to validate **at least similar amount** of transactions, that a cheater has fabricated to hide the double spend, and thus making validation process very time-consuming.
	  **solution?**: We can keep pointers to ~10, ~100, ~1000, ... transactions back and validate 1,2,3,... transactions back as well as 10, 100, 1000, 11, 101, 1001, 12, 102, 1002,... That would (probably?) increase chance of finding malicious transaction, because cheater would probably try to hide behind more transactions if possible.  
	- **problem**: Cheater could always generate 99 or 999 or 9999 transactions, because payee would start validating from 1st, 10th, 100th, 1000th transaction.
	  **solution?**: Make validation unpredictable, introduce randomness, so that pointers would point at **about** required distance from current transaction.  
	- when **adding** new child transaction
		- if parent had *n* magnitude pointers, there is a *1/(2 * 10^n)* chance of creating new magnitude pointer at genesis transaction
		- magnitude pointers are moved forward by *random([0, 1, 2])* steps forward.
			- **problem**: cheater could move forward always by *0*, and it would be valid
			  **solution?**: magnitude pointer cannot be moved by the same amount as step earlier, so no *0* move after another *0* move, as well as no *1* after *1* and no *2* after *2*.  
	- When **validating**, check if magnitude pointers moved by 0-2 steps forward on every transaction, so that attacker could not point them where she wanted.
	- Added O(n) complexity is logarithmic

(Tue, 24.09.2024) Unfortunately, magnitude pointers are not helpful, because even if I can skip validation 1000 transactions back, I have no proof, that the pointed transaction is an ancestor of transaction I want to use, unless I track it myself. Cheater could point the 1000th pointer anywhere she wants.
