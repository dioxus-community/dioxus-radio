use dioxus::prelude::*;
use dioxus_radio::prelude::*;

#[derive(Default)]
struct Data {
    pub lists: Vec<Vec<String>>,
}

pub enum ChannelAction {
    NewList,
    AddToList { list: usize, text: String },
}

impl DataReducer for Data {
    type Action = ChannelAction;
    type Channel = DataChannel;

    fn reduce(&mut self, message: Self::Action) -> Self::Channel {
        match message {
            ChannelAction::NewList => {
                self.lists.push(Vec::default());

                DataChannel::ListCreated
            }
            ChannelAction::AddToList { list, text } => {
                self.lists[list].push(text);

                DataChannel::ListN(list)
            }
        }
    }
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum DataChannel {
    ListCreated,
    ListN(usize),
}

impl RadioChannel<Data> for DataChannel {}

fn main() {
    dioxus::launch(|| {
        use_init_radio_station::<Data, DataChannel>(Data::default);
        let mut radio = use_radio::<Data, DataChannel>(DataChannel::ListCreated);

        let onclick = move |_| {
            radio.apply(ChannelAction::NewList);
        };

        println!("Running DataChannel::ListCreated");

        rsx!(
            button {
                onclick,
                "Add new list",
            }
            for (list_n, _) in radio.read().lists.iter().enumerate() {
                ListComp {
                    list_n
                }
            }
        )
    });
}

#[allow(non_snake_case)]
#[component]
fn ListComp(list_n: usize) -> Element {
    let mut radio = use_radio::<Data, DataChannel>(DataChannel::ListN(list_n));

    println!("Running DataChannel::ListCreated({list_n})");

    rsx!(
        div {
            button {
                onclick: move |_| radio.apply(ChannelAction::AddToList {
                    list: 0,
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
