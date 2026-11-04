//! # Enterprise Bus Pattern
//!
//! Two lightweight in-process buses decouple the HTTP surface from the
//! application layer:
//!
//! * [`CommandBus`] — the **write** side (create / update / delete / …).
//!   Every command names its own output type via the [`Command`] trait.
//! * [`QueryBus`]   — the **read** side (get / list / …), likewise typed
//!   through [`Query`].
//!
//! Handlers are registered per *message type*. Registration wraps the
//! handler in a type-erased adapter keyed by the message's [`TypeId`],
//! so a single handler struct may serve many messages without tripping
//! Rust coherence rules. Dispatch returns a boxed [`Any`] that `send` /
//! `ask` downcasts back to the declared output type.
//!
//! ```text
//! handler ──send(CreateOrdersCommand)──▶ CommandBus ──▶ OrdersHandlers
//!            Result<Orders, BusError>  ◀───────  .handle(command)
//! ```

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::Arc;

use crate::enterprise::repository::RepositoryError;

/// Boxed send-able future used by the erased handler machinery.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Errors raised while routing or executing a message.
#[derive(Debug, thiserror::Error)]
pub enum BusError {
    /// No handler was registered for the message type.
    #[error("no handler registered for this message type")]
    NoHandler,
    /// The erased payload did not downcast to the expected type.
    #[error("message payload did not match the registered handler")]
    WrongType,
    /// The underlying application/repository layer failed.
    #[error(transparent)]
    Repo(#[from] RepositoryError),
}

/// A write-side message with an associated output type.
pub trait Command: Send + 'static {
    /// Value returned to the caller after the command executes.
    type Output: Send + 'static;
}

/// A read-side message with an associated output type.
pub trait Query: Send + 'static {
    /// Value returned to the caller after the query executes.
    type Output: Send + 'static;
}

/// Contract for handlers of a specific [`Command`].
#[async_trait::async_trait]
pub trait CommandHandler<C: Command>: Send + Sync {
    /// Executes the command.
    async fn handle(&self, command: C) -> Result<C::Output, BusError>;
}

/// Contract for handlers of a specific [`Query`].
#[async_trait::async_trait]
pub trait QueryHandler<Q: Query>: Send + Sync {
    /// Executes the query.
    async fn handle(&self, query: Q) -> Result<Q::Output, BusError>;
}

/// Type-erased dispatch target shared by both buses.
trait ErasedHandler: Send + Sync {
    /// Consumes an erased message and produces an erased output.
    fn invoke<'a>(
        &'a self,
        message: Box<dyn Any + Send>,
    ) -> BoxFuture<'a, Result<Box<dyn Any + Send>, BusError>>;
}

/// Wraps a handler for one specific command type.
struct CommandAdapter<C, H> {
    inner: H,
    _command: PhantomData<fn() -> C>,
}

/// Wraps a handler for one specific query type.
struct QueryAdapter<Q, H> {
    inner: H,
    _query: PhantomData<fn() -> Q>,
}

impl<C, H> ErasedHandler for CommandAdapter<C, H>
where
    C: Command + 'static,
    H: CommandHandler<C> + 'static,
{
    fn invoke<'a>(
        &'a self,
        message: Box<dyn Any + Send>,
    ) -> BoxFuture<'a, Result<Box<dyn Any + Send>, BusError>> {
        Box::pin(async move {
            let command = *message.downcast::<C>().map_err(|_| BusError::WrongType)?;
            let output = self.inner.handle(command).await?;
            Ok(Box::new(output) as Box<dyn Any + Send>)
        })
    }
}

impl<Q, H> ErasedHandler for QueryAdapter<Q, H>
where
    Q: Query + 'static,
    H: QueryHandler<Q> + 'static,
{
    fn invoke<'a>(
        &'a self,
        message: Box<dyn Any + Send>,
    ) -> BoxFuture<'a, Result<Box<dyn Any + Send>, BusError>> {
        Box::pin(async move {
            let query = *message.downcast::<Q>().map_err(|_| BusError::WrongType)?;
            let output = self.inner.handle(query).await?;
            Ok(Box::new(output) as Box<dyn Any + Send>)
        })
    }
}

/// In-process command bus (write side of the enterprise bus).
#[derive(Default)]
pub struct CommandBus {
    handlers: HashMap<TypeId, Arc<dyn ErasedHandler>>,
}

impl CommandBus {
    /// Creates an empty bus.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a handler for command type `C`.
    ///
    /// Because registration goes through the type-erased adapter, one
    /// handler struct can be registered for many command types.
    pub fn register<C, H>(&mut self, handler: H)
    where
        C: Command + 'static,
        H: CommandHandler<C> + 'static,
    {
        self.handlers.insert(
            TypeId::of::<C>(),
            Arc::new(CommandAdapter { inner: handler, _command: PhantomData }),
        );
    }

    /// Dispatches a command to its registered handler.
    pub async fn send<C: Command + 'static>(&self, command: C) -> Result<C::Output, BusError> {
        let handler = self
            .handlers
            .get(&TypeId::of::<C>())
            .ok_or(BusError::NoHandler)?;
        let boxed = handler.invoke(Box::new(command)).await?;
        boxed
            .downcast::<C::Output>()
            .map(|b| *b)
            .map_err(|_| BusError::WrongType)
    }
}

/// In-process query bus (read side of the enterprise bus).
#[derive(Default)]
pub struct QueryBus {
    handlers: HashMap<TypeId, Arc<dyn ErasedHandler>>,
}

impl QueryBus {
    /// Creates an empty bus.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a handler for query type `Q`.
    pub fn register<Q, H>(&mut self, handler: H)
    where
        Q: Query + 'static,
        H: QueryHandler<Q> + 'static,
    {
        self.handlers.insert(
            TypeId::of::<Q>(),
            Arc::new(QueryAdapter { inner: handler, _query: PhantomData }),
        );
    }

    /// Dispatches a query to its registered handler.
    pub async fn ask<Q: Query + 'static>(&self, query: Q) -> Result<Q::Output, BusError> {
        let handler = self
            .handlers
            .get(&TypeId::of::<Q>())
            .ok_or(BusError::NoHandler)?;
        let boxed = handler.invoke(Box::new(query)).await?;
        boxed
            .downcast::<Q::Output>()
            .map(|b| *b)
            .map_err(|_| BusError::WrongType)
    }
}
