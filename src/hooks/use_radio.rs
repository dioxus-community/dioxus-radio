use std::{
    collections::{HashMap, HashSet},
    ops::{Deref, DerefMut},
    sync::{Arc, Mutex},
};

use dioxus_lib::prelude::*;
mod warnings {
    pub use warnings::Warning;
}
pub use warnings::Warning;

#[cfg(feature = "tracing")]
pub trait RadioChannel<T>: 'static + PartialEq + Eq + Clone + Hash + std::fmt::Debug + Ord {
    fn derive_channel(self, _radio: &T) -> Vec<Self> {
        vec![self]
    }
}

#[cfg(not(feature = "tracing"))]
pub trait RadioChannel<T>: 'static + PartialEq + Eq + Clone {
    fn derive_channel(self, _radio: &T) -> Vec<Self> {
        vec![self]
    }
}

pub struct RadioListener<Channel> {
    pub(crate) channel: Channel,
    pub(crate) drop_signal: CopyValue<()>,
}

/// Holds a global state and all its subscribers.
pub struct RadioStation<Value, Channel>
where
    Channel: RadioChannel<Value>,
    Value: 'static,
{
    value: Signal<Value>,
    listeners: Signal<HashMap<ReactiveContext, RadioListener<Channel>>>,
}

impl<Value, Channel> Clone for RadioStation<Value, Channel>
where
    Channel: RadioChannel<Value>,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<Value, Channel> Copy for RadioStation<Value, Channel> where Channel: RadioChannel<Value> {}

impl<Value, Channel> RadioStation<Value, Channel>
where
    Channel: RadioChannel<Value>,
{
    pub(crate) fn is_listening(
        &self,
        channel: &Channel,
        reactive_context: &ReactiveContext,
    ) -> bool {
        let listeners = self.listeners.peek_unchecked();
        listeners
            .get(reactive_context)
            .map(|listener| &listener.channel == channel)
            .unwrap_or_default()
    }

    pub(crate) fn listen(&self, channel: Channel, reactive_context: ReactiveContext) {
        dioxus_lib::prelude::warnings::signal_write_in_component_body::allow(|| {
            let mut listeners = self.listeners.write_unchecked();
            listeners.insert(
                reactive_context,
                RadioListener {
                    channel,
                    drop_signal: CopyValue::new_maybe_sync(()),
                },
            );
        });
    }

    pub(crate) fn unlisten(&self, reactive_context: ReactiveContext) {
        dioxus_lib::prelude::warnings::signal_write_in_component_body::allow(|| {
            let mut listeners = match self.listeners.try_write_unchecked() {
                Err(generational_box::BorrowMutError::Dropped(_)) => {
                    // It's safe to skip this error as the RadioStation's signals could have been dropped before the caller of this function.
                    // For instance: If you closed the app, the RadioStation would be dropped along all it's signals, causing the inner components
                    // to still have dropped signals and thus causing this error if they were to call the signals on a custom destructor.
                    return;
                }
                Err(e) => panic!("Unexpected error: {e}"),
                Ok(v) => v,
            };
            listeners.remove(&reactive_context);
        });
    }

    pub(crate) fn notify_listeners(&self, channel: &Channel) {
        let mut listeners = self.listeners.write_unchecked();

        #[cfg(feature = "tracing")]
        tracing::info!("Notifying {channel:?}");

        // Remove dropped listeners
        dioxus_lib::prelude::warnings::copy_value_hoisted::allow(|| {
            listeners.retain(|_, listener| listener.drop_signal.try_write().is_ok());
        });

        for (reactive_context, listener) in listeners.iter() {
            if &listener.channel == channel {
                reactive_context.mark_dirty();
            }
        }
    }

    /// Read the current state value. This effectively subscribes to any change no matter the channel.
    ///
    /// Example:
    ///
    /// ```rs
    /// let value = radio.read();
    /// ```
    pub fn read(&self) -> ReadableRef<Signal<Value>> {
        self.value.read()
    }

    /// Read the current state value without subscribing.
    ///
    /// Example:
    ///
    /// ```rs
    /// let value = radio.peek();
    /// ```
    pub fn peek(&self) -> ReadableRef<Signal<Value>> {
        self.value.peek()
    }

    #[cfg(not(feature = "tracing"))]
    pub fn print_metrics(&self) {}

    #[cfg(feature = "tracing")]
    pub fn print_metrics(&self) {
        use itertools::Itertools;
        use tracing::{info, span, Level};

        let mut channels_subscribers = HashMap::<&Channel, usize>::new();

        let listeners = self.listeners.peek();

        for sub in listeners.values() {
            *channels_subscribers.entry(&sub.channel).or_default() += 1;
        }

        let span = span!(Level::DEBUG, "Radio Station Metrics");
        let _enter = span.enter();

        for (channel, count) in channels_subscribers.iter().sorted() {
            info!(" {count} subscribers for {channel:?}")
        }
    }
}

pub struct RadioAntenna<Value, Channel>
where
    Channel: RadioChannel<Value>,
    Value: 'static,
{
    pub(crate) channel: Channel,
    station: RadioStation<Value, Channel>,
    reactive_context: ReactiveContext,
    pub(crate) subscribers: Arc<Mutex<HashSet<ReactiveContext>>>,
}

impl<Value, Channel> RadioAntenna<Value, Channel>
where
    Channel: RadioChannel<Value>,
{
    pub(crate) fn new(
        channel: Channel,
        station: RadioStation<Value, Channel>,
        reactive_context: ReactiveContext,
    ) -> RadioAntenna<Value, Channel> {
        RadioAntenna {
            channel,
            station,
            reactive_context,
            subscribers: Arc::default(),
        }
    }
}

impl<Value, Channel> Drop for RadioAntenna<Value, Channel>
where
    Channel: RadioChannel<Value>,
{
    fn drop(&mut self) {
        self.station.unlisten(self.reactive_context)
    }
}

pub struct RadioGuard<Value, Channel>
where
    Channel: RadioChannel<Value>,
    Value: 'static,
{
    antenna: Signal<RadioAntenna<Value, Channel>>,
    channels: Vec<Channel>,
    value: WritableRef<'static, Signal<Value>>,
}

impl<Value, Channel> Drop for RadioGuard<Value, Channel>
where
    Channel: RadioChannel<Value>,
{
    fn drop(&mut self) {
        for channel in &mut self.channels {
            self.antenna.peek().station.notify_listeners(channel)
        }
        if !self.channels.is_empty() {
            self.antenna.peek().station.print_metrics();
        }
    }
}

impl<Value, Channel> Deref for RadioGuard<Value, Channel>
where
    Channel: RadioChannel<Value>,
{
    type Target = WritableRef<'static, Signal<Value>>;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<Value, Channel> DerefMut for RadioGuard<Value, Channel>
where
    Channel: RadioChannel<Value>,
{
    fn deref_mut(&mut self) -> &mut WritableRef<'static, Signal<Value>> {
        &mut self.value
    }
}

/// `Radio` lets you access the state and is subscribed given it's `Channel`.
pub struct Radio<Value, Channel>
where
    Channel: RadioChannel<Value>,
    Value: 'static,
{
    antenna: Signal<RadioAntenna<Value, Channel>>,
}

impl<Value, Channel> Clone for Radio<Value, Channel>
where
    Channel: RadioChannel<Value>,
{
    fn clone(&self) -> Self {
        *self
    }
}
impl<Value, Channel> Copy for Radio<Value, Channel> where Channel: RadioChannel<Value> {}

impl<Value, Channel> PartialEq for Radio<Value, Channel>
where
    Channel: RadioChannel<Value>,
{
    fn eq(&self, other: &Self) -> bool {
        self.antenna == other.antenna
    }
}

impl<Value, Channel> Radio<Value, Channel>
where
    Channel: RadioChannel<Value>,
{
    pub(crate) fn new(antenna: Signal<RadioAntenna<Value, Channel>>) -> Radio<Value, Channel> {
        Radio { antenna }
    }

    pub(crate) fn subscribe_if_not(&self) {
        dioxus_lib::prelude::warnings::signal_write_in_component_body::allow(|| {
            if let Some(rc) = ReactiveContext::current() {
                let antenna = &self.antenna.write_unchecked();
                rc.subscribe(antenna.subscribers.clone());
                let channel = antenna.channel.clone();
                let is_listening = antenna.station.is_listening(&channel, &rc);

                // Subscribe the reader reactive context to the channel if it wasn't already
                if !is_listening {
                    antenna.station.listen(channel, rc);
                }
            }
        });
    }

    /// Read the current state value.
    ///
    /// Example:
    ///
    /// ```rs
    /// let value = radio.read();
    /// ```
    pub fn read(&self) -> ReadableRef<Signal<Value>> {
        self.subscribe_if_not();
        self.antenna.peek().station.value.peek_unchecked()
    }

    /// Read the current state value inside a callback.
    ///
    /// Example:
    ///
    /// ```rs
    /// radio.with(|value| {
    ///     // Do something with `value`
    /// });
    /// ```
    pub fn with(&self, cb: impl FnOnce(ReadableRef<Signal<Value>>)) {
        self.subscribe_if_not();
        let value = self.antenna.peek().station.value;
        let borrow = value.read();
        cb(borrow);
    }

    /// Modify the state using the channel this radio was created with.
    ///
    /// Example:
    ///
    /// ```rs
    /// radio.write().value = 1;
    /// ```
    pub fn write(&mut self) -> RadioGuard<Value, Channel> {
        let value = self.antenna.peek().station.value.write_unchecked();
        let channel = self.antenna.peek().channel.clone();
        RadioGuard {
            channels: channel.derive_channel(&*value),
            antenna: self.antenna,
            value,
        }
    }

    /// Get a mutable reference to the current state value, inside a callback.
    ///
    /// Example:
    ///
    /// ```rs
    /// radio.write_with(|value| {
    ///     // Modify `value`
    /// });
    /// ```
    pub fn write_with(&mut self, cb: impl FnOnce(RadioGuard<Value, Channel>)) {
        let guard = self.write();
        cb(guard);
    }

    /// Modify the state using a custom Channel.
    ///
    /// ## Example:
    /// ```rs, no_run
    /// radio.write(Channel::Whatever).value = 1;
    /// ```
    pub fn write_channel(&mut self, channel: Channel) -> RadioGuard<Value, Channel> {
        let value = self.antenna.peek().station.value.write_unchecked();
        RadioGuard {
            channels: channel.derive_channel(&*value),
            antenna: self.antenna,
            value,
        }
    }

    /// Get a mutable reference to the current state value, inside a callback.
    ///
    /// Example:
    ///
    /// ```rs
    /// radio.write_channel_with(Channel::Whatever, |value| {
    ///     // Modify `value`
    /// });
    /// ```
    pub fn write_channel_with(
        &mut self,
        channel: Channel,
        cb: impl FnOnce(RadioGuard<Value, Channel>),
    ) {
        let guard = self.write_channel(channel);
        cb(guard);
    }

    /// Get a mutable reference to the current state value, inside a callback that returns the channel to be used.
    ///
    /// Example:
    ///
    /// ```rs
    /// radio.write_with_channel_selection(|value| {
    ///     // Modify `value`
    ///     if value.cool {
    ///         ChannelSelection::Select(Channel::Whatever)
    ///     } else {
    ///         ChannelSelection::Silence
    ///     }
    /// });
    /// ```
    pub fn write_with_channel_selection(
        &mut self,
        cb: impl FnOnce(&mut Value) -> ChannelSelection<Channel>,
    ) {
        let value = self.antenna.peek().station.value.write_unchecked();
        let mut guard = RadioGuard {
            channels: Vec::default(),
            antenna: self.antenna,
            value,
        };
        let channel_selection = cb(&mut guard.value);
        let channel = match channel_selection {
            ChannelSelection::Current => Some(self.antenna.peek().channel.clone()),
            ChannelSelection::Silence => None,
            ChannelSelection::Select(c) => Some(c),
        };
        if let Some(channel) = channel {
            for channel in channel.derive_channel(&guard.value) {
                self.antenna.peek().station.notify_listeners(&channel)
            }
            self.antenna.peek().station.print_metrics();
        }
    }

    /// Modify the state silently, no component will be notified.
    ///
    /// This is not recommended, the only intended usage for this is inside [RadioAsyncReducer].
    ///
    pub fn write_silently(&mut self) -> RadioGuard<Value, Channel> {
        let value = self.antenna.peek().station.value.write_unchecked();
        RadioGuard {
            channels: Vec::default(),
            antenna: self.antenna,
            value,
        }
    }
}

impl<Channel> Copy for ChannelSelection<Channel> where Channel: Copy {}

#[derive(Clone)]
pub enum ChannelSelection<Channel> {
    /// Notify the channel associated with the used [Radio].
    Current,
    /// Notify a given [Channel].
    Select(Channel),
    /// No subscriber will be notified.
    Silence,
}

impl<Channel> ChannelSelection<Channel> {
    /// Change to [ChannelSelection::Current]
    pub fn current(&mut self) {
        *self = Self::Current
    }

    /// Change to [ChannelSelection::Select]
    pub fn select(&mut self, channel: Channel) {
        *self = Self::Select(channel)
    }

    /// Change to [ChannelSelection::Silence]
    pub fn silence(&mut self) {
        *self = Self::Silence
    }
}

/// Consume the state and subscribe using the given `channel`
/// Any mutation using this radio will notify other subscribers to the same `channel`,
/// unless you explicitely pass a custom channel using other methods as [`Radio::write_channel()`]
pub fn use_radio<Value, Channel>(channel: Channel) -> Radio<Value, Channel>
where
    Channel: RadioChannel<Value>,
    Value: 'static,
{
    let station = use_context::<RadioStation<Value, Channel>>();

    use_hook(|| {
        let antenna = RadioAntenna::new(channel, station, ReactiveContext::current().unwrap());
        Radio::new(Signal::new(antenna))
    })
}

pub fn use_init_radio_station<Value, Channel>(
    init_value: impl FnOnce() -> Value,
) -> RadioStation<Value, Channel>
where
    Channel: RadioChannel<Value>,
    Value: 'static,
{
    use_context_provider(|| RadioStation {
        value: Signal::new(init_value()),
        listeners: Signal::default(),
    })
}

pub fn use_radio_station<Value, Channel>() -> RadioStation<Value, Channel>
where
    Channel: RadioChannel<Value>,
    Value: 'static,
{
    use_context::<RadioStation<Value, Channel>>()
}

pub trait DataReducer {
    type Channel;
    type Action;

    fn reduce(&mut self, action: Self::Action) -> ChannelSelection<Self::Channel>;
}

pub trait RadioReducer {
    type Action;

    fn apply(&mut self, action: Self::Action);
}

impl<
        Data: DataReducer<Channel = Channel, Action = Action>,
        Channel: RadioChannel<Data>,
        Action,
    > RadioReducer for Radio<Data, Channel>
{
    type Action = Action;

    fn apply(&mut self, action: Action) {
        self.write_with_channel_selection(|data| data.reduce(action));
    }
}

pub trait DataAsyncReducer {
    type Channel;
    type Action;

    #[allow(async_fn_in_trait)]
    async fn async_reduce(
        _radio: &mut Radio<Self, Self::Channel>,
        _action: Self::Action,
    ) -> ChannelSelection<Self::Channel>
    where
        Self::Channel: RadioChannel<Self>,
        Self: Sized;
}

pub trait RadioAsyncReducer {
    type Action;

    fn async_apply(&mut self, _action: Self::Action)
    where
        Self::Action: 'static;
}

impl<
        Data: DataAsyncReducer<Channel = Channel, Action = Action>,
        Channel: RadioChannel<Data>,
        Action,
    > RadioAsyncReducer for Radio<Data, Channel>
{
    type Action = Action;

    fn async_apply(&mut self, action: Self::Action)
    where
        Self::Action: 'static,
    {
        let mut radio = *self;
        spawn(async move {
            let channel = Data::async_reduce(&mut radio, action).await;
            radio.write_with_channel_selection(|_| channel);
        });
    }
}
