use crate::utils::init;
use near_sdk::json_types::U128;
use near_sdk_sim::{call, to_yocto, view};

#[test]
fn simulate_total_supply() {
    let initial_balance = to_yocto("100");
    let (_, ft, _, _, _) = init(initial_balance);
    let total_supply: U128 = view!(ft.ft_total_supply()).unwrap_json();
    assert_eq!(initial_balance, total_supply.0);
}

#[test]
fn simulate_simple_transfer() {
    let transfer_amount = to_yocto("100");
    let initial_balance = to_yocto("100000");
    let (root, ft, _, _, rick) = init(initial_balance);

    // Transfer from root to rick.
    call!(
        root,
        ft.ft_transfer(rick.account_id(), transfer_amount.into(), None),
        deposit = 1
    )
    .assert_success();
    let root_balance: U128 = view!(ft.ft_balance_of(root.account_id())).unwrap_json();
    let rick_balance: U128 = view!(ft.ft_balance_of(rick.account_id())).unwrap_json();
    assert_eq!(initial_balance - transfer_amount, root_balance.0);
    assert_eq!(transfer_amount, rick_balance.0);
}
