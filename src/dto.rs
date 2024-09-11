use anyhow::Error;
use postgres_types::{FromSql, ToSql};
use tokio_postgres::Row;

use crate::model::{Delivery, Item, Locale, Payment};

#[derive(Debug, ToSql, FromSql)]
pub struct OrderRepoDto {
    pub order_uid: String,
    pub track_number: String,
    pub entry: String,
    pub delivery_id: i32,
    pub payment_id: String,
    pub locale: Locale,
    pub internal_signature: String,
    pub customer_id: String,
    pub delivery_service: String,
    pub shardkey: String,
    pub sm_id: i32,
    pub date_created: chrono::DateTime<chrono::Utc>,
    pub oof_shard: String,
}

impl TryFrom<Row> for OrderRepoDto {
    type Error = Error;

    fn try_from(row: Row) -> Result<Self, Self::Error> {
        Ok(Self {
            order_uid: row.try_get("order_uid")?,
            track_number: row.try_get("track_number")?,
            entry: row.try_get("entry")?,
            delivery_id: row.try_get("delivery_id")?,
            payment_id: row.try_get("payment_id")?,
            locale: row.try_get("locale")?,
            internal_signature: row.try_get("internal_signature")?,
            customer_id: row.try_get("customer_id")?,
            delivery_service: row.try_get("delivery_service")?,
            shardkey: row.try_get("shardkey")?,
            sm_id: row.try_get("sm_id")?,
            date_created: row.try_get("date_created")?,
            oof_shard: row.try_get("oof_shard")?,
        })
    }
}

impl TryFrom<Row> for Delivery {
    type Error = Error;

    fn try_from(row: Row) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.try_get("id")?,
            name: row.try_get("name")?,
            phone: row.try_get("phone")?,
            zip: row.try_get("zip")?,
            city: row.try_get("city")?,
            address: row.try_get("address")?,
            region: row.try_get("region")?,
            email: row.try_get("email")?,
        })
    }
}

impl TryFrom<Row> for Payment {
    type Error = Error;

    fn try_from(row: Row) -> Result<Self, Self::Error> {
        Ok(Self {
            transaction: row.try_get("transaction")?,
            request_id: row.try_get("request_id")?,
            currency: row.try_get("currency")?,
            provider: row.try_get("provider")?,
            amount: row.try_get("amount")?,
            payment_dt: row.try_get("payment_dt")?,
            bank: row.try_get("bank")?,
            delivery_cost: row.try_get("delivery_cost")?,
            goods_total: row.try_get("goods_total")?,
            custom_fee: row.try_get("custom_fee")?,
        })
    }
}

impl TryFrom<Row> for Item {
    type Error = Error;

    fn try_from(row: Row) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.try_get("id")?,
            chrt_id: row.try_get("chrt_id")?,
            track_number: row.try_get("track_number")?,
            price: row.try_get("price")?,
            rid: row.try_get("rid")?,
            name: row.try_get("name")?,
            sale: row.try_get("sale")?,
            size: row.try_get("size")?,
            total_price: row.try_get("total_price")?,
            nm_id: row.try_get("nm_id")?,
            brand: row.try_get("brand")?,
            status: row.try_get("status")?,
        })
    }
}
