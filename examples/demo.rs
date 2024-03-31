use dioxus::prelude::*;
use dioxus_radio::prelude::*;

#[derive(Default)]
struct Data {
    pub lists: Vec<Vec<String>>,
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum DataChannel {
    ListCreated,
    ListN(usize),
}

fn main() {
    dioxus::launch(|| {
        use_init_radio_station::<Data, DataChannel>(Data::default);
        let mut radio = use_radio::<Data, DataChannel>(DataChannel::ListCreated);

        let onclick = move |_| {
            radio.write().lists.push(Vec::default());
        };

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

    println!("Rerunning list {list_n:?}.");

    rsx!(
        div {
            button {
                onclick: move |_| radio.write().lists[list_n].push("Hello World".to_string()),
                "New Item"
            },
            for (i, item) in radio.read().lists[list_n].iter().enumerate() {
                ul {
                    key: "{i}",
                    "{item}"
                }
            }
        }
    )
}
