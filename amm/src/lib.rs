use std::cmp::max;

use near_contract_standards::fungible_token::core::FungibleTokenCore;
use near_contract_standards::fungible_token::metadata::FungibleTokenMetadata;
use near_contract_standards::fungible_token::receiver::FungibleTokenReceiver;
use near_contract_standards::fungible_token::FungibleToken;
use near_contract_standards::storage_management::{
    StorageBalance, StorageBalanceBounds, StorageManagement,
};
use near_sdk::{
    env, log, near_bindgen, AccountId, Gas, PanicOnDefault, PromiseOrValue, PromiseResult,
};

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::ext_contract;
use near_sdk::json_types::U128;
use near_sdk::serde_json::json;

mod utils;
use utils::{add_decimals, calc_dy, remove_decimals, calc_raito};

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct AMM {
    pub token_amm: FungibleToken,
    pub token_a: (FungibleToken, Option<FungibleTokenMetadata>),
    pub token_b: (FungibleToken, Option<FungibleTokenMetadata>),
    account_id_token_a: AccountId,
    account_id_token_b: AccountId,
}

fn init_token(account_id: &AccountId, prefix: Vec<u8>) -> FungibleToken {
    let mut a = FungibleToken::new(prefix);
    a.internal_register_account(account_id);
    a
}

#[ext_contract(ext_self)]
trait SelfContract {
    fn withdraw_tokens_callback(&mut self, token_name: String, amount: U128);
}

#[ext_contract(ext_ft)]
trait FtContract {
    fn ft_transfer(&self, receiver_id: AccountId, amount: U128, memo: Option<String>);
}

#[near_bindgen]
impl AMM {
    #[init]
    pub fn new(owner_id: AccountId, token_a_id: AccountId, token_b_id: AccountId) -> Self {
        assert!(!env::state_exists(), "Already initialized");
        let ft_a = init_token(&owner_id, b"a".to_vec());
        let ft_b = init_token(&owner_id, b"b".to_vec());
        let token_amm = init_token(&owner_id, b"amm".to_vec());

        Self {
            token_amm,
            token_a: (ft_a, None),
            token_b: (ft_b, None),
            account_id_token_a: token_a_id,
            account_id_token_b: token_b_id,
        }
    }

    pub fn contract_info(&self) -> String {
        self.check_meta();
        let ticker = format!("{}/{}", self.token_a.1.clone().unwrap().symbol, self.token_b.1.clone().unwrap().symbol);
        let token_a_decimals = self.token_a.1.clone().unwrap().decimals;
        let token_b_decimals = self.token_b.1.clone().unwrap().decimals;
        let pool_owner_id = env::current_account_id();
        let a = self.token_a.0.internal_unwrap_balance_of(&pool_owner_id);
        let b = self.token_b.0.internal_unwrap_balance_of(&pool_owner_id);
        let max_decimals = max(token_a_decimals, token_b_decimals);
        let a = add_decimals(a, max_decimals - token_a_decimals);
        let b = add_decimals(b, max_decimals - token_b_decimals);
        let ratio : U128 = calc_raito(a,b, 2).into();
        json!({
            "ticker": ticker,
            "decimals": max(token_a_decimals, token_b_decimals),
            "ratio": ratio
        })
        .to_string()
    }

    pub fn ft_metadata_a(&self) -> String {
        if self.token_a().is_none() {
            panic!("There is no metadata");
        } else {
            let meta = self.token_a.1.clone().unwrap();
            json!({
                "name": meta.name,
                "decimals": meta.decimals
            })
            .to_string()
        }
    }

    pub fn ft_metadata_b(&self) -> String {
        if self.token_b().is_none() {
            panic!("There is no metadata");
        } else {
            let meta = self.token_b.1.clone().unwrap();
            json!({
                "name": meta.name,
                "decimals": meta.decimals
            })
            .to_string()
        }
    }

    pub fn set_metadata_a(&mut self, meta: FungibleTokenMetadata) {
        if self.token_a().is_some() {
            panic!("The token has metadata");
        } else {
            if self.token_b().is_some()
                && meta.name == *self.token_b().unwrap().name
                && meta.symbol == *self.token_b().unwrap().symbol
                && meta.decimals == self.token_b().unwrap().decimals
            {
                panic!("Same token is not acceptable");
            }
            self.token_a.1 = Some(meta);
        }
    }

    pub fn set_metadata_b(&mut self, meta: FungibleTokenMetadata) {
        if self.token_b().is_some() {
            panic!("The token has metadata");
        } else {
            if self.token_a().is_some()
                && meta.name == *self.token_a().unwrap().name
                && meta.symbol == *self.token_a().unwrap().symbol
                && meta.decimals == self.token_a().unwrap().decimals
            {
                panic!("Same token is not acceptable");
            }
            self.token_b.1 = Some(meta);
        }
    }

    pub fn exclude_token_from_pool(
        &mut self,
        token_name: AccountId,
        token_amount: U128,
        memo: Option<String>,
    ) {
        self.check_meta();
        let token = self.get_token_by_name_as_ref(&token_name);
        let predecessor_account_id = env::predecessor_account_id();
        token.0.internal_transfer(
            &env::current_account_id(),
            &predecessor_account_id,
            token_amount.0,
            memo,
        );
        self.token_amm
            .internal_withdraw(&predecessor_account_id, token_amount.0);
    }

    pub fn add_token_to_pool(
        &mut self,
        token_name: AccountId,
        token_amount: U128,
        memo: Option<String>,
    ) {
        self.check_meta();
        let token = self.get_token_by_name_as_ref(&token_name);
        let pool_owner_id = env::current_account_id();
        let payer_id = env::predecessor_account_id();

        token
            .0
            .internal_transfer(&payer_id, &pool_owner_id, token_amount.0, memo);

        let ticker = token.1.clone().unwrap().symbol;
        self.token_amm.internal_deposit(&payer_id, token_amount.0);
        log!(
            "Share {} of token {} has been added to account {}",
            token_amount.0,
            ticker,
            &payer_id
        );
    }

    pub fn swap(
        &mut self,
        buy_token_name: AccountId,
        sell_token_name: AccountId,
        sell_amount: U128,
    ) -> U128 {
        if buy_token_name.eq(&sell_token_name) {
            panic!("Tokens can't be equals")
        }

        self.check_meta();
        let buy_token;
        let sell_token;
        if buy_token_name.eq(&self.account_id_token_a)
            && sell_token_name.eq(&self.account_id_token_b)
        {
            buy_token = &mut self.token_a;
            sell_token = &mut self.token_b;
        } else if buy_token_name.eq(&self.account_id_token_b)
            && sell_token_name.eq(&self.account_id_token_a)
        {
            buy_token = &mut self.token_b;
            sell_token = &mut self.token_a;
        } else {
            panic!("Token not supported");
        }
        let pool_owner_id = env::current_account_id();
        let user_account_id = env::predecessor_account_id();

        // Get current statement of pool
        let x = sell_token.0.internal_unwrap_balance_of(&pool_owner_id);
        let y = buy_token.0.internal_unwrap_balance_of(&pool_owner_id);

        // Send sell_tokens to pool from seller
        sell_token
            .0
            .internal_transfer(&user_account_id, &pool_owner_id, sell_amount.0, None);

        // Convert to the same decimal
        let max_decimals = max(
            buy_token.1.clone().unwrap().decimals,
            sell_token.1.clone().unwrap().decimals,
        );
        let x = add_decimals(x, max_decimals - sell_token.1.clone().unwrap().decimals);
        let y = add_decimals(y, max_decimals - buy_token.1.clone().unwrap().decimals);

        // Calc buy amount
        let buy_amount = calc_dy(x, y, sell_amount.0);

        // Restore decimal
        let buy_amount = remove_decimals(
            buy_amount,
            max_decimals - buy_token.1.clone().unwrap().decimals,
        );

        // Send buy value to user buyer
        buy_token
            .0
            .internal_transfer(&pool_owner_id, &user_account_id, buy_amount, None);

        U128::from(buy_amount)
    }

    #[payable]
    pub fn withdraw_tokens(&mut self, token_name: AccountId, amount: U128) {
        let account_id = env::predecessor_account_id();
        if !self.account_id_token_a.eq(&token_name) && !self.account_id_token_b.eq(&token_name) {
            panic!("Token not supported");
        }
        ext_ft::ft_transfer(
            account_id,
            amount,
            None,
            token_name.clone(),
            1,
            Gas::from(5_000_000_000_000),
        )
        .then(ext_self::withdraw_tokens_callback(
            token_name.to_string(),
            amount,
            env::current_account_id(),
            0,
            Gas::from(5_000_000_000_000),
        ));
    }

    pub fn withdraw_tokens_callback(&mut self, token_name: AccountId, amount: U128) {
        match env::promise_result(0) {
            PromiseResult::NotReady => unreachable!(),
            PromiseResult::Failed => "error!".to_string(),
            PromiseResult::Successful(_) => {
                // Get the user who sent the tokens
                let account_id = env::signer_account_id();
                let token = self.get_token_by_name_as_ref(&token_name);

                // Clear sent tokens value
                token.0.internal_withdraw(&account_id, amount.0);

                "Ok".to_string()
            }
        };
    }

    pub fn ft_balance_of(&self, token_name: AccountId, account_id: AccountId) -> U128 {
        if token_name == env::current_account_id() {
            self.token_amm.ft_balance_of(account_id)
        } else if self.account_id_token_a.eq(&token_name) {
            self.token_a.0.ft_balance_of(account_id)
        } else if self.account_id_token_b.eq(&token_name) {
            self.token_b.0.ft_balance_of(account_id)
        } else {
            panic!("Token not supported")
        }
    }

    #[payable]
    #[allow(dead_code)]
    pub fn storage_deposit(
        &mut self,
        token_name: AccountId,
        account_id: AccountId,
        registration_only: Option<bool>,
    ) {
        if token_name == env::current_account_id() {
            self.token_amm
                .storage_deposit(Some(account_id), registration_only);
        } else {
            let token = self.get_token_by_name_as_ref(&token_name);
            token.0.storage_deposit(Some(account_id), registration_only);
        }
    }

    #[payable]
    #[allow(dead_code)]
    fn storage_withdraw(&mut self, token_name: AccountId, amount: Option<U128>) -> StorageBalance {
        if token_name == env::current_account_id() {
            self.token_amm.storage_withdraw(amount)
        } else {
            let token = self.get_token_by_name_as_ref(&token_name);
            token.0.storage_withdraw(amount)
        }
    }

    #[payable]
    #[allow(dead_code)]
    fn storage_unregister(&mut self, token_name: AccountId, force: Option<bool>) -> bool {
        if token_name == env::current_account_id() {
            if let Some((_, _)) = self.token_amm.internal_storage_unregister(force) {
                return true;
            }
        } else {
            let token = self.get_token_by_name_as_ref(&token_name);
            if let Some((_, _)) = token.0.internal_storage_unregister(force) {
                return true;
            }
        }
        false
    }

    #[allow(dead_code)]
    fn storage_balance_bounds(&self, token_name: AccountId) -> StorageBalanceBounds {
        if token_name == env::current_account_id() {
            self.token_amm.storage_balance_bounds()
        } else {
            let token = self.get_token_by_name(&token_name);
            token.0.storage_balance_bounds()
        }
    }

    #[allow(dead_code)]
    fn storage_balance_of(
        &self,
        token_name: AccountId,
        account_id: AccountId,
    ) -> Option<StorageBalance> {
        if token_name == env::current_account_id() {
            self.token_amm.storage_balance_of(account_id)
        } else {
            let token = self.get_token_by_name(&token_name);
            token.0.storage_balance_of(account_id)
        }
    }

    fn token_a(&self) -> Option<&FungibleTokenMetadata> {
        self.token_a.1.as_ref()
    }

    fn token_b(&self) -> Option<&FungibleTokenMetadata> {
        self.token_b.1.as_ref()
    }

    fn check_meta(&self) {
        if self.token_a().is_none() || self.token_b().is_none() {
            panic!("Please init the metadata of tokens")
        }
    }

    fn get_token_by_name_as_ref(
        &mut self,
        token: &AccountId,
    ) -> &mut (FungibleToken, Option<FungibleTokenMetadata>) {
        if self.account_id_token_a.eq(token) {
            &mut self.token_a
        } else if self.account_id_token_b.eq(token) {
            &mut self.token_b
        } else {
            panic!("Token not supported");
        }
    }

    fn get_token_by_name(
        &self,
        token: &AccountId,
    ) -> &(FungibleToken, Option<FungibleTokenMetadata>) {
        if self.account_id_token_a.eq(token) {
            &self.token_a
        } else if self.account_id_token_b.eq(token) {
            &self.token_b
        } else {
            panic!("Token not supported");
        }
    }
}

#[near_bindgen]
impl FungibleTokenReceiver for AMM {
    fn ft_on_transfer(
        &mut self,
        sender_id: AccountId,
        amount: U128,
        #[allow(unused_variables)] msg: String,
    ) -> PromiseOrValue<U128> {
        let token_name = &env::predecessor_account_id();
        let token = self.get_token_by_name_as_ref(token_name);
        token.0.internal_deposit(&sender_id, amount.0);
        PromiseOrValue::Value(U128::from(0_u128))
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::*;
    use near_contract_standards::fungible_token::metadata::FT_METADATA_SPEC;
    use near_sdk::test_utils::accounts;

    fn meta_a() -> FungibleTokenMetadata {
        FungibleTokenMetadata {
            spec: FT_METADATA_SPEC.to_string(),
            name: "Example NEAR fungible token".to_string(),
            symbol: "FTA".to_string(),
            icon: None,
            reference: None,
            reference_hash: None,
            decimals: 8,
        }
    }
    fn meta_b() -> FungibleTokenMetadata {
        FungibleTokenMetadata {
            spec: FT_METADATA_SPEC.to_string(),
            name: "Example NEAR fungible token".to_string(),
            symbol: "FTB".to_string(),
            icon: None,
            reference: None,
            reference_hash: None,
            decimals: 8,
        }
    }

    #[test]
    fn test_init() {
        let rick_id = accounts(1);
        let token_a = accounts(2);
        let token_b = accounts(3);
        let mut amm = AMM::new(rick_id.clone(), token_a.clone(), token_b.clone());
        amm.set_metadata_a(meta_a());
        amm.set_metadata_b(meta_b());

        assert_eq!(
            amm.ft_metadata_a(),
            json!({
                "name": "Example NEAR fungible token".to_string(),
                "decimals": 8
            })
            .to_string()
        );
    }

    #[test]
    #[should_panic(expected = "There is no metadata")]
    fn test_metadata_a() {
        let rick_id = accounts(1);
        let token_a = accounts(2);
        let token_b = accounts(3);
        let amm = AMM::new(rick_id.clone(), token_a.clone(), token_b.clone());
        amm.ft_metadata_a();
    }

    #[test]
    #[should_panic(expected = "There is no metadata")]
    fn test_metadata_b() {
        let rick_id = accounts(1);
        let token_a = accounts(2);
        let token_b = accounts(3);
        let amm = AMM::new(rick_id.clone(), token_a.clone(), token_b.clone());
        amm.ft_metadata_b();
    }

    #[test]
    #[should_panic(expected = "The token has metadata")]
    fn test_set_metadata_a() {
        let rick_id = accounts(1);
        let token_a = accounts(2);
        let token_b = accounts(3);
        let mut amm = AMM::new(rick_id.clone(), token_a.clone(), token_b.clone());
        amm.set_metadata_a(meta_a());
        amm.set_metadata_a(meta_a());
    }

    #[test]
    #[should_panic(expected = "The token has metadata")]
    fn test_set_metadata_b() {
        let rick_id = accounts(1);
        let token_a = accounts(2);
        let token_b = accounts(3);
        let mut amm = AMM::new(rick_id.clone(), token_a.clone(), token_b.clone());
        amm.set_metadata_b(meta_b());
        amm.set_metadata_b(meta_b());
    }

    #[test]
    #[should_panic(expected = "Same token is not acceptable")]
    fn test_init_same_token_a_and_b() {
        let rick_id = accounts(1);
        let token_a = accounts(2);
        let token_b = accounts(3);
        let mut amm = AMM::new(rick_id, token_a, token_b);
        amm.set_metadata_a(meta_a());
        amm.set_metadata_b(meta_a());
    }

    #[test]
    #[should_panic(expected = "Same token is not acceptable")]
    fn test_init_same_token_b_and_a() {
        let rick_id = accounts(1);
        let token_a = accounts(2);
        let token_b = accounts(3);
        let mut amm = AMM::new(rick_id, token_a, token_b);
        amm.set_metadata_b(meta_b());
        amm.set_metadata_a(meta_b());
    }

    #[test]
    #[should_panic(expected = "Token not supported")]
    fn test_add_token_to_pool_zombie_token() {
        let owner = accounts(1);
        let token_rick = accounts(2);
        let token_morty = accounts(3);
        let token_zombie = accounts(4);
        let amount = 10_000_u128;
        let mut amm = AMM::new(owner, token_rick, token_morty);
        amm.set_metadata_a(meta_a());
        amm.set_metadata_b(meta_b());
        amm.add_token_to_pool(token_zombie, amount.into(), None);
    }

    #[test]
    #[should_panic(expected = "Please init the metadata of tokens")]
    fn test_add_token_to_pool_not_init_tokens() {
        let owner = accounts(1);
        let token_rick = accounts(2);
        let token_morty = accounts(3);
        let amount = 10_000_u128;
        let mut amm = AMM::new(owner, token_rick.clone(), token_morty.clone());
        amm.add_token_to_pool(token_rick, amount.into(), None);
    }

    #[test]
    #[should_panic(expected = "Tokens can't be equals")]
    fn test_swap_same_tokens() {
        let owner = accounts(1);
        let token_rick = accounts(2);
        let token_morty = accounts(3);
        let amount = 10_000_u128;
        let mut amm = AMM::new(owner, token_rick.clone(), token_morty);
        amm.swap(token_rick.clone(), token_rick, amount.into());
    }

    #[test]
    #[should_panic(expected = "Token not supported")]
    fn test_swap_not_supported() {
        let owner = accounts(1);
        let token_rick = accounts(2);
        let token_morty = accounts(3);
        let token_zombie = accounts(4);
        let amount = 10_000_u128;
        let mut amm = AMM::new(owner, token_rick.clone(), token_morty);
        amm.set_metadata_a(meta_a());
        amm.set_metadata_b(meta_b());
        amm.swap(token_rick, token_zombie, amount.into());
    }
}
