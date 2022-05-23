use crate::utils::{init, meta_a, meta_b, AMM_ID};
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

#[test]
fn send_tokens_to_amm_and_withdraw() {
    let transfer_to_rick_amount_a = 20_000_u128;
    let transfer_to_amm_amount_a = 10_000_u128;
    let transfer_to_rick_amount_b = 26_000_u128;
    let transfer_to_amm_amount_b = 12_000_u128;
    let initial_balance = 280_000_u128;
    let (root, ft_a, ft_b, amm, rick) = init(initial_balance);

    // Transfer from root to rick at FTA.
    call!(
        root,
        ft_a.ft_transfer(rick.account_id(), transfer_to_rick_amount_a.into(), None),
        deposit = 1
    )
    .assert_success();

    // Transfer from root to rick at FTB.
    call!(
        root,
        ft_b.ft_transfer(rick.account_id(), transfer_to_rick_amount_b.into(), None),
        deposit = 1
    )
    .assert_success();

    // Open storage in AMM for Rick
    call!(
        root,
        amm.storage_deposit(ft_a.account_id(), rick.account_id(), None),
        deposit = near_sdk::env::storage_byte_cost() * 125
    )
    .assert_success();
    call!(
        root,
        amm.storage_deposit(ft_b.account_id(), rick.account_id(), None),
        deposit = near_sdk::env::storage_byte_cost() * 125
    )
    .assert_success();

    // Send A-tokens to AMM
    call!(
        rick,
        ft_a.ft_transfer_call(
            AMM_ID.parse().unwrap(),
            transfer_to_amm_amount_a.into(),
            None,
            "".to_string()
        ),
        deposit = 1
    )
    .assert_success();

    // Send B-tokens to AMM
    call!(
        rick,
        ft_b.ft_transfer_call(
            AMM_ID.parse().unwrap(),
            transfer_to_amm_amount_b.into(),
            None,
            "".to_string()
        ),
        deposit = 1
    )
    .assert_success();

    // Check FTA root balance without sent tokens
    let root_balance_a: U128 = view!(ft_a.ft_balance_of(root.account_id())).unwrap_json();
    assert_eq!(
        initial_balance - transfer_to_rick_amount_a,
        root_balance_a.0
    );
    // Check FTB root balance without sent tokens
    let root_balance_b: U128 = view!(ft_b.ft_balance_of(root.account_id())).unwrap_json();
    assert_eq!(
        initial_balance - transfer_to_rick_amount_b,
        root_balance_b.0
    );

    // Check Rick balance in FTA
    let rick_balance_ft_a: U128 = view!(ft_a.ft_balance_of(rick.account_id())).unwrap_json();
    let rick_balance_ft_b: U128 = view!(ft_b.ft_balance_of(rick.account_id())).unwrap_json();
    assert_eq!(
        transfer_to_rick_amount_a - transfer_to_amm_amount_a,
        rick_balance_ft_a.0
    );
    assert_eq!(
        transfer_to_rick_amount_b - transfer_to_amm_amount_b,
        rick_balance_ft_b.0
    );

    // Check Rick Balance in AMM
    let rick_balance_amm_a: U128 =
        view!(amm.ft_balance_of(ft_a.account_id(), rick.account_id())).unwrap_json();
    let rick_balance_amm_b: U128 =
        view!(amm.ft_balance_of(ft_b.account_id(), rick.account_id())).unwrap_json();
    assert_eq!(transfer_to_amm_amount_a, rick_balance_amm_a.0);
    assert_eq!(transfer_to_amm_amount_b, rick_balance_amm_b.0);

    // Withdraw all tokens back
    call!(
        rick,
        amm.withdraw_tokens(ft_a.account_id(), rick_balance_amm_a),
        gas = 300000000000000
    )
    .assert_success();

    call!(
        rick,
        amm.withdraw_tokens(ft_b.account_id(), rick_balance_amm_b),
        gas = 300000000000000
    )
    .assert_success();

    // Check Rick balance in FTA & FTB
    let rick_balance_ft_a: U128 = view!(ft_a.ft_balance_of(rick.account_id())).unwrap_json();
    let rick_balance_ft_b: U128 = view!(ft_b.ft_balance_of(rick.account_id())).unwrap_json();
    assert_eq!(rick_balance_ft_a.0, transfer_to_rick_amount_a);
    assert_eq!(rick_balance_ft_b.0, transfer_to_rick_amount_b);

    // Check Rick Balance in AMM (must be zero)
    let rick_balance_amm_a: U128 =
        view!(amm.ft_balance_of(ft_a.account_id(), rick.account_id())).unwrap_json();
    let rick_balance_amm_b: U128 =
        view!(amm.ft_balance_of(ft_b.account_id(), rick.account_id())).unwrap_json();
    assert_eq!(rick_balance_amm_a.0, 0);
    assert_eq!(rick_balance_amm_b.0, 0);
}

#[test]
fn test_add_and_exclude_pool() {
    let transfer_to_rick_amount_a = 20_000_u128;
    let transfer_to_amm_amount_a = 10_000_u128;
    let transfer_to_rick_amount_b = 26_000_u128;
    let transfer_to_amm_amount_b = 12_000_u128;
    let initial_balance = 280_000_u128;
    let (root, ft_a, ft_b, amm, rick) = init(initial_balance);

    // Transfer from root to rick at FTA.
    call!(
        root,
        ft_a.ft_transfer(rick.account_id(), transfer_to_rick_amount_a.into(), None),
        deposit = 1
    )
    .assert_success();

    // Transfer from root to rick at FTB.
    call!(
        root,
        ft_b.ft_transfer(rick.account_id(), transfer_to_rick_amount_b.into(), None),
        deposit = 1
    )
    .assert_success();

    // Open storage in AMM for Rick
    call!(
        root,
        amm.storage_deposit(ft_a.account_id(), rick.account_id(), None),
        deposit = near_sdk::env::storage_byte_cost() * 125
    )
    .assert_success();
    call!(
        root,
        amm.storage_deposit(ft_b.account_id(), rick.account_id(), None),
        deposit = near_sdk::env::storage_byte_cost() * 125
    )
    .assert_success();

    // Send A-tokens to AMM
    call!(
        rick,
        ft_a.ft_transfer_call(
            AMM_ID.parse().unwrap(),
            transfer_to_amm_amount_a.into(),
            None,
            "".to_string()
        ),
        deposit = 1
    )
    .assert_success();

    // Send B-tokens to AMM
    call!(
        rick,
        ft_b.ft_transfer_call(
            AMM_ID.parse().unwrap(),
            transfer_to_amm_amount_b.into(),
            None,
            "".to_string()
        ),
        deposit = 1
    )
    .assert_success();
    call!(
        root,
        amm.storage_deposit(amm.account_id(), rick.account_id(), None),
        deposit = near_sdk::env::storage_byte_cost() * 250
    )
    .assert_success();

    // Add tokens to pool
    let send_a_tokens_to_pool = 8_000_u128;
    let send_b_tokens_to_pool = 4_000_u128;
    let rick_balance_amm_a_before: U128 =
        view!(amm.ft_balance_of(ft_a.account_id(), rick.account_id())).unwrap_json();
    let rick_balance_amm_b_before: U128 =
        view!(amm.ft_balance_of(ft_b.account_id(), rick.account_id())).unwrap_json();

    // init metadata tokens A & B
    call!(root, amm.set_metadata_a(meta_a())).assert_success();
    call!(root, amm.set_metadata_b(meta_b())).assert_success();

    call!(
        rick,
        amm.add_token_to_pool(ft_a.account_id(), send_a_tokens_to_pool.into(), None)
    )
    .assert_success();
    call!(
        rick,
        amm.add_token_to_pool(ft_b.account_id(), send_b_tokens_to_pool.into(), None)
    )
    .assert_success();

    let rick_balance_amm_a: U128 =
        view!(amm.ft_balance_of(ft_a.account_id(), rick.account_id())).unwrap_json();
    let rick_balance_amm_b: U128 =
        view!(amm.ft_balance_of(ft_b.account_id(), rick.account_id())).unwrap_json();
    let rick_balance_amm_amm: U128 =
        view!(amm.ft_balance_of(amm.account_id(), rick.account_id())).unwrap_json();
    let owner_balance_amm_a: U128 =
        view!(amm.ft_balance_of(ft_a.account_id(), amm.account_id())).unwrap_json();
    let owner_balance_amm_b: U128 =
        view!(amm.ft_balance_of(ft_b.account_id(), amm.account_id())).unwrap_json();

    assert_eq!(
        rick_balance_amm_amm.0,
        send_a_tokens_to_pool + send_b_tokens_to_pool
    );
    assert_eq!(
        rick_balance_amm_a.0,
        rick_balance_amm_a_before.0 - send_a_tokens_to_pool
    );
    assert_eq!(
        rick_balance_amm_b.0,
        rick_balance_amm_b_before.0 - send_b_tokens_to_pool
    );
    assert_eq!(owner_balance_amm_a.0, send_a_tokens_to_pool);
    assert_eq!(owner_balance_amm_b.0, send_b_tokens_to_pool);

    //hatef
    let info : String = view!(amm.contract_info()).unwrap_json();
    assert_eq!(info, "hatef".to_string());
    //hatef

    call!(
        rick,
        amm.exclude_token_from_pool(ft_a.account_id(), send_a_tokens_to_pool.into(), None)
    )
    .assert_success();
    call!(
        rick,
        amm.exclude_token_from_pool(ft_b.account_id(), send_b_tokens_to_pool.into(), None)
    )
    .assert_success();

    let rick_balance_amm_a: U128 =
        view!(amm.ft_balance_of(ft_a.account_id(), rick.account_id())).unwrap_json();
    let rick_balance_amm_b: U128 =
        view!(amm.ft_balance_of(ft_b.account_id(), rick.account_id())).unwrap_json();
    let rick_balance_amm_amm: U128 =
        view!(amm.ft_balance_of(amm.account_id(), rick.account_id())).unwrap_json();
    let owner_balance_amm_a: U128 =
        view!(amm.ft_balance_of(ft_a.account_id(), amm.account_id())).unwrap_json();
    let owner_balance_amm_b: U128 =
        view!(amm.ft_balance_of(ft_b.account_id(), amm.account_id())).unwrap_json();
    assert_eq!(rick_balance_amm_amm.0, 0);
    assert_eq!(rick_balance_amm_a.0, rick_balance_amm_a_before.0);
    assert_eq!(rick_balance_amm_b.0, rick_balance_amm_b_before.0);
    assert_eq!(owner_balance_amm_a.0, 0);
    assert_eq!(owner_balance_amm_b.0, 0);
}

#[test]
fn test_swap_tokens() {
    let transfer_to_rick_amount_a = 100_000_u128;
    let transfer_to_amm_amount_a = 50_000_u128;
    let transfer_to_rick_amount_b = 200_000_u128;
    let transfer_to_amm_amount_b = 100_000_u128;
    let initial_balance = 1_000_000_u128;
    let (root, ft_a, ft_b, amm, rick) = init(initial_balance);

    // Transfer from root to rick at FTA
    call!(
        root,
        ft_a.ft_transfer(rick.account_id(), transfer_to_rick_amount_a.into(), None),
        deposit = 1
    )
    .assert_success();
    call!(
        root,
        ft_b.ft_transfer(rick.account_id(), transfer_to_rick_amount_b.into(), None),
        deposit = 1
    )
    .assert_success();

    // Open storage in AMM for Rick
    call!(
        root,
        amm.storage_deposit(ft_a.account_id(), rick.account_id(), None),
        deposit = near_sdk::env::storage_byte_cost() * 125
    )
    .assert_success();
    call!(
        root,
        amm.storage_deposit(ft_b.account_id(), rick.account_id(), None),
        deposit = near_sdk::env::storage_byte_cost() * 125
    )
    .assert_success();

    // Send A-tokens to AMM
    call!(
        rick,
        ft_a.ft_transfer_call(
            AMM_ID.parse().unwrap(),
            transfer_to_amm_amount_a.into(),
            None,
            "".to_string()
        ),
        deposit = 1
    )
    .assert_success();

    // Send B-tokens to AMM
    call!(
        rick,
        ft_b.ft_transfer_call(
            AMM_ID.parse().unwrap(),
            transfer_to_amm_amount_b.into(),
            None,
            "".to_string()
        ),
        deposit = 1
    )
    .assert_success();
    call!(
        root,
        amm.storage_deposit(amm.account_id(), rick.account_id(), None),
        deposit = near_sdk::env::storage_byte_cost() * 250
    )
    .assert_success();

    // Add tokens to pool
    let send_a_tokens_to_pool = 30_000_u128;
    let send_b_tokens_to_pool = 10_000_u128;
    let rick_balance_amm_a_before: U128 =
        view!(amm.ft_balance_of(ft_a.account_id(), rick.account_id())).unwrap_json();
    let rick_balance_amm_b_before: U128 =
        view!(amm.ft_balance_of(ft_b.account_id(), rick.account_id())).unwrap_json();

    // init metadata tokens A & B
    call!(root, amm.set_metadata_a(meta_a())).assert_success();
    call!(root, amm.set_metadata_b(meta_b())).assert_success();

    call!(
        rick,
        amm.add_token_to_pool(ft_a.account_id(), send_a_tokens_to_pool.into(), None)
    )
    .assert_success();

    call!(
        rick,
        amm.add_token_to_pool(ft_b.account_id(), send_b_tokens_to_pool.into(), None)
    )
    .assert_success();
    let rick_balance_amm_a: U128 =
        view!(amm.ft_balance_of(ft_a.account_id(), rick.account_id())).unwrap_json();
    let rick_balance_amm_b: U128 =
        view!(amm.ft_balance_of(ft_b.account_id(), rick.account_id())).unwrap_json();
    let rick_balance_amm_amm: U128 =
        view!(amm.ft_balance_of(amm.account_id(), rick.account_id())).unwrap_json();
    let owner_balance_amm_a: U128 =
        view!(amm.ft_balance_of(ft_a.account_id(), amm.account_id())).unwrap_json();
    let owner_balance_amm_b: U128 =
        view!(amm.ft_balance_of(ft_b.account_id(), amm.account_id())).unwrap_json();
    assert_eq!(
        rick_balance_amm_amm.0,
        send_a_tokens_to_pool + send_b_tokens_to_pool
    );
    assert_eq!(
        rick_balance_amm_a.0,
        rick_balance_amm_a_before.0 - send_a_tokens_to_pool
    );
    assert_eq!(
        rick_balance_amm_b.0,
        rick_balance_amm_b_before.0 - send_b_tokens_to_pool
    );
    assert_eq!(owner_balance_amm_a.0, send_a_tokens_to_pool);
    assert_eq!(owner_balance_amm_b.0, send_b_tokens_to_pool);

    // Swap tokens
    let sell_token = ft_a.account_id();
    let buy_token = ft_b.account_id();
    let sell_token_amount = 10_000_u128;
    let rick_balance_amm_a_prev: U128 =
        view!(amm.ft_balance_of(ft_a.account_id(), rick.account_id())).unwrap_json();
    let rick_balance_amm_b_prev: U128 =
        view!(amm.ft_balance_of(ft_b.account_id(), rick.account_id())).unwrap_json();
    let rick_balance_amm_amm_prev: U128 =
        view!(amm.ft_balance_of(amm.account_id(), rick.account_id())).unwrap_json();
    let owner_balance_amm_a_prev: U128 =
        view!(amm.ft_balance_of(ft_a.account_id(), amm.account_id())).unwrap_json();
    let owner_balance_amm_b_prev: U128 =
        view!(amm.ft_balance_of(ft_b.account_id(), amm.account_id())).unwrap_json();
    let outcome = call!(
        rick,
        amm.swap(buy_token, sell_token, sell_token_amount.into())
    );
    outcome.assert_success();
    let buy_amount: U128 = outcome.unwrap_json();

    let rick_balance_amm_a: U128 =
        view!(amm.ft_balance_of(ft_a.account_id(), rick.account_id())).unwrap_json();
    let rick_balance_amm_b: U128 =
        view!(amm.ft_balance_of(ft_b.account_id(), rick.account_id())).unwrap_json();
    let rick_balance_amm_amm: U128 =
        view!(amm.ft_balance_of(amm.account_id(), rick.account_id())).unwrap_json();
    let owner_balance_amm_a: U128 =
        view!(amm.ft_balance_of(ft_a.account_id(), amm.account_id())).unwrap_json();
    let owner_balance_amm_b: U128 =
        view!(amm.ft_balance_of(ft_b.account_id(), amm.account_id())).unwrap_json();

    assert_eq!(
        rick_balance_amm_a.0,
        rick_balance_amm_a_prev.0 - sell_token_amount
    );
    assert_eq!(
        rick_balance_amm_b.0,
        rick_balance_amm_b_prev.0 + buy_amount.0
    );
    assert_eq!(rick_balance_amm_amm.0, rick_balance_amm_amm_prev.0);
    assert_eq!(
        owner_balance_amm_a.0,
        owner_balance_amm_a_prev.0 + sell_token_amount
    );
    assert_eq!(
        owner_balance_amm_b.0,
        owner_balance_amm_b_prev.0 - buy_amount.0
    );
}
