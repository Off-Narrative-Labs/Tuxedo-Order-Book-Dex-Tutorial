//! Unit tests for the Dex piece

use super::*;
use tuxedo_core::verifier::TestVerifier;
use money::Coin;

/// An simple dex config to use in unit tests.
struct TestConfig;
impl DexConfig for TestConfig {
    type Verifier = TestVerifier;
    type A = Coin<0>;
    type B = Coin<1>;
}

/// A concrete `Order` type. It uses the test config above.
type TestOrder = Order<TestConfig>;

/// A concrete `MakeOrder` constraint checker. It uses the test config above.
type MakeTestOrder = MakeOrder<TestConfig>;

#[test]
fn summing_two_coins_for_collateral_works() {
    let order = TestOrder {
        offer_amount: 100,
        ask_amount: 150,
        payout_verifier: TestVerifier { verifies: true },
        _ph_data: Default::default(),
    };

    let first_coin = Coin::<0>(40);
    let second_coin = Coin::<0>(60);

    let result = MakeTestOrder::default().check(
        &vec![first_coin.into(), second_coin.into()],
        &vec![order.into()],
    );
    assert!(result.is_ok());
}

#[test]
fn making_order_with_inputs_and_outputs_reversed_fails() {
    let order = TestOrder {
        offer_amount: 100,
        ask_amount: 150,
        payout_verifier: TestVerifier { verifies: true },
        _ph_data: Default::default(),
    };

    let first_coin = Coin::<0>(40);
    let second_coin = Coin::<0>(60);

    let result = MakeTestOrder::default().check(
        &vec![order.into()],
        &vec![first_coin.into(), second_coin.into()],
    );

    assert_eq!(result, Err(DexError::TooManyOutputsWhenMakingOrder));
}