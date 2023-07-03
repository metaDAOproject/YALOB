## YALOB

**Y**et **a**nother **l**imit **o**rder **b**ook. This one is optimized for *simplicity and performance*. That means:
- ~1,000 lines of code instead of the typical ~10,000
- basic data structures like linked lists and bitmaps instead of more complex ones like red-black trees or patricia tries
- ~3,300 to ~5,800 CUs to submit and update limit orders, with a typical access pattern (frequent updates at the top of the book) leading to sub-4,000 CU consumption
- ~2,600 CUs to cancel limit orders
- no keeper transactions ('cranking') required

To accomplish this, YALOB makes a number of trade-offs. These include:
- a book depth of 128 orders instead of the typical ~1,000
- market makers need to register themselves on an order book before they can submit limit orders to that book
- missing features such as automatically-expiring orders, oracle-pegged orders, and permissioned markets
.
### TWAP Oracle

One feature that YALOB *does* have is a TWAP oracle. This oracle can be used by other applications to get the average price of a token over a time range, specified in slots. For example, a borrow-lend protocol could value a user's token collateral by fetching the token's average price over the last 9000 slots (approximately 1 hour).

This TWAP oracle works by making an `observation` the first time the program is called in a slot. In most cases, the `observation` will simply be the spot price of the token: the average of the highest bid and the lowest offer. 

The program multiplies `observation` by the number of slots that have passed since the last one (expected to be 1 for liquid markets), to receive `weighted_observation`. Then, `weighted_observation` is added to `observation_accumulator`, so that `observation_accumulator` represents the sum of all `weighted_observation`s since the dawn of the market.

To calculate a TWAP, one must first retreive the value of the `observation_accumulator` at the start of the time range. Then, at the end of the time range, one must pull it again, subtract the earlier accumulator from the later one, and divide by the number of slots passed. 

#### Manipulation-resistance

One of the problems with decentralized TWAP oracles is that they are sensitive to manipulation. This is especially true on PoS networks with leader schedules like Solana: a validator can clear out an order book at the end of one block, and then make the first trade on the order book the next block, pushing the price up to infinity or down to zero. 

Our solution is to only allow an observation to change a certain amount per slot, such as 1%. We call this amount `oracle_sensitivity`, and make it configurable. 
