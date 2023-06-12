## YALOB

**Y**et **a**nother **l**imit **o**rder **b**ook. This one is optimized for *simplicity and performance*. Materially, that manifests in the following ways:
- ~1,000 lines of code, instead of the typical ~10,000
- basic data structures like linked lists and bitmaps, instead of more complex ones like red-black trees or patricia tries
- ~3,300 to ~5,800 CUs to submit and update limit orders, with a typical access pattern (frequent updates at the top of the book) leading to sub-4,000 CU consumption
- ~2,600 CUs to cancel limit orders
- no keeper transactions ('cranking') required

To accomplish this, YALOB makes a number of trade-offs. These include:
- a book depth of 128 orders, instead of the typical ~1,000
- market makers need to register themselves on an order book before they can submit limit orders to that book
- missing features such as automatically-expiring orders, oracle-pegged orders, and permissioned markets

One feature that YALOB *does* have is a TWAP oracle.
