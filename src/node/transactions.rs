#[derive(Debug, Clone)]
pub enum ParsedTx {
    CreateUser {
        name: String,
        balance: i64,
    },
    Transfer {
        from: String,
        to: String,
        sum: i64,
    },
}

pub fn parse_transaction(data: &str) -> Option<ParsedTx> {
    if let Some((name, balance)) = data.split_once('=') {
        if let Ok(amount) = balance.parse::<i64>() {
            return Some(ParsedTx::CreateUser {
                name: name.to_string(),
                balance: amount,
            });
        }
    }

    if let Some((from_part, rest)) = data.split_once("->") {
        if let Some((to, amount)) = rest.split_once(':') {
            if let Ok(sum) = amount.parse::<i64>() {
                return Some(ParsedTx::Transfer {
                    from: from_part.to_string(),
                    to: to.to_string(),
                    sum,
                });
            }
        }
    }

    None
}