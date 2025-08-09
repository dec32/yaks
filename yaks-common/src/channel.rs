pub trait SenderExt {
    type Msg;
    fn send_or_panic(&self, msg: Self::Msg) -> impl Future<Output = ()>;
}

impl<T> SenderExt for async_channel::Sender<T> {
    type Msg = T;

    async fn send_or_panic(&self, msg: Self::Msg) -> () {
        self.send(msg).await.unwrap()
    }
}