use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
    sync::Arc,
};

use dioxus_lib::prelude::*;

pub trait RadioChannel: 'static + PartialEq + Eq + Clone {}

impl<T> RadioChannel for T where T: 'static + PartialEq + Eq + Clone {}

pub struct RadioStation<Value, Channel>
where
    Channel: RadioChannel,
    Value: 'static,
{
    value: Signal<Value>,
    listeners: Signal<HashMap<ScopeId, Channel>>,
    schedule_update_any: Signal<Arc<dyn Fn(ScopeId) + Send + Sync>>,
}

impl<Value, Channel> Clone for RadioStation<Value, Channel>
where
    Channel: RadioChannel,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<Value, Channel> Copy for RadioStation<Value, Channel> where Channel: RadioChannel {}

impl<Value, Channel> RadioStation<Value, Channel>
where
    Channel: RadioChannel,
{
    pub(crate) fn listen(&self, channel: Channel, scope_id: ScopeId) {
        let mut listeners = self.listeners.write_unchecked();
        listeners.insert(scope_id, channel);
    }

    pub(crate) fn unlisten(&self, scope_id: ScopeId) {
        let mut listeners = self.listeners.write_unchecked();
        listeners.remove(&scope_id);
    }

    pub(crate) fn notify_listeners(&self, channel: &Channel) {
        let listeners = self.listeners.write_unchecked();

        for (scope_id, listener_channel) in listeners.iter() {
            if listener_channel == channel {
                (self.schedule_update_any.peek())(*scope_id)
            }
        }
    }

    pub(crate) fn get_scope_channel(&self, scope_id: ScopeId) -> Channel {
        let listeners = self.listeners.peek();
        listeners.get(&scope_id).unwrap().clone()
    }
}

pub struct RadioAntenna<Value, Channel>
where
    Channel: RadioChannel,
    Value: 'static,
{
    station: RadioStation<Value, Channel>,
    scope_id: ScopeId,
}

impl<Value, Channel> RadioAntenna<Value, Channel>
where
    Channel: RadioChannel,
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
    Channel: RadioChannel,
{
    fn drop(&mut self) {
        self.station.unlisten(self.scope_id)
    }
}

pub struct RadioGuard<Value, Channel>
where
    Channel: RadioChannel,
    Value: 'static,
{
    antenna: Signal<RadioAntenna<Value, Channel>>,
    channel: Channel,
    value: WritableRef<'static, Signal<Value>>,
}

impl<Value, Channel> Drop for RadioGuard<Value, Channel>
where
    Channel: RadioChannel,
{
    fn drop(&mut self) {
        self.antenna.peek().station.notify_listeners(&self.channel)
    }
}

impl<Value, Channel> Deref for RadioGuard<Value, Channel>
where
    Channel: RadioChannel,
{
    type Target = WritableRef<'static, Signal<Value>>;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<Value, Channel> DerefMut for RadioGuard<Value, Channel>
where
    Channel: RadioChannel,
{
    fn deref_mut(&mut self) -> &mut WritableRef<'static, Signal<Value>> {
        &mut self.value
    }
}

pub struct Radio<Value, Channel>
where
    Channel: RadioChannel,
    Value: 'static,
{
    antenna: Signal<RadioAntenna<Value, Channel>>,
}

impl<Value, Channel> Clone for Radio<Value, Channel>
where
    Channel: RadioChannel,
{
    fn clone(&self) -> Self {
        *self
    }
}
impl<Value, Channel> Copy for Radio<Value, Channel> where Channel: RadioChannel {}

impl<Value, Channel> Radio<Value, Channel>
where
    Channel: RadioChannel,
{
    pub(crate) fn new(antenna: Signal<RadioAntenna<Value, Channel>>) -> Radio<Value, Channel> {
        Radio { antenna }
    }

    /// Read the current state value.
    //// Example:
    ///
    /// ```rs
    /// let value = radio.read();
    /// ```
    pub fn read(&self) -> ReadableRef<Signal<Value>> {
        self.antenna.peek().station.value.read_unchecked()
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
    pub fn write(&self) -> RadioGuard<Value, Channel> {
        RadioGuard {
            channel: self.antenna.peek().get_channel(),
            antenna: self.antenna,
            value: self.antenna.peek().station.value.write_unchecked(),
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
    pub fn write_with(&self, cb: impl FnOnce(RadioGuard<Value, Channel>)) {
        let guard = self.write();
        cb(guard);
    }

    /// Modify the state using a custom Channel.
    ///
    /// ## Example:
    /// ```rs, no_run
    /// radio.write(Channel::Whatever).value = 1;
    /// ```
    pub fn write_channel(&self, channel: Channel) -> RadioGuard<Value, Channel> {
        RadioGuard {
            channel,
            antenna: self.antenna,
            value: self.antenna.peek().station.value.write_unchecked(),
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
        &self,
        channel: Channel,
        cb: impl FnOnce(RadioGuard<Value, Channel>),
    ) {
        let guard = self.write_channel(channel);
        cb(guard);
    }
}

pub fn use_radio<Value, Channel>(channel: Channel) -> Radio<Value, Channel>
where
    Channel: RadioChannel,
    Value: 'static,
{
    let station = use_context::<RadioStation<Value, Channel>>();

    let radio = use_hook(|| {
        let antenna = RadioAntenna::new(station, current_scope_id().unwrap());
        Radio::new(Signal::new(antenna))
    });

    station.listen(channel, current_scope_id().unwrap());

    radio
}

pub fn use_init_radio_station<Value, Channel>(
    init_value: impl FnOnce() -> Value,
) -> RadioStation<Value, Channel>
where
    Channel: RadioChannel,
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
    Channel: RadioChannel,
    Value: 'static,
{
    use_context::<RadioStation<Value, Channel>>()
}
