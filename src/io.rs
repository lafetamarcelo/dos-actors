/*!
# [Actor]s communication module

[Actor]s communicate using channels, one input of an [actor] send data through
either a [bounded] or an [unbounded] channel to an output of another actor.
The data that moves through a channel is encapsulated into a [Data] structure.

Each input and output has a reference to the [Actor] client that reads data from
 the input and write data to the output only if the client implements the [Read]
and [Write] traits.

[Actor]s have  access to input and output methods through the `InputObject` and
`Outputobject` traits.
`InputObject` and `Outputobject` traits are trait-safe objects making the inputs and
outputs vector of [Actor]s.

[Actor]: crate::Actor
[bounded]: https://docs.rs/flume/latest/flume/fn.bounded
[unbounded]: https://docs.rs/flume/latest/flume/fn.unbounded
*/

use crate::{ActorError, Result, Who};
use async_trait::async_trait;
use flume::{Receiver, Sender};
use futures::future::join_all;
use std::{
    any::Any,
    fmt,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    sync::Arc,
};
use tokio::sync::Mutex;

/// input/output data
///
/// `T` is the data primitive type and `U` is the data unique identifier (UID)
pub struct Data<T, U>(pub T, pub PhantomData<U>);
impl<T, U> Deref for Data<T, U> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<T, U> DerefMut for Data<T, U> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl<T, U> Data<T, U> {
    /// Create a new [Data] object
    pub fn new(data: T) -> Self {
        Data(data, PhantomData)
    }
}
impl<T, U> From<&Data<Vec<T>, U>> for Vec<T>
where
    T: Clone,
{
    fn from(data: &Data<Vec<T>, U>) -> Self {
        data.to_vec()
    }
}
impl<T, U> From<Vec<T>> for Data<Vec<T>, U> {
    /// Returns data UID
    fn from(u: Vec<T>) -> Self {
        Data(u, PhantomData)
    }
}
impl<T, U> Who<U> for Data<T, U> {}
impl<T: fmt::Debug, U> fmt::Debug for Data<T, U> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct(&self.who()).field("data", &self.0).finish()
    }
}
impl<T: Default, U> Default for Data<Vec<T>, U> {
    fn default() -> Self {
        Data::new(Default::default())
    }
}

pub(crate) type S<T, U> = Arc<Data<T, U>>;

/// Actor data consumer interface
pub trait Read<T, U> {
    /// Read data from an input
    fn read(&mut self, data: Arc<Data<T, U>>);
}
/// [Actor](crate::Actor)s input
pub(crate) struct Input<C: Read<T, U>, T, U, const N: usize> {
    rx: Receiver<S<T, U>>,
    client: Arc<Mutex<C>>,
}
impl<C: Read<T, U>, T, U, const N: usize> Input<C, T, U, N> {
    /// Creates a new intput from a [Receiver] and an [Actor] client
    pub fn new(rx: Receiver<S<T, U>>, client: Arc<Mutex<C>>) -> Self {
        Self { rx, client }
    }
}
impl<C: Read<T, U>, T, U, const N: usize> Who<U> for Input<C, T, U, N> {}

#[async_trait]
pub(crate) trait InputObject: Send + Sync {
    /// Receives output data
    async fn recv(&mut self) -> Result<()>;
    fn who(&self) -> String;
}

#[async_trait]
impl<C, T, U, const N: usize> InputObject for Input<C, T, U, N>
where
    C: Read<T, U> + Send,
    T: Send + Sync,
    U: Send + Sync,
{
    async fn recv(&mut self) -> Result<()> {
        log::debug!("{} receiving", Who::who(self));
        log::debug!("{} receiving (locking client)", Who::who(self));
        let mut client = self.client.lock().await;
        log::debug!("{} receiving (client locked)", Who::who(self));
        (*client).read(self.rx.recv_async().await?);
        log::debug!("{} received", Who::who(self));
        Ok(())
    }
    fn who(&self) -> String {
        Who::who(self)
    }
}
/*
impl<C, T, U, const N: usize> From<&Input<C, Vec<T>, U, N>> for Vec<T>
where
    T: Default + Clone,
    C: Consuming<Vec<T>, U>,
{
    fn from(input: &Input<C, Vec<T>, U, N>) -> Self {
        input.data.as_ref().into()
    }
}
*/
/// Actor data producer interface
pub trait Write<T, U> {
    fn write(&mut self) -> Option<Arc<Data<T, U>>>;
}

pub(crate) struct OutputBuilder<C, T, U, const N: usize>
where
    C: Write<T, U>,
{
    tx: Vec<Sender<S<T, U>>>,
    client: Arc<Mutex<C>>,
    bootstrap: bool,
}
impl<C, T, U, const N: usize> OutputBuilder<C, T, U, N>
where
    C: Write<T, U>,
{
    pub fn new(client: Arc<Mutex<C>>) -> Self {
        Self {
            tx: Vec::new(),
            client,
            bootstrap: false,
        }
    }
    pub fn senders(self, tx: Vec<Sender<S<T, U>>>) -> Self {
        Self { tx, ..self }
    }
    pub fn bootstrap(self) -> Self {
        Self {
            bootstrap: true,
            ..self
        }
    }
    pub fn build(self) -> Output<C, T, U, N> {
        Output {
            data: None,
            tx: self.tx,
            client: self.client,
            bootstrap: self.bootstrap,
        }
    }
}

/// [Actor](crate::Actor)s output
pub(crate) struct Output<C, T, U, const N: usize>
where
    C: Write<T, U>,
{
    data: Option<S<T, U>>,
    tx: Vec<Sender<S<T, U>>>,
    client: Arc<Mutex<C>>,
    bootstrap: bool,
}
impl<C, T, U, const N: usize> Output<C, T, U, N>
where
    C: Write<T, U>,
{
    /// Creates a new output from a [Sender] and data [Default]
    pub fn builder(client: Arc<Mutex<C>>) -> OutputBuilder<C, T, U, N> {
        OutputBuilder::new(client)
    }
}
impl<C, T, U, const N: usize> Who<U> for Output<C, T, U, N> where C: Write<T, U> {}

#[async_trait]
pub trait OutputObject: Send + Sync {
    async fn send(&mut self) -> Result<()>;
    fn bootstrap(&self) -> bool;
    fn as_any(&self) -> &dyn Any;
    fn as_mut_any(&mut self) -> &mut dyn Any;
    fn len(&self) -> usize;
    fn who(&self) -> String;
}
#[async_trait]
impl<C, T, U, const N: usize> OutputObject for Output<C, T, U, N>
where
    C: 'static + Write<T, U> + Send,
    T: 'static + Send + Sync,
    U: 'static + Send + Sync,
{
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_mut_any(&mut self) -> &mut dyn Any {
        self
    }
    /// Sends output data
    async fn send(&mut self) -> Result<()> {
        self.data = (*self.client.lock().await).write();
        if let Some(data) = &self.data {
            log::debug!("{} sending", Who::who(self));
            let futures: Vec<_> = self
                .tx
                .iter()
                .map(|tx| tx.send_async(data.clone()))
                .collect();
            join_all(futures)
                .await
                .into_iter()
                .collect::<std::result::Result<Vec<()>, flume::SendError<_>>>()
                .map_err(|_| flume::SendError(()))?;
            log::debug!("{} sent", Who::who(self));
            Ok(())
        } else {
            for tx in &self.tx {
                drop(tx);
            }
            Err(ActorError::Disconnected(Who::who(self)))
        }
    }
    /// Bootstraps output
    fn bootstrap(&self) -> bool {
        self.bootstrap
    }
    fn who(&self) -> String {
        Who::who(self)
    }

    fn len(&self) -> usize {
        self.tx.len()
    }
}
