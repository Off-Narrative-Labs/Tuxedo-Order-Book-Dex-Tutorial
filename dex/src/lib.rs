//! An Order Book Decentralized Exchange.
//! 
//! Allows users to place trade orders offering a certain amount of
//! one token asking a certain amount of another token in exchange.
//! 
//! Also allows matching sets of compatible orders together.
//! Orders can be matched as long as every ask is fulfilled.
//! 
//! This piece is instantiable and parameterized in two tokens.
//! If you want multiple trading pairs, then you will need multiple
//! instances of this piece.

#![cfg_attr(not(feature = "std"), no_std)]

use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::transaction_validity::TransactionPriority;
use sp_std::prelude::*;
use tuxedo_core::{
    Verifier,
    dynamic_typing::{DynamicallyTypedData, DynamicTypingError, UtxoData},
    ensure,
    traits::Cash,
    SimpleConstraintChecker,
    support_macros::{CloneNoBound, DebugNoBound, DefaultNoBound},
};

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone, TypeInfo)]
/// An order in the order book represents a binding collateralized
/// offer to make a trade.
///
/// The user who opens this order must put up a corresponding amount of
/// token A. This order can be matched with other orders so long as
/// the ask amount of token B may be paid to this user.
///
/// When a match is made, the payment token will be protected with the
/// verifier contained in this order.
pub struct Order<V: Verifier> {
    /// The amount of token A in this order
    pub offer_amount: u128,
    /// The amount of token B in this order
    pub ask_amount: u128,
    /// The verifier that will protect the payout coin
    /// in the event of a successful match.
    pub payout_verifier: V,
}

impl<V: Verifier> UtxoData for Order<V> {
    const TYPE_ID: [u8; 4] = *b"ordr";
}


// TODO Error Type


// TODO MakeOrder SimpleConstraintChecker


// TODO MatchOrder ConstraintChecker
