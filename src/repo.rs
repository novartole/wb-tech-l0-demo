use std::str::FromStr;

use bb8::Pool;
use bb8_postgres::PostgresConnectionManager;
use postgres_types::ToSql;
use tokio::try_join;
use tokio_postgres::{Config, NoTls, Transaction};
use tracing::debug;

use crate::{
    dto::OrderRepoDto,
    error::Error,
    model::{Delivery, Item, Order, Payment},
    state::StoreOrder,
};

type PostgresConnectionPool = Pool<PostgresConnectionManager<NoTls>>;

#[derive(Clone)]
pub struct PostgresRepo {
    pool: PostgresConnectionPool,
}

impl PostgresRepo {
    pub async fn try_new(params: &str) -> Result<Self, tokio_postgres::Error> {
        debug!(repo = "postgres", "configure with params: {}", params);

        let config = Config::from_str(params)?;
        let manager = PostgresConnectionManager::new(config, NoTls);
        let pool = Pool::builder().build(manager).await?;

        Ok(Self { pool })
    }
}

impl StoreOrder for PostgresRepo {
    async fn get_order(&self, order_id: &str) -> Result<Option<Order>, Error> {
        debug!(repo = "postgres", "get order by order_id: {}", order_id);

        let conn = self.pool.get().await.map_err(Error::PgConnFailed)?;

        let order_dto: OrderRepoDto = {
            let maybe_row = conn
                .query_opt("SELECT * FROM orders WHERE order_uid = $1", &[&order_id])
                .await?;

            match maybe_row {
                Some(row) => row.try_into()?,
                None => return Ok(None),
            }
        };

        let get_delivery = async {
            conn.query_one(
                "SELECT * FROM deliveries WHERE id = $1",
                &[&order_dto.delivery_id],
            )
            .await?
            .try_into().map_err(Error::Other)
        };

        let get_payment = async {
            conn.query_one(
                "SELECT * FROM payments WHERE transaction = $1",
                &[&order_dto.payment_id],
            )
            .await?
            .try_into().map_err(Error::Other)
        };

        let get_items = async {
            conn.query(
                "
                    SELECT * 
                    FROM items 
                    WHERE id IN (
                        SELECT item_id 
                        FROM items_to_order 
                        WHERE order_id = $1
                    )
                ",
                &[&order_id],
            )
            .await?
            .into_iter()
            .map(Item::try_from)
            .try_fold(Vec::default(), |mut items, result_item| {
                items.push(result_item?);
                Ok::<_, Error>(items)
            })
        };

        let (delivery, payment, items) = try_join!(
            get_delivery, 
            get_payment, 
            get_items
        )?;

        Ok(Some(Order {
            order_uid: order_dto.order_uid,
            track_number: order_dto.track_number,
            entry: order_dto.entry,
            delivery,
            payment,
            items,
            locale: order_dto.locale,
            internal_signature: order_dto.internal_signature,
            customer_id: order_dto.customer_id,
            delivery_service: order_dto.delivery_service,
            shardkey: order_dto.shardkey,
            sm_id: order_dto.sm_id,
            date_created: order_dto.date_created,
            oof_shard: order_dto.oof_shard,
        }))
    }

    async fn get_items(&self, order_id: &str) -> Result<Option<Vec<Item>>, Error> {
        debug!(repo = "postgres", "get order items by order_id: {}", order_id);

        let conn = self.pool.get().await?;

        let order_count = async {
            Ok::<_, Error>(conn
                .query_one("SELECT count(*) FROM orders WHERE order_uid = $1", &[&order_id])
                .await?
                .get::<usize, i64>(0))
        };

        let get_items = async { 
            conn
                .query(
                    "
                        SELECT * 
                        FROM items 
                        WHERE id IN (
                            SELECT item_id
                            FROM items_to_order
                            WHERE order_id = $1 
                        )
                    ",
                    &[&order_id],
                )
                .await?
                .into_iter()
                .map(Item::try_from)
                .try_fold(Vec::default(), |mut items, result_item| {
                    items.push(result_item?);
                    Ok(items)
                })
        };

        let (count, items) = try_join!(order_count, get_items)?;

        Ok(if count > 0 {
            Some(items)
        } else {
            None
        })
    }

    async fn get_delivery(&self, order_id: &str) -> Result<Option<Delivery>, Error> {
        debug!(repo = "postgres", "get order delivery by order_id: {}", order_id);

        let maybe_row = self
            .pool
            .get()
            .await?
            .query_opt(
                "
                    SELECT * 
                    FROM deliveries 
                    WHERE id IN (
                        SELECT delivery_id
                        FROM orders
                        WHERE order_uid = $1
                    ) 
                ",
                &[&order_id],
            )
            .await?;

        match maybe_row {
            Some(row) => Ok(Some(row.try_into()?)),
            None => Ok(None),
        }
    }

    async fn get_payment(&self, order_id: &str) -> Result<Option<Payment>, Error> {
        debug!(repo = "postgres", "get order payment by order_id: {}", order_id);

        let maybe_row = self
            .pool
            .get()
            .await?
            .query_opt(
                "
                    SELECT * 
                    FROM payments 
                    WHERE transaction IN (
                        SELECT payment_id
                        FROM orders
                        WHERE order_uid = $1
                    )
                ",
                &[&order_id],
            )
            .await?;

        match maybe_row {
            Some(row) => Ok(Some(row.try_into()?)),
            None => Ok(None),
        }
    }

    async fn create_order(&self, order: Order) -> Result<(), Error> {
        debug!(repo = "postgres", "create order: {:?}", order);

        let mut conn = self.pool.get().await?;
        let trx = conn.transaction().await?;

        let (delivery_id, payment_id, item_ids) = try_join!(
            select_delivery_id(&trx, &order.delivery), 
            select_payment_id(&trx, &order.payment), 
            select_item_ids(&trx, &order.items)
        )?;

        try_join!(
            insert_into_orders(&trx, &order, &delivery_id, &payment_id), 
            insert_into_items_to_order(&trx, &order.order_uid, &item_ids)
        )?;

        trx.commit().await?;

        return Ok(());

        //
        // implementation
        //

        async fn select_delivery_id(
            trx: &Transaction<'_>, 
            delivery: &Delivery,
        ) -> Result<impl ToSql + Sync, Error> 
        {
            Ok(trx.query_one(
                "
                    INSERT INTO deliveries (name, phone, zip, city, address, region, email)
                    VALUES ($1, $2, $3, $4, $5, $6, $7) 
                    RETURNING id
                ",
                &[
                    &delivery.name,
                    &delivery.phone,
                    &delivery.zip,
                    &delivery.city,
                    &delivery.address,
                    &delivery.region,
                    &delivery.email,
                ],
            )
            .await?
            .get::<usize, i32>(0))
        }

        async fn select_payment_id(
            trx: &Transaction<'_>, 
            payment: &Payment
        ) -> Result<impl ToSql + Sync, Error> 
        {
            Ok(trx.query_one(
                "
                    INSERT INTO payments 
                        (transaction
                        , request_id
                        , currency
                        , provider
                        , amount
                        , payment_dt
                        , bank
                        , delivery_cost
                        , goods_total
                        , custom_fee)
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) 
                    RETURNING transaction
                ",
                &[
                    &payment.transaction,
                    &payment.request_id,
                    &payment.currency,
                    &payment.provider,
                    &payment.amount,
                    &payment.payment_dt,
                    &payment.bank,
                    &payment.delivery_cost,
                    &payment.goods_total,
                    &payment.custom_fee,
                ],
            )
            .await?
            .get::<usize, String>(0))
        }

        async fn select_item_ids(
            trx: &Transaction<'_>, 
            items: &[Item]
        ) -> Result<Vec<impl ToSql + Sync>, Error> {
            if items.is_empty() {
                return Ok(Vec::<i32>::new());
            }

            #[rustfmt::skip]
            let mut query = 
                "
                    WITH item_ids AS (
                        INSERT INTO items
                            (chrt_id
                            , track_number
                            , price
                            , rid
                            , name
                            , sale
                            , size
                            , total_price
                            , nm_id
                            , brand
                            , status)
                        VALUES
                "
                .to_owned();

            let params = (1..)
                .step_by(11)
                .zip(items.iter().enumerate().rev())
                .try_fold(
                    Vec::<&(dyn ToSql + Sync)>::with_capacity(items.len() * 11),
                    |mut params, (param_id, (id, item))| {
                        use std::fmt::Write;

                        write!(
                            &mut query, 
                            "(${}, ${}, ${}, ${}, ${}, ${}, ${}, ${}, ${}, ${}, ${})", 
                            param_id, 
                            param_id + 1, 
                            param_id + 2, 
                            param_id + 3, 
                            param_id + 4, 
                            param_id + 5, 
                            param_id + 6, 
                            param_id + 7, 
                            param_id + 8, 
                            param_id + 9, 
                            param_id + 10
                        )?;
                        if id > 0 {
                            query.push(',');
                        }

                        params.extend_from_slice(&[
                            &item.chrt_id,
                            &item.track_number,
                            &item.price,
                            &item.rid,
                            &item.name,
                            &item.sale,
                            &item.size,
                            &item.total_price,
                            &item.nm_id,
                            &item.brand,
                            &item.status,
                        ]);

                        Ok::<_, anyhow::Error>(params)
                    },
                )?;

            #[rustfmt::skip]
            query.push_str(
                "
                        RETURNING id
                    )
                    SELECT id
                    FROM item_ids
                "
            );

            Ok(trx
                .query(&query, &params)
                .await?
                .into_iter()
                .map(|row| row.get(0))
                .collect())
        }

        async fn insert_into_orders(
            trx: &Transaction<'_>, 
            order: &Order, 
            delivery_id: impl ToSql + Sync, 
            payment_id: impl ToSql + Sync
        ) -> Result<(), Error> 
        {
            trx.execute(
                "
                    INSERT INTO orders
                        (order_uid
                        , track_number
                        , entry
                        , delivery_id
                        , payment_id
                        , locale
                        , internal_signature
                        , customer_id
                        , delivery_service
                        , shardkey
                        , sm_id
                        , date_created
                        , oof_shard)
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
                ",
                &[
                    &order.order_uid,
                    &order.track_number,
                    &order.entry,
                    &delivery_id,
                    &payment_id,
                    &order.locale,
                    &order.internal_signature,
                    &order.customer_id,
                    &order.delivery_service,
                    &order.shardkey,
                    &order.sm_id,
                    &order.date_created,
                    &order.oof_shard,
                ],
            )
            .await?;

            Ok(())
        }

        async fn insert_into_items_to_order(
            trx: &Transaction<'_>, 
            order_id: impl ToSql + Sync, 
            item_ids: &[impl ToSql + Sync]
        ) -> Result<(), Error> 
        {
            if item_ids.is_empty() {
                return Ok(());
            }

            #[rustfmt::skip]
            let mut query = 
                "
                    INSERT INTO items_to_order (order_id, item_id)
                    VALUES
                "
                .to_owned();

            let params = (1..)
                .step_by(2)
                .zip(item_ids.iter().enumerate().rev())
                .try_fold(
                    Vec::<&(dyn ToSql + Sync)>::with_capacity(item_ids.len() * 2),
                    |mut params, (param_id, (id, item_id))| {
                        use std::fmt::Write;

                        write!(&mut query, "(${}, ${})", param_id, param_id + 1)?;
                        if id > 0 {
                            query.push(',');
                        }

                        params.extend_from_slice(&[&order_id, item_id]);

                        Ok::<_, anyhow::Error>(params)
                    },
                )?;

            trx.execute(&query, &params).await?;

            Ok(())
        }
    }

}
