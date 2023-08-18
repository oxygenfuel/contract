contract;

mod errors;
mod data_structures;
mod interface;
mod event;

use std::storage::*;
use std::{
    auth::{
        AuthError,
        msg_sender,
    },
    block::timestamp,
    call_frames::{
        contract_id,
        msg_asset_id,
    },
    context::msg_amount,
    revert::require,
    token::mint_to,
    token::transfer_to_address,
};
use ::data_structures::{Match, OpenLimitOrder, MatchResult, Trade, ZERO_B256};
use ::interface::{PlaceOrder};
use ::errors::{OrderbookError};

storage {
    base_asset_id: ContractId = ContractId {value:ZERO_B256},
    quote_asset_id: ContractId = ContractId {value:ZERO_B256},
    seq: u64 = 0,
    bids: StorageVec<OpenLimitOrder> = StorageVec {},
    asks: StorageVec<OpenLimitOrder> = StorageVec {},
    deposits: StorageMap<(Address, ContractId), u64> = StorageMap {},
    locked: StorageMap<(Address, ContractId), u64> = StorageMap {},
    trades: StorageVec<Trade> = StorageVec {},
}

abi Orderbook {
    #[storage(read, write)]
    fn place_order(palce_order: PlaceOrder) -> Vec<Match>;

    #[storage(read)]
    fn orderbook(side: u64) -> Vec<OpenLimitOrder>;

    #[storage(write, read), payable]
    fn deposit();

    #[storage(write, read)]
    fn withdraw(asset_id: ContractId, amount: u64);

    #[storage(read)]
    fn balance(address: Address, asset_id: ContractId) -> u64;

    #[storage(read, write)]
    fn init(base_asset:ContractId, quote_asset:ContractId);

    #[storage(read)]
    fn recent_trades(num: u64) -> Vec<Trade>;
}

impl Orderbook for Contract {
    #[storage(read, write)]
    fn init(base_asset:ContractId, quote_asset:ContractId) {
        storage.base_asset_id = base_asset;
        storage.quote_asset_id = quote_asset;
    }
    
    #[storage(read, write)]
    fn place_order(req: PlaceOrder) -> Vec<Match> {
        let price = req.price;
        let side = req.order_side;
        let mut unfilled_amount = req.amount;
        let mut matches: Vec<Match> = Vec::new();
        let mut filled_amount = 0;
        storage.seq = storage.seq + 1;
        let sender = get_msg_sender_address_or_panic();
        if side == 0 {
            // buy
            // check quote token amount 
            require((price / 1_000_000_000) * unfilled_amount < get_quote_balance(sender), OrderbookError::InsufficientQuoteAmountError);
           
            let mut i = 0;
            let mut resting_orders = asks_stroage_to_vec();
            while i < resting_orders.len() {
                let match_order = resting_orders.get(i).unwrap();
                if match_order.price > price {
                    break
                }

                let mut match_ = Match::new();
                if unfilled_amount >= match_order.amount {
                    unfilled_amount -= match_order.amount;
                    match_.fill_qty = match_order.amount;
                    match_.maker_order_removed = true;
                    match_.maker_account = match_order.address;
                    match_.fill_price = match_order.price;
                    match_.maker_order_index = i;
                } else {
                    match_.fill_qty = unfilled_amount;
                    unfilled_amount = 0;
                    match_.maker_order_removed = false;
                    match_.maker_account = match_order.address;
                    match_.fill_price = match_order.price;
                    match_.maker_order_index = i;
                }

                matches.push(match_);

                if unfilled_amount == 0 {
                    break
                }
                i += 1;
            }

            let mut i = 0;
            while i < matches.len() {
                let m = matches.get(i).unwrap();
                if m.maker_order_removed {
                    storage.asks.remove(0);
                }else {
                    let mut order = storage.asks.get(0).unwrap();
                    order.amount = order.amount - m.fill_qty;
                    order.filled = order.filled + m.fill_qty;
                    storage.asks.insert(1, order);
                    storage.asks.remove(0);
                }
                i+=1;
            }

            if unfilled_amount > 0 {
                // place on orderobok 
                save_bid_order(OpenLimitOrder {
                    price: req.price,
                    amount: unfilled_amount,
                    seq: storage.seq,
                    address: get_msg_sender_address_or_panic(),
                    filled: 0,
                    side: side,
                    timestamp: timestamp(),
                })
            }
        } else {
            // sell
            // check base amount
            require(get_base_balance(sender) >= unfilled_amount, OrderbookError::InsufficientBaseAmountError);

            let mut i = 0;
            let mut resting_orders = bids_stroage_to_vec();
            while i < resting_orders.len() {
                let match_order = resting_orders.get(i).unwrap();
                if match_order.price < price {
                    break
                }
                let mut match_ = Match::new();
                if unfilled_amount >= match_order.amount {
                    unfilled_amount -= match_order.amount;
                    match_.fill_qty = match_order.amount;
                    match_.maker_order_removed = true;
                    match_.maker_account = match_order.address;
                    match_.fill_price = match_order.price;
                    match_.maker_order_index = i;
                } else {
                    match_.fill_qty = unfilled_amount;
                    unfilled_amount = 0;
                    match_.maker_order_removed = false;
                    match_.maker_account = match_order.address;
                    match_.fill_price = match_order.price;
                    match_.maker_order_index = i;
                }
                matches.push(match_);
                if unfilled_amount == 0 {
                    break
                }
                i += 1;
            }

            let mut i = 0;
            while i < matches.len() {
                let m = matches.get(i).unwrap();
                if m.maker_order_removed {
                    storage.bids.remove(0);
                }else{
                    let mut order = storage.bids.get(0).unwrap();
                    order.amount = order.amount - m.fill_qty;
                    order.filled = order.filled + m.fill_qty;
                    storage.bids.insert(1, order);
                    storage.bids.remove(0);
                }
                i+=1;
            }

            if unfilled_amount > 0 {
                // place on orderobok 
                save_ask_order(OpenLimitOrder {
                    price: req.price,
                    amount: unfilled_amount,
                    seq: storage.seq,
                    address: get_msg_sender_address_or_panic(),
                    filled: 0,
                    side: side,
                    timestamp: timestamp(),
                })
            }
        }
        
        settle(matches, side);
        matches
    }

    #[storage(read)]
    fn orderbook(side: u64) -> Vec<OpenLimitOrder> {
        let mut order_list: Vec<OpenLimitOrder> = Vec::new();
        if (side == 0) {
            let mut i = 0;
            while i < storage.bids.len() {
                order_list.push(storage.bids.get(i).unwrap());
                i += 1;
            }
        } else {
            let mut i = 0;
            while i < storage.asks.len() {
                order_list.push(storage.asks.get(i).unwrap());
                i += 1;
            }
        }

        order_list
    }

    #[storage(write, read), payable]
    fn deposit() {
        let amount = msg_amount();
        let asset_id = msg_asset_id();
        let address = get_msg_sender_address_or_panic();

        let key = (address, asset_id);
        let amt = storage.deposits.get(key);
        if  amt.is_some(){
            let amount = amount + amt.unwrap();
            storage.deposits.insert(key, amount);
        }else {
            storage.deposits.insert(key, amount);
        }
    }

    #[storage(write, read)]
    fn withdraw(asset_id: ContractId, amount: u64) {
        let address = get_msg_sender_address_or_panic();
        let balance = balance_internal(address, asset_id);
        // require(balance >= amount, Error::InsufficientBalance);
        transfer_to_address(amount, asset_id, address);
        let amount_after = balance - amount;
        let key = (address, asset_id);
        if amount_after > 0 {
            storage.deposits.insert(key, amount_after);
        } else {
            storage.deposits.insert(key, 0);
        }
    }

    #[storage(read)]
    fn balance(address: Address, asset_id: ContractId) -> u64 {
        balance_internal(address, asset_id)
    }

    #[storage(read)]
    fn recent_trades(num: u64) -> Vec<Trade> {
        let mut trades: Vec<Trade> = Vec::new();
        let mut i = 0;
        while i < storage.trades.len() {
            let trade = storage.trades.get(i).unwrap();
            trades.push(trade);
            i += 1;
        }
        trades
    }
}

#[storage(read)]
pub fn asks_stroage_to_vec() -> Vec<OpenLimitOrder> {
    let mut order_list: Vec<OpenLimitOrder> = Vec::new();
    let mut i = 0;
    while i < storage.asks.len() {
        let order = storage.asks.get(i).unwrap();
        order_list.push(order);
        i += 1;
    }

    order_list
}

#[storage(read)]
pub fn bids_stroage_to_vec() -> Vec<OpenLimitOrder> {
    let mut order_list: Vec<OpenLimitOrder> = Vec::new();
    let mut i = 0;
    while i < storage.bids.len() {
        let order = storage.bids.get(i).unwrap();
        order_list.push(order);
        i += 1;
    }

    order_list
}

#[storage(read, write)]
fn save_bid_order(order: OpenLimitOrder) {
    let order_list = bids_stroage_to_vec();
    let price = order.price;
    let loc = find_bid_order_loc(price, order.seq, order_list);
    insert_bid_order(loc, order);
}

#[storage(read, write)]
fn save_ask_order(order: OpenLimitOrder) {
    let order_list = asks_stroage_to_vec();
    let price = order.price;
    let loc = find_ask_order_loc(price, order.seq, order_list);
    insert_ask_order(loc, order);
}

// #[storage(read, write)]
fn find_ask_order_loc(price: u64, seq: u64, order_list: Vec<OpenLimitOrder>) -> u64 {
    binary_search_ask_order(order_list, price)
}

// #[storage(read, write)]
fn find_bid_order_loc(price: u64, seq: u64, order_list: Vec<OpenLimitOrder>) -> u64 {
    binary_search_bid_order(order_list, price)
}

pub fn binary_search_bid_order(slice: Vec<OpenLimitOrder>, price: u64) -> u64 {
    let mut size = slice.len();
    if size == 0 {
        0
    } else {
        let mut base: u64 = 0;
        while size > 1 {
            let half = size / 2;
            let mid = base + half;
            base = if slice.get(mid).unwrap().price <= price {
                base
            } else {
                mid
            };
            size -= half;
        }

        let mut diff:u64 = 0;
        if slice.get(base).unwrap().price >= price {
            diff = 1;
        }

        base + diff
    }
}

pub fn binary_search_ask_order(slice: Vec<OpenLimitOrder>, price: u64) -> u64 {
    let mut size = slice.len();
    if size == 0 {
        0
    } else {
        let mut base: u64 = 0;
        while size > 1 {
            let half = size / 2;
            let mid = base + half;
            base = if slice.get(mid).unwrap().price > price {
                base
            } else {
                mid
            };
            size -= half;
        }

        let mut diff:u64 = 0;
        if slice.get(base).unwrap().price < price {
            diff = 1;
        }

        base + diff
    }
}

#[storage(read)]
fn balance_internal(address: Address, asset_id: ContractId) -> u64 {
    let key = (address, asset_id);
    let val = storage.deposits.get(key);
    if val.is_some() {
        val.unwrap()
    }else{
        0
    }
}

#[storage(read, write)]
#[inline(never)]
pub fn settle(matches:Vec<Match>, side:u64){
    if side == 0 {
        settle_buy(matches);
    }else{
        settle_sell(matches);
    }
}

#[storage(read, write)]
pub fn settle_buy(matches:Vec<Match>) {
    let taker = get_msg_sender_address_or_panic();
    let mut i =0 ;
    while i < matches.len() {
        // transfer base asset
        let m = matches.get(i).unwrap();
        let maker_base_balance = get_base_balance(m.maker_account);
        let maker_base_key = (m.maker_account, storage.base_asset_id);
        storage.deposits.insert(maker_base_key, maker_base_balance - m.fill_qty);

        let taker_base_balance = get_base_balance(taker);
        let taker_base_key = (taker, storage.base_asset_id);
        storage.deposits.insert(taker_base_key, taker_base_balance + m.fill_qty);

        // transfer quote asset
        let quote_amount =  m.fill_qty * (m.fill_price / 1_000_000_000);
        let maker_quote_key = (m.maker_account, storage.quote_asset_id);
        let maker_quote_balance = get_quote_balance(m.maker_account);
        storage.deposits.insert(maker_quote_key, maker_quote_balance + quote_amount);

        let taker_quote_balance = get_quote_balance(taker);
        let taker_quote_key = (taker, storage.quote_asset_id);
        storage.deposits.insert(taker_quote_key, taker_quote_balance - quote_amount);
        i += 1;

        storage.trades.push(Trade{
            maker: m.maker_account,
            taker: taker,
            price: m.fill_price,
            amount: m.fill_qty,
            timestamp: timestamp(),
            side: 0,
        })
    }
}

#[storage(read, write)]
pub fn settle_sell(matches:Vec<Match>) {
    let taker = get_msg_sender_address_or_panic();
    let mut i =0 ;
    while i < matches.len() {
        // transfer base asset
        let m = matches.get(i).unwrap();
        let maker_base_balance = get_base_balance(m.maker_account);
        let maekr_base_key = (m.maker_account, storage.base_asset_id);
        storage.deposits.insert(maekr_base_key, maker_base_balance + m.fill_qty);

        let taker_base_balance = get_base_balance(taker);
        let taker_base_key = (taker, storage.base_asset_id);
        storage.deposits.insert(taker_base_key, taker_base_balance - m.fill_qty);

        // transfer quote asset
        let maker_quote_key = (m.maker_account, storage.quote_asset_id);
        let maker_quote_balance = get_quote_balance(m.maker_account);
        storage.deposits.insert(maker_quote_key, maker_quote_balance - m.fill_qty * (m.fill_price / 1_000_000_000));

        let taker_quote_balance = get_quote_balance(taker);
        let taker_quote_key = (taker, storage.quote_asset_id);
        storage.deposits.insert(taker_quote_key, taker_quote_balance + m.fill_qty * (m.fill_price / 1_000_000_000));
        i += 1;

        storage.trades.push(Trade{
            maker: m.maker_account,
            taker: taker,
            price: m.fill_price,
            amount: m.fill_qty,
            timestamp: timestamp(),
            side: 1,
        })
    }
}

#[storage(read)]
pub fn get_base_balance(address:Address) -> u64 {
    balance_internal(address, storage.base_asset_id)
}

#[storage(read)]
pub fn get_quote_balance(address:Address) -> u64{
    balance_internal(address, storage.quote_asset_id)
}

#[storage(read, write)]
#[inline(never)]
fn insert_bid_order(location:u64, new_order:OpenLimitOrder){
    if storage.bids.len()>0{
        let val = storage.bids.get(location);
        if val.is_some() {
            let order = val.unwrap();
            storage.bids.insert(location+1, new_order);
            storage.bids.insert(location+2, order);
            storage.bids.remove(location);
        }else {
            storage.bids.push(new_order);
        }
    }else{
        storage.bids.insert(location, new_order);
    }
}

#[storage(read, write)]
#[inline(never)]
fn insert_ask_order(location:u64, new_order:OpenLimitOrder){
    if storage.asks.len()>0{
        let val = storage.asks.get(location);
        if val.is_some() {
            let order = val.unwrap();
            storage.asks.insert(location+1, new_order);
            storage.asks.insert(location+2, order);
            storage.asks.remove(location);
        }else {
            storage.asks.push(new_order);
        }
    }else {
        storage.asks.insert(location, new_order);
    }
}

fn get_msg_sender_address_or_panic() -> Address {
    let sender = msg_sender();
    if let Identity::Address(address) = sender.unwrap() {
        address
    } else {
        revert(0);
    }
}