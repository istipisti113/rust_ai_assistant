use apca::{ApiInfo, Client};
use apca::api::v2::order;
use apca::api::v2::account;

use anyhow;

pub async fn make_order(client: &Client, symbol: &str, side: order::Side, amount: order::Amount) -> anyhow::Result<()> {
    let request = order::CreateReqInit {
        type_: order::Type::Market,
        limit_price: None, ..Default::default()
    }.init(symbol, side, amount);
    let order = client.issue::<order::Create>(&request).await?;
    Ok(())
}
