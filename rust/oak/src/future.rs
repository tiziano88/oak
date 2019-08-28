use crate::ReceiveChannelHalf;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, RawWaker, Waker};

impl ReceiveChannelHalf {
    fn read_future(&mut self) -> ReadFuture {
        ReadFuture {}
    }
}

pub struct ReadFuture {
    // TODO: Store reference to source channel half.
}
