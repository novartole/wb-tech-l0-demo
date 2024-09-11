use crate::{
    error::Error,
    model::{Delivery, Item, Order, Payment},
    state::{AppState, CacheOrder, StoreOrder},
};

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use tracing::trace;

type Result<T> = std::result::Result<T, Error>;
type JsonResult<T> = Result<Json<T>>;

pub async fn get_order<R, C>(
    Path(order_id): Path<String>,
    State(state): State<AppState<R, C>>,
) -> JsonResult<Order>
where
    R: StoreOrder + Clone,
    C: CacheOrder + Clone + Send + 'static,
{
    let mut maybe_order = None;

    if let Some(cache) = &state.cache {
        trace!(order_id, "get order from cache");
        maybe_order = cache.get_order(&order_id).await?;
    }

    if maybe_order.is_none() {
        trace!(order_id, "get order from database");
        maybe_order = state.repo.get_order(&order_id).await?;

        if let Some(order) = maybe_order.clone() {
            if let Some(cache) = state.cache.clone() {
                trace!(?order, "insert order into cache");
                tokio::spawn(async move { cache.insert_order(&order).await });
            }
        }
    }

    let order = maybe_order.ok_or(Error::not_found("order_id", order_id, "order"))?;

    Ok(Json(order))
}

pub async fn get_delivery<R, C>(
    Path(order_id): Path<String>,
    State(state): State<AppState<R, C>>,
) -> JsonResult<Delivery>
where
    R: StoreOrder + Clone,
    C: CacheOrder + Clone,
{
    trace!(order_id, "get order delivery by order_id from db");

    let delivery = state
        .repo
        .get_delivery(&order_id)
        .await?
        .ok_or(Error::not_found("order_id", order_id, "order delivery"))?;

    Ok(Json(delivery))
}

pub async fn get_payment<R, C>(
    Path(order_id): Path<String>,
    State(state): State<AppState<R, C>>,
) -> JsonResult<Payment>
where
    R: StoreOrder + Clone,
    C: CacheOrder + Clone,
{
    trace!(order_id, "get order payment by order_id from db");

    let payment = state
        .repo
        .get_payment(&order_id)
        .await?
        .ok_or(Error::not_found("order_id", order_id, "order payment"))?;

    Ok(Json(payment))
}

pub async fn get_items<R, C>(
    Path(order_id): Path<String>,
    State(state): State<AppState<R, C>>,
) -> JsonResult<Vec<Item>>
where
    R: StoreOrder + Clone,
    C: CacheOrder + Clone,
{
    trace!(order_id, "get order items by order_id from db");

    let items = state
        .repo
        .get_items(&order_id)
        .await?
        .ok_or(Error::not_found("order_id", order_id, "order delivery"))?;

    Ok(Json(items))
}

pub async fn create_order<R, C>(
    State(state): State<AppState<R, C>>,
    Json(order): Json<Order>,
) -> Result<StatusCode>
where
    R: StoreOrder + Clone,
    C: CacheOrder + Clone,
{
    trace!(?order, "create order in database");

    state
        .repo
        .create_order(order)
        .await
        .map(|_| StatusCode::CREATED)
}
