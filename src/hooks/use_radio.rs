use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
    sync::Arc,
};

use dioxus_lib::prelude::*;

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
    listeners: Signal<HashMap<ScopeId, RadioListener<Channel>>>,
    schedule_update_any: Signal<Arc<dyn Fn(ScopeId) + Send + Sync>>,
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
    pub(crate) fn is_listening(&self, channel: &Channel, scope_id: &ScopeId) -> bool {
        let listeners = self.listeners.peek_unchecked();
        listeners
            .get(scope_id)
            .map(|listener| &listener.channel == channel)
            .unwrap_or_default()
    }

    pub(crate) fn listen(&self, channel: Channel, scope_id: ScopeId) {
        let mut listeners = self.listeners.write_unchecked();
        listeners.insert(
            scope_id,
            RadioListener {
                channel,
                drop_signal: CopyValue::new_maybe_sync(()),
            },
        );
    }

    pub(crate) fn unlisten(&self, scope_id: ScopeId) {
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
        listeners.remove(&scope_id);
    }

    pub(crate) fn notify_listeners(&self, channel: &Channel) {
        let mut listeners = self.listeners.write_unchecked();

        // Remove dropped listeners
        listeners.retain(|_, listener| listener.drop_signal.try_write().is_ok());

        for (scope_id, listener) in listeners.iter() {
            if &listener.channel == channel {
                (self.schedule_update_any.peek())(*scope_id)
            }
        }
    }

    pub(crate) fn get_scope_channel(&self, scope_id: ScopeId) -> Channel {
        let listeners = self.listeners.peek();
        listeners.get(&scope_id).unwrap().channel.clone()
    }

    /// Read the current state value.
    //// Example:
    ///
    /// ```rs
    /// let value = radio.read();
    /// ```
    pub fn read(&self) -> ReadableRef<Signal<Value>> {
        self.value.read()
    }

    /// Read the current state value without subscribing.
    //// Example:
    ///
    /// ```rs
    /// let value = radio.peek();
    /// ```
    pub fn peek(&self) -> ReadableRef<Signal<Value>> {
        self.value.peek()
    }
}

pub struct RadioAntenna<Value, Channel>
where
    Channel: RadioChannel<Value>,
    Value: 'static,
{
    station: RadioStation<Value, Channel>,
    scope_id: ScopeId,
}

impl<Value, Channel> RadioAntenna<Value, Channel>
where
    Channel: RadioChannel<Value>,
{
    pub(crate) fn new(
        station: RadioStation<Value, Channel>,
        scope_id: ScopeId,
    ) -> RadioAntenna<Value, Channel> {
        RadioAntenna { station, scope_id }
    }

    pub fn get_channel(&self) -> Channel {
        self.station.get_scope_channel(self.scope_id)
    }
}

impl<Value, Channel> Drop for RadioAntenna<Value, Channel>
where
    Channel: RadioChannel<Value>,
{
    fn drop(&mut self) {
        self.station.unlisten(self.scope_id)
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

    pub(crate) fn subscribe_scope_if_not(&self) {
        if !dioxus_core::vdom_is_rendering() {
            return;
        }

        let scope_id = current_scope_id().unwrap();
        let antenna = &self.antenna.write_unchecked();
        let channel = antenna.get_channel();
        let is_listening = antenna.station.is_listening(&channel, &scope_id);

        // Subscribe the reader scope to the channel if it wasn't already
        if !is_listening {
            antenna.station.listen(channel, scope_id);
        }
    }

    /// Read the current state value.
    //// Example:
    ///
    /// ```rs
    /// let value = radio.read();
    /// ```
    pub fn read(&self) -> ReadableRef<Signal<Value>> {
        self.subscribe_scope_if_not();
        self.antenna.peek().station.value.peek_unchecked()
    }

    /// Read the current state value inside a callback.
    //// Example:
    ///
    /// ```rs
    /// radio.with(|value| {
    ///     // Do something with `value`
    /// });
    /// ```
    pub fn with(&self, cb: impl FnOnce(ReadableRef<Signal<Value>>)) {
        self.subscribe_scope_if_not();
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
        RadioGuard {
            channels: self.antenna.peek().get_channel().derive_channel(&*value),
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

    let radio = use_hook(|| {
        let antenna = RadioAntenna::new(station, current_scope_id().unwrap());
        Radio::new(Signal::new(antenna))
    });

    radio
        .antenna
        .peek()
        .station
        .listen(channel, current_scope_id().unwrap());

    radio
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
        schedule_update_any: Signal::new(schedule_update_any()),
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
