use dioxus::prelude::*;
use dioxus_radio::prelude::*;

#[derive(Default)]
struct Data {
    pub count: i32,
}

pub enum DataAction {
    FetchData,
}

impl DataAsyncReducer for Data {
    type Action = DataAction;
    type Channel = DataChannel;

    async fn async_reduce(
        radio: &mut Radio<Data, Self::Channel>,
        action: Self::Action,
    ) -> Self::Channel {
        match action {
            DataAction::FetchData => {
                radio.write_silently().count += 1;

                DataChannel::FetchedData
            }
        }
    }
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum DataChannel {
    FetchedData,
    Void,
}

impl RadioChannel<Data> for DataChannel {}

fn main() {
    dioxus::launch(|| {
        use_init_radio_station::<Data, DataChannel>(Data::default);
        let mut radio = use_radio::<Data, DataChannel>(DataChannel::Void);

        let onclick = move |_| {
            radio.async_apply(DataAction::FetchData);
        };

        rsx!(
            button {
                onclick,
                "Increment",
            }
            Counter { }
        )
    });
}

#[allow(non_snake_case)]
#[component]
fn Counter() -> Element {
    let radio = use_radio::<Data, DataChannel>(DataChannel::FetchedData);

    println!("Running DataChannel::FetchedData");

    let count = radio.read().count; 

    rsx!(
        p {
            "{count}"
        }
    )
}
