use dioxus::prelude::*;
use dioxus_radio::prelude::*;

#[derive(Default)]
struct Data {
    pub lists: Vec<Vec<String>>,
}

pub enum DataAction {
    NewList,
    AddToList { list: usize, text: String },
}

impl DataReducer for Data {
    type Action = DataAction;
    type Channel = DataChannel;

    fn reduce(&mut self, message: Self::Action) -> ChannelSelection<Self::Channel> {
        match message {
            DataAction::NewList => {
                self.lists.push(Vec::default());

                ChannelSelection::Select(DataChannel::ListCreation)
            }
            DataAction::AddToList { list, text } => {
                self.lists[list].push(text);

                ChannelSelection::Select(DataChannel::SpecificListItemUpdate(list))
            }
        }
    }
}

#[derive(PartialEq, Eq, Clone, Debug, Copy, Hash)]
pub enum DataChannel {
    ListCreation,
    SpecificListItemUpdate(usize),
    AnyListItemIsUpdated,
}

impl RadioChannel<Data> for DataChannel {
    fn derive_channel(self, _radio: &Data) -> Vec<Self> {
        let mut channel = vec![self];
        if let Self::SpecificListItemUpdate(_) = self {
            channel.push(Self::AnyListItemIsUpdated);
        }
        channel
    }
}

fn main() {
    dioxus::launch(|| {
        use_init_radio_station::<Data, DataChannel>(Data::default);
        let mut radio = use_radio::<Data, DataChannel>(DataChannel::ListCreation);

        let onclick = move |_| {
            radio.apply(DataAction::NewList);
        };

        println!("Running DataChannel::ListCreation");

        rsx!(
            ListObserver {}
            button {
                onclick,
                "Add new list",
            }
            for (list_n, _) in radio.read().lists.iter().enumerate() {
                ListComp {
                    key: "{list_n}",
                    list_n
                }
            }
        )
    });
}

#[allow(non_snake_case)]
#[component]
fn ListObserver() -> Element {
    let radio = use_radio::<Data, DataChannel>(DataChannel::AnyListItemIsUpdated);

    use_effect(move || {
        let _ = radio.read();
        println!("Running DataChannel::AnyListItemIsUpdated");
    });

    println!("Created DataChannel::AnyListItemIsUpdated");

    Ok(VNode::placeholder())
}

#[allow(non_snake_case)]
#[component]
fn ListComp(list_n: usize) -> Element {
    let mut radio = use_radio::<Data, DataChannel>(DataChannel::SpecificListItemUpdate(list_n));

    println!("Running DataChannel::SpecificListItemUpdate({list_n})");

    rsx!(
        div {
            button {
                onclick: move |_| radio.apply(DataAction::AddToList {
                    list: list_n,
                    text: "Hello, World".to_string()
                }),
                "New Item"
            },
            ul {
                for (i, item) in radio.read().lists[list_n].iter().enumerate() {
                    li {
                        key: "{i}",
                        "{item}"
                    }
                }
            }
        }
    )
}
