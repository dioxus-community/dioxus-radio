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

pub struct RadioStation<Sound, Channel>
where
    Channel: RadioChannel,
{
    sound: Rc<RefCell<Sound>>,
    listeners: Rc<RefCell<HashMap<ScopeId, Channel>>>,
    play_button: Arc<dyn Fn(ScopeId) + Send + Sync>,
}

impl<Sound, Channel> Clone for RadioStation<Sound, Channel>
where
    Channel: RadioChannel,
{
    fn clone(&self) -> Self {
        Self {
            sound: self.sound.clone(),
            listeners: self.listeners.clone(),
            play_button: self.play_button.clone(),
        }
    }
}

impl<Sound, Channel> RadioStation<Sound, Channel>
where
    Channel: RadioChannel,
{
    pub fn listen(&self, channel: Channel, scope_id: ScopeId) {
        let mut listeners = self.listeners.borrow_mut();
        listeners.insert(scope_id, channel);
    }

    pub fn unlisten(&self, scope_id: ScopeId) {
        let mut listeners = self.listeners.borrow_mut();
        listeners.remove(&scope_id);
    }

    pub fn play(&self, channel: &Channel) {
        let listeners = self.listeners.borrow();

        for (scope_id, listener_channel) in listeners.iter() {
            if listener_channel == channel {
                (self.play_button)(*scope_id)
            }
        }
    }

    pub fn what_is_scope_listening(&self, scope_id: ScopeId) -> Channel {
        let listeners = self.listeners.borrow();
        listeners.get(&scope_id).unwrap().clone()
    }
}

pub struct Radioantenna<Sound, Channel>
where
    Channel: RadioChannel,
{
    station: RadioStation<Sound, Channel>,
    scope_id: ScopeId,
}

impl<Sound, Channel> Radioantenna<Sound, Channel>
where
    Channel: RadioChannel,
{
    pub fn new(
        station: RadioStation<Sound, Channel>,
        scope_id: ScopeId,
    ) -> Radioantenna<Sound, Channel> {
        Radioantenna { station, scope_id }
    }
}

impl<Sound, Channel> Drop for Radioantenna<Sound, Channel>
where
    Channel: RadioChannel,
{
    fn drop(&mut self) {
        self.station.unlisten(self.scope_id)
    }
}

pub struct RadioGuard<'a, Sound, Channel>
where
    Channel: RadioChannel,
{
    antenna: &'a Rc<Radioantenna<Sound, Channel>>,
    channel: Channel,
    value: RefMut<'a, Sound>,
}

impl<'a, Sound, Channel> Drop for RadioGuard<'a, Sound, Channel>
where
    Channel: RadioChannel,
{
    fn drop(&mut self) {
        self.antenna.station.play(&self.channel)
    }
}

impl<'a, Sound, Channel> Deref for RadioGuard<'a, Sound, Channel>
where
    Channel: RadioChannel,
{
    type Target = RefMut<'a, Sound>;

    fn deref(&self) -> &RefMut<'a, Sound> {
        &self.value
    }
}

impl<'a, Sound, Channel> DerefMut for RadioGuard<'a, Sound, Channel>
where
    Channel: RadioChannel,
{
    fn deref_mut(&mut self) -> &mut RefMut<'a, Sound> {
        &mut self.value
    }
}

pub struct Radio<Sound, Channel>
where
    Channel: RadioChannel,
{
    antenna: Rc<Radioantenna<Sound, Channel>>,
}

impl<Sound, Channel> Radio<Sound, Channel>
where
    Channel: RadioChannel,
{
    pub fn new(antenna: Rc<Radioantenna<Sound, Channel>>) -> Radio<Sound, Channel> {
        Radio { antenna }
    }

    pub fn read(&self) -> Ref<Sound> {
        self.antenna.station.sound.borrow()
    }

    pub fn write(&self) -> RadioGuard<Sound, Channel> {
        RadioGuard {
            channel: self
                .antenna
                .station
                .what_is_scope_listening(self.antenna.scope_id),
            antenna: &self.antenna,
            value: self.antenna.station.sound.borrow_mut(),
        }
    }
}

pub fn use_radio<Sound: 'static, Channel: 'static>(
    cx: &ScopeState,
    channel: Channel,
) -> &Radio<Sound, Channel>
where
    Channel: RadioChannel,
{
    let station = use_context::<RadioStation<Sound, Channel>>(cx).unwrap();

    let radio = cx.use_hook(|| {
        let antenna = Radioantenna::new(station.clone(), cx.scope_id());
        Radio::new(Rc::new(antenna))
    });

    station.listen(channel, cx.scope_id());

    radio
}

pub fn use_init_radio_station<Sound: 'static, Channel: 'static>(
    cx: &ScopeState,
    init_value: impl FnOnce() -> Sound,
) -> &RadioStation<Sound, Channel>
where
    Channel: RadioChannel,
{
    use_context_provider(cx, || RadioStation {
        sound: Rc::new(RefCell::new(init_value())),
        play_button: cx.schedule_update_any(),
        listeners: Rc::default(),
    })
}
