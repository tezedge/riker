#![deny(clippy::all)]
// #![deny(clippy::pedantic)]
// #![deny(clippy::nursery)]
#![allow(clippy::new_ret_no_self)]
#![allow(clippy::from_over_into)]
#![allow(clippy::new_without_default)]
#![forbid(unsafe_code)]

mod validate;

pub mod actor;
mod config;
pub mod kernel;
pub mod system;
mod tokio_backend;

use std::any::Any;
use std::fmt;
use std::fmt::Debug;

use crate::actor::BasicActorRef;

pub use self::config::{load_config, Config};

/// Wraps message and sender
#[derive(Debug, Clone)]
pub struct Envelope<T: Message> {
    pub sender: Option<BasicActorRef>,
    pub msg: T,
}

pub trait Message: Debug + Clone + Send + 'static {}
impl<T: Debug + Clone + Send + 'static> Message for T {}

pub struct AnyMessage {
    pub one_time: bool,
    pub msg: Option<Box<dyn Any + Send>>,
}

pub struct DowncastAnyMessageError;

impl AnyMessage {
    pub fn new<T>(msg: T, one_time: bool) -> Self
    where
        T: Any + Message,
    {
        Self {
            one_time,
            msg: Some(Box::new(msg)),
        }
    }

    pub fn take<T>(&mut self) -> Result<T, DowncastAnyMessageError>
    where
        T: Any + Message,
    {
        if self.one_time {
            match self.msg.take() {
                Some(m) => {
                    if m.is::<T>() {
                        Ok(*m.downcast::<T>().unwrap())
                    } else {
                        Err(DowncastAnyMessageError)
                    }
                }
                None => Err(DowncastAnyMessageError),
            }
        } else {
            match self.msg.as_ref() {
                Some(m) if m.is::<T>() => Ok(m.downcast_ref::<T>().cloned().unwrap()),
                Some(_) => Err(DowncastAnyMessageError),
                None => Err(DowncastAnyMessageError),
            }
        }
    }
}

impl Clone for AnyMessage {
    fn clone(&self) -> Self {
        panic!("Can't clone a message of type `AnyMessage`");
    }
}

impl Debug for AnyMessage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("AnyMessage")
    }
}

pub mod actors {
    pub use crate::actor::{
        actor, channel, Actor, ActorArgs, ActorFactory, ActorFactoryArgs, ActorPath, ActorProducer,
        ActorRef, ActorRefFactory, ActorReference, ActorUri, All, BasicActorRef, BoxActorProd,
        BoxedTell, Channel, ChannelMsg, ChannelRef, Context, CreateError, DLChannelMsg, DeadLetter,
        EventsChannel, Props, Publish, Receive, Sender, Subscribe, SubscribeWithResponse,
        SubscribedResponse, SysTopic, Tell, Topic, Unsubscribe, UnsubscribeAll,
    };
    pub use crate::system::{
        ActorSystem, ActorSystemBackend, ScheduleId, SendingBackend, SystemBuilder, SystemEvent,
        SystemMsg, Timer,
    };
    pub use crate::tokio_backend::ActorSystemBackendTokio;
    pub use crate::{AnyMessage, Message};
}
