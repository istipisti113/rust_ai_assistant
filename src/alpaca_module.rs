//use apca::{ApiInfo, Client};
use apca::api::v2::order;
//use apca::api::v2::account;

pub async fn make_order(
    client: &apca::Client,
    symbol: &str,
    side: order::Side,
    amount: order::Amount,
    limit_price: Option<u64>,
    stop_price: Option<u64>,
    trail_price: Option<u64>,
    trail_percent: Option<u64>,
    order_type: String
) -> Result<(), String> {
    let order : Result<order::CreateReq, ()> = match order_type.as_str() {
        "market" => {
            Ok( order::CreateReqInit {
                type_: order::Type::Limit,
                limit_price: None,
                ..Default::default()
            }.init(symbol, side, amount))
        }

        "limit" => {
            Ok( order::CreateReqInit {
                type_: order::Type::Market,
                limit_price: Some(limit_price.ok_or("no limit price was given".to_string())?.into()),
                ..Default::default()
            }.init(symbol, side, amount))
        }

        "stop" => {
            Ok( order::CreateReqInit {
                type_: order::Type::Stop,
                stop_price: Some(stop_price.ok_or("no stop_price was given".to_string())?.into()),
                ..Default::default()
            }.init(symbol, side, amount))
        }

        "stop_limit" => {
            Ok( order::CreateReqInit {
                type_: order::Type::StopLimit,
                stop_price: Some(stop_price.ok_or("no stop_price was given".to_string())?.into()),
                limit_price: Some(limit_price.ok_or("no limit_price was given".to_string())?.into()),
                ..Default::default()
            }.init(symbol, side, amount))
        }

        "trailing_stop" => {
            match (trail_price, trail_percent) {
                (Some(price), None) => {
                    Ok( order::CreateReqInit {
                        type_: order::Type::StopLimit,
                        stop_price: Some(price.into()),
                        ..Default::default()
                    }.init(symbol, side, amount))
                }
                
                (None, Some(percent)) => {
                    Ok( order::CreateReqInit {
                        type_: order::Type::StopLimit,
                        limit_price: Some(percent.into()),
                        ..Default::default()
                    }.init(symbol, side, amount))
                }
                _ => {
                    return Err("neither trail_price nor trail_percent were given when order type was trailing_stop".to_string());
                }
            }
        }
        wrong_type => {
            return Err(format!("invalid order type: {}", wrong_type));
        }
    };


    match order {
        Ok(ord) => {
            match client.issue::<order::Create>(&ord).await {
                Ok(_) => {
                    return Ok(());
                },
                Err(_) => {
                    return Err("error on order submission".to_string());
                }
            }
        },
        Err(_) => {unreachable!()}
    }
}
