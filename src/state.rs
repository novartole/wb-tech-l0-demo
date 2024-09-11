use std::future::Future;

use crate::{
    error::Error,
    model::{Delivery, Item, Order, Payment},
};

pub trait CacheOrder {
    fn get_order(
        &self,
        order_id: &str,
    ) -> impl Future<Output = Result<Option<Order>, Error>> + Send;

    fn insert_order(&self, order: &Order) -> impl Future<Output = Result<(), Error>> + Send;
}

pub trait StoreOrder {
    fn create_order(&self, order: Order) -> impl Future<Output = Result<(), Error>> + Send;

    fn get_order(
        &self,
        order_id: &str,
    ) -> impl Future<Output = Result<Option<Order>, Error>> + Send;

    fn get_delivery(
        &self,
        order_id: &str,
    ) -> impl Future<Output = Result<Option<Delivery>, Error>> + Send;

    fn get_items(
        &self,
        order_id: &str,
    ) -> impl Future<Output = Result<Option<Vec<Item>>, Error>> + Send;

    fn get_payment(
        &self,
        order_id: &str,
    ) -> impl Future<Output = Result<Option<Payment>, Error>> + Send;
}

#[derive(Clone)]
pub struct AppState<R, C>
where
    R: Clone,
    C: Clone,
{
    pub repo: R,
    pub cache: Option<C>,
}

impl<R, C> AppState<R, C>
where
    R: Clone,
    C: Clone,
{
    pub fn new(repo: R, cache: Option<C>) -> Self {
        Self { repo, cache }
    }
}
