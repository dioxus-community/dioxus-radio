use dioxus::prelude::*;
use dioxus_radio::prelude::*;

#[derive(Default)]
struct Data {
    list_a: Vec<String>,
    list_b: Vec<String>,
}

impl Data {
    pub fn get_list(&self, channel: &DataChannel) -> &[String] {
        match channel {
            DataChannel::ListA => &self.list_a,
            DataChannel::ListB => &self.list_b,
        }
    }

    pub fn push_to_list(&mut self, channel: &DataChannel, item: String) {
        match channel {
            DataChannel::ListA => self.list_a.push(item),
            DataChannel::ListB => self.list_b.push(item),
        }
    }
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum DataChannel {
    ListA,
    ListB,
}

fn main() {
    dioxus_desktop::launch(|cx: Scope| {
        use_init_radio_station::<Data, DataChannel>(cx, Data::default);

        render!(
            ListComp {
                channel: DataChannel::ListA
            }
            ListComp {
                channel: DataChannel::ListB
            }
        )
    });
}

#[allow(non_snake_case)]
#[component]
fn ListComp(cx: Scope, channel: DataChannel) -> Element {
    let radio = use_radio::<Data, DataChannel>(cx, channel.clone());

    println!("Rerunning with channel {channel:?}");

    render!(
        button {
            onclick: |_| radio.write().push_to_list(channel, "Hello World".to_string()),
            "New Item"
        },
        for (i, item) in radio.read().get_list(channel).iter().enumerate() {
            ul {
                key: "{i}",
                "{item}"
            }
        }
    )
}
