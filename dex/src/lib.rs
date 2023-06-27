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

use core::marker::PhantomData;

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
    support_macros::{CloneNoBound, DebugNoBound, DefaultNoBound}, ConstraintChecker, types::Output,
};

/// A Configuration for a Decentralized Exchange.
pub trait DexConfig {
    /// The type of verifiers that can be used in dex payouts.
    /// Typically this should just be the outer verifier type of the runtime.
    type Verifier: Verifier + PartialEq;
    /// The first token in the Dex's pair
    type A: Cash + UtxoData;
    /// The second token in the Dex's pair
    type B: Cash + UtxoData;
}

#[derive(PartialEq, Eq, TypeInfo)]
/// This type represents a configuration that has the tokens swapped from
/// some original configuration.
///
/// When opening orders, we want to allow orders for both sides of the trade.
/// Similarly, when matching orders we have to be sure that the matched orders are on
/// opposite sides of the same trading pair. This type allows us to conveniently
/// express "same pair, but opposite side".
pub struct OppositeSide<T: DexConfig>(PhantomData<T>);

impl<T: DexConfig> DexConfig for OppositeSide<T> {
    type Verifier = T::Verifier;
    type A = T::B;
    type B = T::A;
}

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
pub struct Order<T: DexConfig> {
    /// The amount of token A in this order
    pub offer_amount: u128,
    /// The amount of token B in this order
    pub ask_amount: u128,
    /// The verifier that will protect the payout coin
    /// in the event of a successful match.
    pub payout_verifier: T::Verifier,
    pub _ph_data: PhantomData<T>,
}

impl<T: DexConfig> UtxoData for Order<T> {
    const TYPE_ID: [u8; 4] = [b'$', b'$', T::A::ID, T::B::ID];
}


#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
/// All the things that can go wrong while checking constraints on dex transactions
pub enum DexError {
    /// Some dynamically typed data was not of the expected type
    TypeError,
    /// No outputs were supplied when making an order.
    /// When making an order, exactly one output should be supplied, which is the order.
    OrderMissing,
    /// More than one output was supplied.
    /// When making an order, exactly one output should be supplied, which is the order.
    TooManyOutputsWhenMakingOrder,
    /// The coins provided do not have enough combined value to back the order that you attempted to open.
    NotEnoughCollateralToOpenOrder,
    /// This transaction has a different number of input orders than output payouts.
    /// When matching orders, the number of inputs and outputs must be equal.
    OrderAndPayoutCountDiffer,
    /// This transaction tries to match an order but provides an incorrect payout.
    PayoutDoesNotSatisfyOrder,
    /// The amount of token A supplied by the orders is not enough to match with the demand.
    InsufficientTokenAForMatch,
    /// The amount of token B supplied by the orders is not enough to match with the demand.
    InsufficientTokenBForMatch,
    /// The verifier who is receiving the tokens is not correct one that was specified in the original order.
    VerifierMismatchForTrade,
}

impl From<DynamicTypingError> for DexError {
    fn from(_value: DynamicTypingError) -> Self {
        Self::TypeError
    }
}


#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, PartialEq, Eq, CloneNoBound, DefaultNoBound, DebugNoBound, TypeInfo)]
/// The Constraint checking logic for opening a new order.
///
/// It is generic over the verifier type which can be used to protect
/// matched outputs. Typically this should be set to the runtime's
/// outer verifier type. By the end of the tutorial, it will also be
/// generic over the two coins that will trade in this order book.
pub struct MakeOrder<T: DexConfig>(pub PhantomData<T>);

impl<T: DexConfig> SimpleConstraintChecker for MakeOrder<T> {
    type Error = DexError;

    fn check(
        &self,
        input_data: &[DynamicallyTypedData],
        output_data: &[DynamicallyTypedData],
    ) -> Result<TransactionPriority, Self::Error> {
        // There should be a single order as the output
        ensure!(!output_data.is_empty(), DexError::OrderMissing);
        ensure!(
            output_data.len() == 1,
            DexError::TooManyOutputsWhenMakingOrder
        );
        let order: Order<T> = output_data[0].extract()?;

        // There may be many inputs and they should all be tokens whose combined value
        // equals or exceeds the amount of token they need to provide for this order
        let mut total_collateral = 0;
        for input in input_data {
            let coin: T::A = input.extract()?;
            total_collateral += coin.value();
        }

        ensure!(total_collateral == order.offer_amount, DexError::NotEnoughCollateralToOpenOrder);

        Ok(0)
    }
}


#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, PartialEq, Eq, CloneNoBound, DebugNoBound, DefaultNoBound, TypeInfo)]
/// Constraint checking logic for matching existing open orders against one another
pub struct MatchOrders<T: DexConfig>(pub PhantomData<T>);

impl<T: DexConfig> ConstraintChecker<T::Verifier> for MatchOrders<T> {
    type Error = DexError;

    fn check(
        &self,
        inputs: &[Output<T::Verifier>],
        outputs: &[Output<T::Verifier>],
    ) -> Result<TransactionPriority, Self::Error> {
        // The input and output slices can be arbitrarily long. We
        // assume there is a 1:1 correspondence in the sorting such that
        // the first output is the coin associated with the first order etc.
        ensure!(inputs.len() == outputs.len(), DexError::OrderAndPayoutCountDiffer);

        // Each order will add some tokens to the matching pot
        // and demand some tokens from the matching pot.
        // As we loop through the orders, we will keep track of these totals.
        // After all orders have been inspected, we will make sure the
        // amounts add up.
        let mut total_a_required = 0;
        let mut total_b_required = 0;
        let mut a_so_far = 0;
        let mut b_so_far = 0;

        // As we loop through all the orders, we:
        // 1. Make sure the output properly fills the order's ask
        // 2. Update the totals for checking at the end
        for (input, output) in inputs.iter().zip(outputs) {
            // It could be Order<V, A, B> or Order<V, B, A> so we will try both.
            if let Ok(order) = input.payload.extract::<Order<T>>() {
                a_so_far += order.offer_amount;
                total_b_required += order.ask_amount;

                // Ensure the payout is the right amount
                let payout = output.payload.extract::<T::B>()?;
                ensure!(
                    payout.value() == order.ask_amount,
                    DexError::PayoutDoesNotSatisfyOrder
                );

                // ensure that the payout was given to the right owner
                ensure!(
                    output.verifier == order.payout_verifier,
                    DexError::VerifierMismatchForTrade
                )
            } else if let Ok(order) = input.payload.extract::<Order<OppositeSide<T>>>() {
                b_so_far += order.offer_amount;
                total_a_required += order.ask_amount;

                // Ensure the payout is the right amount
                let payout = output.payload.extract::<T::A>()?;
                ensure!(
                    payout.value() == order.ask_amount,
                    DexError::PayoutDoesNotSatisfyOrder
                );

                // ensure that the payout was given to the right owner
                ensure!(
                    output.verifier == order.payout_verifier,
                    DexError::VerifierMismatchForTrade
                )

            } else {
                // If the order doesn't decode to either side of this pair, then it is not the
                // right type and we return the general type error.
                Err(DexError::TypeError)?
            };
        }

        // Make sure the amounts in the orders actually match and satisfy each other.
        ensure!(
            a_so_far >= total_a_required,
            DexError::InsufficientTokenAForMatch
        );
        ensure!(
            b_so_far >= total_b_required,
            DexError::InsufficientTokenBForMatch
        );

        Ok(0)
    }
}
