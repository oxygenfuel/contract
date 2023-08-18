library;

pub struct OpenLimitOrder{
    price: u64,
    amount: u64,
    seq: u64,
    address: Address,
    filled: u64,
    side: u64,
    timestamp: u64,
}

pub struct NewOrder {
    seq:u64,
    price:u64,
    amount:u64,
}

pub const ZERO_B256 = 0x0000000000000000000000000000000000000000000000000000000000000000;

pub struct Match {
    maker_order_index: u64,
    maker_account: Address,
    fill_qty: u64,
    fill_price: u64,
    maker_order_removed: bool,
}

impl Match {
    pub fn new() -> Self {
        Match{
            maker_order_index: 0,
            maker_account:Address {value: ZERO_B256},
            fill_qty: 0,
            fill_price: 0,
            maker_order_removed: false,
        } 
    }
}

pub struct MatchResult {
   m: Match,
}

pub struct Trade {
    maker: Address,
    taker: Address,
    price: u64,
    amount: u64,
    timestamp: u64,
    side: u64,
}