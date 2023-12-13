use dioxus::{
    core::{ScopeId, ScopeState},
    hooks::{use_context, use_context_provider, Ref, RefCell, RefMut},
};
use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
    rc::Rc,
    sync::Arc,
};

pub trait RadioChannel: PartialEq + Eq + Clone {}

impl<T> RadioChannel for T where T: PartialEq + Eq + Clone {}

pub struct RadioStation<Value, Channel>
where
    Channel: RadioChannel,
{
    value: Rc<RefCell<Value>>,
    listeners: Rc<RefCell<HashMap<ScopeId, Channel>>>,
    schedule_update_any: Arc<dyn Fn(ScopeId) + Send + Sync>,
}

impl<Value, Channel> Clone for RadioStation<Value, Channel>
where
    Channel: RadioChannel,
{
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            listeners: self.listeners.clone(),
            schedule_update_any: self.schedule_update_any.clone(),
        }
    }
}

impl<Value, Channel> RadioStation<Value, Channel>
where
    Channel: RadioChannel,
{
    pub(crate) fn listen(&self, channel: Channel, scope_id: ScopeId) {
        let mut listeners = self.listeners.borrow_mut();
        listeners.insert(scope_id, channel);
    }

    pub(crate) fn unlisten(&self, scope_id: ScopeId) {
        let mut listeners = self.listeners.borrow_mut();
        listeners.remove(&scope_id);
    }

    pub(crate) fn notify_listeners(&self, channel: &Channel) {
        let listeners = self.listeners.borrow();

        for (scope_id, listener_channel) in listeners.iter() {
            if listener_channel == channel {
                (self.schedule_update_any)(*scope_id)
            }
        }
    }

    pub(crate) fn get_scope_channel(&self, scope_id: ScopeId) -> Channel {
        let listeners = self.listeners.borrow();
        listeners.get(&scope_id).unwrap().clone()
    }
}

pub struct RadioAntenna<Value, Channel>
where
    Channel: RadioChannel,
{
    station: RadioStation<Value, Channel>,
    scope_id: ScopeId,
}

impl<Value, Channel> RadioAntenna<Value, Channel>
where
    Channel: RadioChannel,
{
    pub fn new(
        station: RadioStation<Value, Channel>,
        scope_id: ScopeId,
    ) -> RadioAntenna<Value, Channel> {
        RadioAntenna { station, scope_id }
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

pub struct RadioGuard<'a, Value, Channel>
where
    Channel: RadioChannel,
{
    antenna: &'a Rc<RadioAntenna<Value, Channel>>,
    channel: Channel,
    value: RefMut<'a, Value>,
}

impl<'a, Value, Channel> Drop for RadioGuard<'a, Value, Channel>
where
    Channel: RadioChannel,
{
    fn drop(&mut self) {
        self.antenna.station.notify_listeners(&self.channel)
    }
}

impl<'a, Value, Channel> Deref for RadioGuard<'a, Value, Channel>
where
    Channel: RadioChannel,
{
    type Target = RefMut<'a, Value>;

    fn deref(&self) -> &RefMut<'a, Value> {
        &self.value
    }
}

impl<'a, Value, Channel> DerefMut for RadioGuard<'a, Value, Channel>
where
    Channel: RadioChannel,
{
    fn deref_mut(&mut self) -> &mut RefMut<'a, Value> {
        &mut self.value
    }
}

pub struct Radio<Value, Channel>
where
    Channel: RadioChannel,
{
    antenna: Rc<RadioAntenna<Value, Channel>>,
}

impl<Value, Channel> Radio<Value, Channel>
where
    Channel: RadioChannel,
{
    pub fn new(antenna: Rc<RadioAntenna<Value, Channel>>) -> Radio<Value, Channel> {
        Radio { antenna }
    }

    /// Read the current state value.
    //// Example:
    ///
    /// ```rs
    /// let value = radio.read();
    /// ```
    pub fn read(&self) -> Ref<Value> {
        self.antenna.station.value.borrow()
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
            channel: self
                .antenna
                .station
                .get_scope_channel(self.antenna.scope_id),
            antenna: &self.antenna,
            value: self.antenna.station.value.borrow_mut(),
        }
    }

    /// Modify the state using a custom Channel.
    ///
    /// ## Example:
    /// ```rs, no_run
    /// radio.write(Channel::Whatever).value = 1;
    /// ```
    pub fn write_with(&self, channel: Channel) -> RadioGuard<Value, Channel> {
        RadioGuard {
            channel,
            antenna: &self.antenna,
            value: self.antenna.station.value.borrow_mut(),
        }
    }
}

pub fn use_radio<Value: 'static, Channel: 'static>(
    cx: &ScopeState,
    channel: Channel,
) -> &Radio<Value, Channel>
where
    Channel: RadioChannel,
{
    let station = use_context::<RadioStation<Value, Channel>>(cx).unwrap();

    let radio = cx.use_hook(|| {
        let antenna = RadioAntenna::new(station.clone(), cx.scope_id());
        Radio::new(Rc::new(antenna))
    });

    station.listen(channel, cx.scope_id());

    radio
}

pub fn use_init_radio_station<Value: 'static, Channel: 'static>(
    cx: &ScopeState,
    init_value: impl FnOnce() -> Value,
) -> &RadioStation<Value, Channel>
where
    Channel: RadioChannel,
{
    use_context_provider(cx, || RadioStation {
        value: Rc::new(RefCell::new(init_value())),
        schedule_update_any: cx.schedule_update_any(),
        listeners: Rc::default(),
    })
}
