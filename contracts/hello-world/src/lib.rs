#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, Address, Env, Symbol, Vec,
};

const AUCT_NS: Symbol = symbol_short!("SBID");

// Auction state
#[contracttype]
#[derive(Clone)]
pub enum AuctionStatus {
    Open,
    Closed,
}

#[contracttype]
#[derive(Clone)]
pub struct Bid {
    pub bidder: Address,
    pub amount: i128,
}

#[contracttype]
#[derive(Clone)]
pub struct Auction {
    pub auction_id: u64,
    pub seller: Address,
    pub status: AuctionStatus,
    pub min_bid: i128,
    pub highest_bid: i128,
    pub highest_bidder: Option<Address>,
}

#[contract]
pub struct SafeBidMarket;

#[contractimpl]
impl SafeBidMarket {
    // Seller opens a new auction with a minimum bid
    pub fn open_auction(env: Env, auction_id: u64, seller: Address, min_bid: i128) {
        if min_bid <= 0 {
            panic!("min_bid must be positive");
        }

        let inst = env.storage().instance();
        let key = Self::auction_key(auction_id);

        if inst.has(&key) {
            panic!("auction id exists");
        }

        let a = Auction {
            auction_id,
            seller,
            status: AuctionStatus::Open,
            min_bid,
            highest_bid: 0,
            highest_bidder: None,
        };

        inst.set(&key, &a);

        let bids_key = Self::bids_key(auction_id);
        let empty: Vec<Bid> = Vec::new(&env);
        inst.set(&bids_key, &empty);
    }

    // Place a bid; must exceed current highest and min_bid
    pub fn place_bid(env: Env, auction_id: u64, bidder: Address, amount: i128) {
        if amount <= 0 {
            panic!("amount must be positive");
        }

        let inst = env.storage().instance();
        let a_key = Self::auction_key(auction_id);
        let b_key = Self::bids_key(auction_id);

        let mut a: Auction =
            inst.get(&a_key).unwrap_or_else(|| panic!("auction not found"));

        if let AuctionStatus::Open = a.status {
        } else {
            panic!("auction closed");
        }

        if amount < a.min_bid || amount <= a.highest_bid {
            panic!("bid too low");
        }

        let mut bids: Vec<Bid> =
            inst.get(&b_key).unwrap_or_else(|| panic!("bids vector missing"));

        let bid = Bid { bidder: bidder.clone(), amount };
        bids.push_back(bid);

        a.highest_bid = amount;
        a.highest_bidder = Some(bidder);

        inst.set(&a_key, &a);
        inst.set(&b_key, &bids);
    }

    // Close auction; returns highest bid and winner (if any)
    pub fn close_auction(env: Env, auction_id: u64, caller: Address) -> Option<(Address, i128)> {
        let inst = env.storage().instance();
        let a_key = Self::auction_key(auction_id);

        let mut a: Auction =
            inst.get(&a_key).unwrap_or_else(|| panic!("auction not found"));

        if a.seller != caller {
            panic!("only seller can close");
        }

        if let AuctionStatus::Open = a.status {
        } else {
            panic!("already closed");
        }

        a.status = AuctionStatus::Closed;
        inst.set(&a_key, &a);

        match a.highest_bidder {
            Some(winner) if a.highest_bid > 0 => Some((winner, a.highest_bid)),
            _ => None,
        }
    }

    // View helpers
    pub fn get_auction(env: Env, auction_id: u64) -> Option<Auction> {
        let inst = env.storage().instance();
        let key = Self::auction_key(auction_id);
        inst.get(&key)
    }

    pub fn get_bids(env: Env, auction_id: u64) -> Option<Vec<Bid>> {
        let inst = env.storage().instance();
        let key = Self::bids_key(auction_id);
        inst.get(&key)
    }

    // Storage keys
    fn auction_key(id: u64) -> (Symbol, u64) {
        (AUCT_NS, id)
    }

    fn bids_key(id: u64) -> (Symbol, Symbol, u64) {
        (AUCT_NS, symbol_short!("BIDS"), id)
    }
}
