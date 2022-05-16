use near_contract_standards::fungible_token::metadata::FungibleTokenMetadata;
use near_contract_standards::fungible_token::FungibleToken;
use near_sdk::{env, near_bindgen, AccountId, PanicOnDefault};

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct AMM {
    pub token_amm: FungibleToken,
    pub token_a: (FungibleToken, Option<TokenInfo>),
    pub token_b: (FungibleToken, Option<TokenInfo>),
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
    decimals: u8,
}

#[near_bindgen]
impl AMM {
    #[init]
    pub fn new(owner_id: AccountId, token_a_id: AccountId, token_b_id: AccountId) -> Self {
        assert!(!env::state_exists(), "Already initialized");
        let ft_a = init_token(&token_a_id, b"a".to_vec());
        let ft_b = init_token(&token_b_id, b"b".to_vec());
        let mut token_amm = init_token(&owner_id, b"amm".to_vec());
        token_amm.internal_register_account(&token_a_id);
        token_amm.internal_register_account(&token_b_id);

        Self {
            token_amm,
            token_a: (ft_a, None),
            token_b: (ft_b, None),
        }
    }

    #[handle_result]
    pub fn ft_metadata_a(&self) -> Result<TokenInfo, &'static str> {
        if self.token_a().is_none() {
            Err("Err")
        } else {
            Ok(self.token_a.1.clone().unwrap())
        }
    }

    #[handle_result]
    pub fn ft_metadata_b(&self) -> Result<TokenInfo, &'static str> {
        if self.token_b().is_none() {
            Err("Err")
        } else {
            Ok(self.token_b.1.clone().unwrap())
        }
    }

    #[handle_result]
    pub fn set_metadata_a(&mut self, meta: FungibleTokenMetadata) -> Result<(), &'static str> {
        if self.token_a().is_some() {
            Err("Err")
        } else {
            let info = TokenInfo {
                name: meta.name,
                decimals: meta.decimals,
            };
            self.token_a.1 = Some(info);
            Ok(())
        }
    }

    #[handle_result]
    pub fn set_metadata_b(&mut self, meta: FungibleTokenMetadata) -> Result<(), &'static str> {
        if self.token_b().is_some() {
            Err("Err")
        } else {
            let info = TokenInfo {
                name: meta.name,
                decimals: meta.decimals,
            };
            self.token_b.1 = Some(info);
            Ok(())
        }
    }

    fn token_a(&self) -> Option<&TokenInfo> {
        self.token_a.1.as_ref()
    }

    fn token_b(&self) -> Option<&TokenInfo> {
        self.token_b.1.as_ref()
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::*;
    use near_contract_standards::fungible_token::metadata::FT_METADATA_SPEC;
    use near_sdk::test_utils::accounts;

    #[test]
    fn test_init() {
        let mut amm = AMM::new(accounts(0), accounts(1), accounts(2));
        assert!(amm.ft_metadata_a().is_err());
        assert!(amm.ft_metadata_b().is_err());
        let meta_a = FungibleTokenMetadata {
            spec: FT_METADATA_SPEC.to_string(),
            name: "Example NEAR fungible token".to_string(),
            symbol: "FTA".to_string(),
            icon: None,
            reference: None,
            reference_hash: None,
            decimals: 8,
        };
        let meta_b = FungibleTokenMetadata {
            spec: FT_METADATA_SPEC.to_string(),
            name: "Example NEAR fungible token".to_string(),
            symbol: "FTB".to_string(),
            icon: None,
            reference: None,
            reference_hash: None,
            decimals: 8,
        };
        assert!(amm.set_metadata_a(meta_a.clone()).is_ok());
        assert!(amm.set_metadata_a(meta_a).is_err());
        assert!(amm.ft_metadata_a().is_ok());
        assert!(amm.set_metadata_b(meta_b.clone()).is_ok());
        assert!(amm.set_metadata_b(meta_b).is_err());
        assert!(amm.ft_metadata_b().is_ok());

        assert_eq!(
            amm.ft_metadata_b().unwrap(),
            TokenInfo {
                name: "Example NEAR fungible token".to_string(),
                decimals: 8
            }
        );
    }
}
