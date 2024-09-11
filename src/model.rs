pub use self::percent::Percent;

use chrono::{DateTime, Utc};
use postgres_types::{FromSql, ToSql};
use redis_macros::{FromRedisValue, ToRedisArgs};
use serde::{Deserialize, Deserializer, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

// reserve a type for operations with money
pub type Money = i32;

// keep it in separate mod to get benefits from incapsulation
mod percent {
    use super::*;

    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSql, FromSql)]
    #[serde(try_from = "i16")]
    #[postgres(transparent)]
    pub struct Percent(i16);

    impl TryFrom<i16> for Percent {
        type Error = String;

        fn try_from(value: i16) -> Result<Self, Self::Error> {
            if (0..=100).contains(&value) {
                Ok(Self(value))
            } else {
                Err(format!(
                    "Value must be within 0..=100 range, but got '{}'",
                    value
                ))
            }
        }
    }
}

// got the idea from WB API
#[allow(clippy::upper_case_acronyms)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSql, FromSql)]
#[postgres(name = "locale")]
#[serde(rename_all = "lowercase")]
pub enum Locale {
    EN,
    RU,
    ZH,
}

// status codes are typically known in advance, let's reserve an enum
#[derive(Clone, Debug, PartialEq, Serialize_repr, Deserialize_repr, ToSql, FromSql)]
#[postgres(name = "item_status")]
#[repr(u16)]
pub enum ItemStatus {
    StatusCode = 202,
}

// same about the currency
#[allow(clippy::upper_case_acronyms)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSql, FromSql)]
#[postgres(name = "currency")]
pub enum Currency {
    USD,
    RU,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromRedisValue, ToRedisArgs)]
pub struct Order {
    pub order_uid: String,
    pub track_number: String,
    pub entry: String,
    pub delivery: Delivery,
    pub payment: Payment,
    pub items: Vec<Item>,
    pub locale: Locale,
    pub internal_signature: String,
    pub customer_id: String,
    pub delivery_service: String,
    pub shardkey: String,
    pub sm_id: i32,
    pub date_created: DateTime<Utc>,
    pub oof_shard: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromRedisValue, ToRedisArgs)]
pub struct Delivery {
    #[serde(skip)]
    pub id: Option<i32>,
    pub name: String,
    // add minimum validation
    #[serde(deserialize_with = "de_phonenumber")]
    pub phone: String,
    pub zip: String,
    pub city: String,
    pub address: String,
    pub region: String,
    pub email: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromRedisValue, ToRedisArgs)]
pub struct Payment {
    pub transaction: String,
    pub request_id: String,
    pub currency: Currency,
    pub provider: String,
    pub amount: Money,
    pub payment_dt: i32,
    pub bank: String,
    pub delivery_cost: Money,
    pub goods_total: Money,
    pub custom_fee: Percent,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromRedisValue, ToRedisArgs)]
pub struct Item {
    #[serde(skip)]
    pub id: Option<i32>,
    pub chrt_id: i32,
    pub track_number: String,
    pub price: Money,
    pub rid: String,
    pub name: String,
    pub sale: Percent,
    pub size: String,
    pub total_price: Money,
    pub nm_id: i32,
    pub brand: String,
    pub status: ItemStatus,
}

/// Keep number as it is, but check if it starts with '+'.
fn de_phonenumber<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;

    String::deserialize(deserializer).and_then(|string| {
        if string.starts_with('+') {
            Ok(string)
        } else {
            Err(Error::custom("Expected number starts with '+"))
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::fmt::Debug;

    use serde::de::DeserializeOwned;
    use serde_json::{json, value::Value};

    fn test_serde<T>(value_from: Value, target: T, value_to: Value)
    where
        T: Serialize + DeserializeOwned + Debug + PartialEq,
    {
        // deserialize
        assert_eq!(target, serde_json::from_value(value_from).unwrap());

        // serialize
        assert_eq!(serde_json::to_value(&target).unwrap(), value_to);
    }

    #[test]
    fn serde_locale() {
        test_serde(json!("en"), Locale::EN, json!("en"));
        test_serde(json!("ru"), Locale::RU, json!("ru"));
        test_serde(json!("zh"), Locale::ZH, json!("zh"));

        assert!(serde_json::from_value::<Locale>(json!("En")).is_err());
        assert!(serde_json::from_value::<Locale>(json!("EN")).is_err());
        assert!(serde_json::from_value::<Locale>(json!("eN")).is_err());
        assert!(serde_json::from_value::<Locale>(json!("bad")).is_err(),);
        assert!(serde_json::from_value::<Locale>(json!("")).is_err());
    }

    #[test]
    fn serde_item_status() {
        test_serde(json!(202), ItemStatus::StatusCode, json!(202));

        assert!(serde_json::from_value::<ItemStatus>(json!("202")).is_err());
        assert!(serde_json::from_value::<ItemStatus>(json!(0)).is_err());
        assert!(serde_json::from_value::<ItemStatus>(json!(-1)).is_err());
        assert!(serde_json::from_value::<ItemStatus>(json!(u16::MAX as u32 + 1)).is_err());
    }

    #[test]
    fn serde_currency() {
        test_serde(json!("USD"), Currency::USD, json!("USD"));
        test_serde(json!("RU"), Currency::RU, json!("RU"));

        assert!(serde_json::from_value::<Currency>(json!("usd")).is_err());
        assert!(serde_json::from_value::<Currency>(json!("bad")).is_err());
        assert!(serde_json::from_value::<Currency>(json!("")).is_err());
    }

    #[test]
    fn serde_percent() {
        test_serde(json!(0), Percent::try_from(0).unwrap(), json!(0));
        test_serde(json!(50), Percent::try_from(50).unwrap(), json!(50));
        test_serde(json!(100), Percent::try_from(100).unwrap(), json!(100));

        assert!(serde_json::from_value::<Currency>(json!("50")).is_err());
        assert!(serde_json::from_value::<Currency>(json!(-1)).is_err());
        assert!(serde_json::from_value::<Currency>(json!(101)).is_err());
    }

    #[test]
    fn serde_order_demo() {
        test_serde(
            json!({
                "order_uid": "b563feb7b2b84b6test",
                "track_number": "WBILMTESTTRACK",
                "entry": "WBIL",
                "delivery": {
                    "id": 666,
                    "name": "Test Testov",
                    "phone": "+9720000000",
                    "zip": "2639809",
                    "city": "Kiryat Mozkin",
                    "address": "Ploshad Mira 15",
                    "region": "Kraiot",
                    "email": "test@gmail.com"
                },
                "payment": {
                    "transaction": "b563feb7b2b84b6test",
                    "request_id": "",
                    "currency": "USD",
                    "provider": "wbpay",
                    "amount": 1817,
                    "payment_dt": 1637907727,
                    "bank": "alpha",
                    "delivery_cost": 1500,
                    "goods_total": 317,
                    "custom_fee": 0
                },
                "items": [
                    {
                        "id": 777,
                        "chrt_id": 9934930,
                        "track_number": "WBILMTESTTRACK",
                        "price": 453,
                        "rid": "ab4219087a764ae0btest",
                        "name": "Mascaras",
                        "sale": 30,
                        "size": "0",
                        "total_price": 317,
                        "nm_id": 2389212,
                        "brand": "Vivienne Sabo",
                        "status": 202
                    }
                ],
                "locale": "en",
                "internal_signature": "",
                "customer_id": "test",
                "delivery_service": "meest",
                "shardkey": "9",
                "sm_id": 99,
                "date_created": "2021-11-26T06:22:19Z",
                "oof_shard": "1"
            }),
            Order {
                order_uid: "b563feb7b2b84b6test".to_owned(),
                track_number: "WBILMTESTTRACK".to_owned(),
                entry: "WBIL".to_owned(),
                delivery: Delivery {
                    id: None,
                    name: "Test Testov".to_owned(),
                    phone: "+9720000000".to_owned(),
                    zip: "2639809".to_owned(),
                    city: "Kiryat Mozkin".to_owned(),
                    address: "Ploshad Mira 15".to_owned(),
                    region: "Kraiot".to_owned(),
                    email: "test@gmail.com".parse().unwrap(),
                },
                payment: Payment {
                    transaction: "b563feb7b2b84b6test".to_owned(),
                    request_id: String::new(),
                    currency: Currency::USD,
                    provider: "wbpay".to_owned(),
                    amount: 1817,
                    payment_dt: 1637907727,
                    bank: "alpha".to_owned(),
                    delivery_cost: 1500,
                    goods_total: 317,
                    custom_fee: Percent::try_from(0).unwrap(),
                },
                items: vec![Item {
                    id: None,
                    chrt_id: 9934930,
                    track_number: "WBILMTESTTRACK".to_owned(),
                    price: 453,
                    rid: "ab4219087a764ae0btest".to_owned(),
                    name: "Mascaras".to_owned(),
                    sale: Percent::try_from(30).unwrap(),
                    size: "0".to_owned(),
                    total_price: 317,
                    nm_id: 2389212,
                    brand: "Vivienne Sabo".to_owned(),
                    status: ItemStatus::StatusCode,
                }],
                locale: Locale::EN,
                internal_signature: String::new(),
                customer_id: "test".to_owned(),
                delivery_service: "meest".to_owned(),
                shardkey: "9".to_owned(),
                sm_id: 99,
                date_created: "2021-11-26T06:22:19Z".parse().unwrap(),
                oof_shard: "1".to_owned(),
            },
            json!({
                "order_uid": "b563feb7b2b84b6test",
                "track_number": "WBILMTESTTRACK",
                "entry": "WBIL",
                "delivery": {
                    "name": "Test Testov",
                    "phone": "+9720000000",
                    "zip": "2639809",
                    "city": "Kiryat Mozkin",
                    "address": "Ploshad Mira 15",
                    "region": "Kraiot",
                    "email": "test@gmail.com"
                },
                "payment": {
                    "transaction": "b563feb7b2b84b6test",
                    "request_id": "",
                    "currency": "USD",
                    "provider": "wbpay",
                    "amount": 1817,
                    "payment_dt": 1637907727,
                    "bank": "alpha",
                    "delivery_cost": 1500,
                    "goods_total": 317,
                    "custom_fee": 0
                },
                "items": [
                    {
                        "chrt_id": 9934930,
                        "track_number": "WBILMTESTTRACK",
                        "price": 453,
                        "rid": "ab4219087a764ae0btest",
                        "name": "Mascaras",
                        "sale": 30,
                        "size": "0",
                        "total_price": 317,
                        "nm_id": 2389212,
                        "brand": "Vivienne Sabo",
                        "status": 202
                    }
                ],
                "locale": "en",
                "internal_signature": "",
                "customer_id": "test",
                "delivery_service": "meest",
                "shardkey": "9",
                "sm_id": 99,
                "date_created": "2021-11-26T06:22:19Z",
                "oof_shard": "1"
            }),
        );
    }
}
