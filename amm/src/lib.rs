use near_contract_standards::fungible_token::metadata::FungibleTokenMetadata;
use near_contract_standards::fungible_token::FungibleToken;
use near_sdk::{env, log, near_bindgen, AccountId, Balance, PanicOnDefault, PromiseOrValue};

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::U128;

mod utils;
use utils::add_decimals;

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct AMM {
    pub token_amm: FungibleToken,
    pub token_a: (FungibleToken, Option<TokenInfo>),
    pub token_b: (FungibleToken, Option<TokenInfo>),
    account_id_token_a: AccountId,
    account_id_token_b: AccountId,
}

fn init_token(account_id: &AccountId, prefix: Vec<u8>) -> FungibleToken {
    let mut a = FungibleToken::new(prefix);
    a.internal_register_account(account_id);
    a
}

#[near_bindgen]
#[derive(Debug, PartialEq, Clone, BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct TokenInfo {
    name: String,
    symbol: String,
    decimals: u8,
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

    pub fn ft_metadata_a(&self) -> TokenInfo {
        if self.token_a().is_none() {
            panic!("There is no metadata");
        } else {
            self.token_a.1.clone().unwrap()
        }
    }

    pub fn ft_metadata_b(&self) -> TokenInfo {
        if self.token_b().is_none() {
            panic!("There is no metadata");
        } else {
            self.token_b.1.clone().unwrap()
        }
    }

    pub fn set_metadata_a(&mut self, meta: FungibleTokenMetadata) {
        if self.token_a().is_some() {
            panic!("The token has metadata");
        } else {
            let info = meta.into();
            if self.token_b().is_some() && info == *self.token_b().unwrap() {
                panic!("Same token is not acceptable");
            }
            self.token_a.1 = Some(info);
        }
    }

    pub fn set_metadata_b(&mut self, meta: FungibleTokenMetadata) {
        if self.token_b().is_some() {
            panic!("The token has metadata");
        } else {
            let info = TokenInfo::from(meta);
            if self.token_a().is_some() && info == *self.token_a().unwrap() {
                panic!("Same token is not acceptable");
            }
            self.token_b.1 = Some(info);
        }
    }

    pub fn add_token_to_pool(
        &mut self,
        token_name: AccountId,
        token_amount: U128,
        memo: Option<String>,
    ) {
        self.check_meta();
        let token = self.get_token_by_name(token_name);
        let pool_owner_id = env::current_account_id();
        let payer_id = env::predecessor_account_id();
        if !pool_owner_id.eq(&payer_id) {
            panic!("You don't have right permission for this method");
        }

        token
            .0
            .internal_transfer(&payer_id, &pool_owner_id, token_amount.0, memo);
        let share = add_decimals(token_amount.0, token.1.clone().unwrap().decimals);

        let ticker = token.1.clone().unwrap().symbol;
        self.token_amm.internal_deposit(&payer_id, share);
        log!(
            "Share {} of token {} has been added to account {}",
            share,
            ticker,
            &payer_id
        );
    }

    fn token_a(&self) -> Option<&TokenInfo> {
        self.token_a.1.as_ref()
    }

    fn token_b(&self) -> Option<&TokenInfo> {
        self.token_b.1.as_ref()
    }

    fn on_account_closed(&mut self, account_id: AccountId, balance: Balance) {
        log!("Clsed @{} with {}", account_id, balance);
    }

    fn on_tokens_burned(&mut self, account_id: AccountId, amount: Balance) {
        log!("Account @{} burned {}", account_id, amount);
    }

    fn check_meta(&self) {
        if self.token_a().is_none() || self.token_b().is_none() {
            panic!("Please init the metadata of tokens")
        }
    }

    fn get_token_by_name(&mut self, token: AccountId) -> &mut (FungibleToken, Option<TokenInfo>) {
        if self.account_id_token_a.eq(&token) {
            &mut self.token_a
        } else if self.account_id_token_b.eq(&token) {
            &mut self.token_b
        } else {
            panic!("Token not supported");
        }
    }
}

impl From<FungibleTokenMetadata> for TokenInfo {
    fn from(meta: FungibleTokenMetadata) -> Self {
        TokenInfo {
            name: meta.name,
            symbol: meta.symbol,
            decimals: meta.decimals,
        }
    }
}

near_contract_standards::impl_fungible_token_core!(AMM, token_amm, on_tokens_burned);
near_contract_standards::impl_fungible_token_storage!(AMM, token_amm, on_account_closed);

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::*;
    use near_contract_standards::fungible_token::metadata::FT_METADATA_SPEC;
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::testing_env;

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
    fn get_context(predecessor_account_id: AccountId) -> VMContextBuilder {
        let mut builder = VMContextBuilder::new();
        builder
            .current_account_id(accounts(0))
            .signer_account_id(predecessor_account_id.clone())
            .predecessor_account_id(predecessor_account_id);
        builder
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
            amm.ft_metadata_b(),
            TokenInfo {
                name: "Example NEAR fungible token".to_string(),
                symbol: "FTB".to_string(),
                decimals: 8
            }
        );
        let mut context = get_context(rick_id);
        testing_env!(context.build());
        testing_env!(context.is_view(true).build());
        assert_eq!(amm.ft_total_supply().0, 0);
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
        let context = get_context(accounts(0));
        testing_env!(context.build());
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
        let context = get_context(accounts(0));
        testing_env!(context.build());
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
        let context = get_context(accounts(0));
        testing_env!(context.build());
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
        let context = get_context(accounts(0));
        testing_env!(context.build());
        let owner = accounts(1);
        let token_rick = accounts(2);
        let token_morty = accounts(3);
        let amount = 10_000_u128;
        let mut amm = AMM::new(owner, token_rick.clone(), token_morty.clone());
        amm.add_token_to_pool(token_rick, amount.into(), None);
    }
}
