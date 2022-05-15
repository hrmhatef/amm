use ft::FTContractContract as FTContract;

use near_contract_standards::fungible_token::metadata::{FungibleTokenMetadata, FT_METADATA_SPEC};
use near_sdk::serde_json::json;
use near_sdk_sim::{deploy, init_simulator, to_yocto, ContractAccount, UserAccount};


pub const FT_ID: &str = "token";

// Load in contract bytes at runtime
near_sdk_sim::lazy_static_include::lazy_static_include_bytes! {
    FT_WASM_BYTES => "res/ft.wasm",
}

// Register the given `user` with FT contract
pub fn register_user(contract_id: &str, user: &near_sdk_sim::UserAccount) {
    user.call(
        contract_id.parse().unwrap(),
        "storage_deposit",
        &json!({
            "account_id": user.account_id()
        })
        .to_string()
        .into_bytes(),
        near_sdk_sim::DEFAULT_GAS / 2,
        near_sdk::env::storage_byte_cost() * 125, // attached deposit
    )
    .assert_success();
}

pub fn init(
    initial_balance: u128,
) -> (
    UserAccount,
    ContractAccount<FTContract>,
    UserAccount,
) {
    let root = init_simulator(None);
    let meta = FungibleTokenMetadata {
        spec: FT_METADATA_SPEC.to_string(),
        name: "FT".to_string(),
        symbol: "FTS".to_string(),
        icon: None,
        reference: None,
        reference_hash: None,
        decimals: 3,
    };

    // Init Token contract
    let token_contract = deploy!(
        contract: FTContract,
        contract_id: FT_ID,
        bytes: &FT_WASM_BYTES,
        signer_account: root,
        init_method: new(
            root.account_id(),
            initial_balance.into(),
            meta.clone()
        )
    );

    let rick = root.create_user("rick".parse().unwrap(), to_yocto("100"));
    register_user(FT_ID, &rick);

    (
        root,
        token_contract,
        rick
    )
}
