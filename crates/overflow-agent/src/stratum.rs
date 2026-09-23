//! Minimal Stratum v1 helpers.

use serde_json::Value;

pub fn payout_address_from_username(username: &str) -> String {
    username
        .split('.')
        .next()
        .unwrap_or(username)
        .trim()
        .to_string()
}

pub fn authorize_username(line: &str) -> Option<String> {
    let value: Value = serde_json::from_str(line.trim()).ok()?;
    let method = value.get("method")?.as_str()?;
    if !method.eq_ignore_ascii_case("mining.authorize") {
        return None;
    }
    let params = value.get("params")?.as_array()?;
    let user = params.first()?.as_str()?.trim();
    if user.is_empty() {
        return None;
    }
    Some(user.to_string())
}

pub fn username_looks_payable(username: &str) -> bool {
    let addr = payout_address_from_username(username);
    addr.len() >= 16
        && (addr.starts_with("bc1")
            || addr.starts_with("tb1")
            || addr.starts_with('1')
            || addr.starts_with('3')
            || addr.starts_with('m')
            || addr.starts_with('n')
            || addr.starts_with('2'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_authorize() {
        let line = r#"{"id":2,"method":"mining.authorize","params":["bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4.w","x"]}"#;
        let u = authorize_username(line).unwrap();
        assert_eq!(
            payout_address_from_username(&u),
            "bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4"
        );
    }
}
