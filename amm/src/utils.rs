use near_sdk::Balance;

pub fn add_decimals(value: Balance, decimals: u8) -> Balance {
    value * 10_u128.pow(decimals as u32)
}

pub fn remove_decimals(value: Balance, decimals: u8) -> Balance {
    value / 10_u128.pow(decimals as u32)
}

pub fn calc_dy(x: Balance, y: Balance, amount: Balance) -> Balance {
    y - (x * y / (x + amount))
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_add_decimals() {
        let decimals = add_decimals(86, 3);
        assert_eq!(decimals, 86_000);
    }

    #[test]
    fn test_remove_decimals() {
        let decimals = remove_decimals(25400, 3);
        assert_eq!(decimals, 25);
    }

    // use utils::calc_string;

    #[test]
    fn check_calculator() {
        let x = 10;
        let y = 20;
        let dy = calc_dy(x, y, 2);
        assert_eq!(dy, 4);

        let x = 10_000;
        let y = 20_0;
        let y = add_decimals(y, 2);
        let dy = calc_dy(x, y, 2_000);
        assert_eq!(remove_decimals(dy, 2), 3_3);

        let x = 860_000_000_000_0;
        let y = 270_000_000_000_0;
        let dy = calc_dy(x, y, 1000_000_000);
        assert_eq!(remove_decimals(dy, 1), 0_313_916_98);
    }
}
