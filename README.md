
# Introduction
 
OXYGEN presents an on-chain order book matching solution on Fuel blockchain.

Traditionally, order books are maintained off-chain by centralized exchanges, where they store and manage the buy and sell orders from traders. However, in the case of an on-chain order book, the order book data is stored directly on the blockchain smart contract, making it accessible to all participants and ensuring transparency and immutability.

# On-chain order book matching on Fuel

On-chain order book matching refers to the process of matching buy and sell orders for a specific asset directly on a decentralized blockchain network. It eliminates the need for intermediaries or centralized exchanges to facilitate trading.

In a traditional centralized exchange, order book matching occurs off-chain, where the exchange's matching engine pairs buy and sell orders and executes trades. However, with on-chain order book matching, the entire process, including order placement, matching, and execution, takes place directly on the blockchain.

Here's a simplified overview of how on-chain order book matching on OXYGEN works:

1. Data Structure: The order book is implemented as a vector, which is a resizable array-like data structure in Sway. Vectors provide constant time access to elements and efficient insertion and removal operations.

2. Order Representation: Each order is represented by a struct that contains relevant information such as the price, quantity, order type (buy or sell), and any additional metadata required for order matching.

3. Sorting: The vector is sorted based on the price of the orders. Typically, buy orders are sorted in descending order (highest price first) and sell orders in ascending order (lowest price first). This sorting allows for efficient retrieval and matching of orders during trading.

4. Order Insertion: When a new order is received, it is inserted into the order book at the appropriate position based on its price. The vector's insert method is used to maintain the order book's sorted nature.

5. Order Matching: The order book facilitates the matching of buy and sell orders. When a new order is inserted, it is compared with existing orders based on their prices to identify potential matches.

6. Quantity Adjustment: Once a match is found between two orders, the quantities of the matched orders are adjusted accordingly. The order book updates the quantities of the matched orders in place.

7. Order Removal: If an order's quantity reaches zero after matching, it is considered fully executed and removed from the order book using the vector's remove method. This keeps the order book up to date and ensures efficient order matching.

On-chain order book matching offers several advantages. It enhances transparency as all order book data and trade executions are publicly verifiable on the blockchain. It eliminates the need to trust a centralized exchange with custody of funds, as trades occur directly between participants. Additionally, it enables decentralized applications (dApps) and smart contracts to interact with the order book, allowing for more complex trading strategies and automation.

# Vector-based order book in Sway

Here's how vector-based order book implemented in Sway:

Order Book Structure: The order book is represented as two separate storage vectors for buy orders and another for sell orders. Each vector contains order objects that include relevant information such as price, quantity, and trader details.


    
    storage {
        bids: StorageVec<OpenLimitOrder> = StorageVec {},
        asks: StorageVec<OpenLimitOrder> = StorageVec {},
    }


Sorting: The buy orders vector is sorted in descending order based on prices, while the sell orders vector is sorted in ascending order.

Buy Orders:
| Price  | Quantity |
|--------|----------|
| $10.50 | 5        |
| $10.25 | 3        |
| $10.00 | 8        |
| $9.75  | 2        |
| $9.50  | 6        |

Sell Orders:
| Price  | Quantity |
|--------|----------|
| $11.00 | 10       |
| $11.25 | 7        |
| $11.50 | 2        |
| $12.00 | 4        |
| $12.50 | 6        |



Finding the Match: The binary search algorithm divides the search range in half with each comparison, reducing the search space until it finds the matching price or determines that no match exists. The algorithm compares the target price with the middle price of the current search range and adjusts the search range accordingly.

Trade Execution: Once the matching price is found, the new order is matched with the existing order(s) at that price. The trade execution involves adjusting the quantities of the matched orders and transferring the assets between the involved parties' addresses.

 
 # Components
- Frontend 
    
    Frontend is the first place user to interact with the OXYGEN. Provides a simple method for 
    traders to create and submit orders, allowing a trader to request an amount of token they wish to buy or sell, and a price point, and whether they want a limit or market order 
        
- Orderbook Contract

    Sway programming language implementation for onchain order book matching , order management, order execution and settlement.
     
- Indexer

    Off-chain API for indexing on-chain order book data, open orders and trade history. 


# Compile the contracts using:
```console
make build
```
# Deploy contracts:


```console
./deploy_contracts.sh
```