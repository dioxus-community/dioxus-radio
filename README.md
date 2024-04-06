[![Discord Server](https://img.shields.io/discord/899851952891002890.svg?logo=discord&style=flat-square)](https://discord.gg/sKJSVNSCDJ)

# dioxus-radio 📡🦀

**Fully-typed global state management with a topics subscription system** for [Dioxus 🧬](https://dioxuslabs.com/).

## Who is this for
- You want a global state with nested reactivity
- You really don't want unnecessary re-runs
- You want a precise state subscription system
- You don't want to clone the state unnecessarily

## Support

- **Dioxus v0.5** 🧬
- All renderers ([web](https://dioxuslabs.com/learn/0.5/getting_started/wasm), [desktop](https://dioxuslabs.com/learn/0.5/getting_started/desktop), [freya](https://github.com/marc2332/freya), etc)
- Both WASM and native targets

## Installation
Install the latest release:
```sh
cargo add dioxus-radio
```

## Example

```bash	
cargo run --example demo
```

## Problem

You have a single global state but your components end up rerunning unnecessarily because even though the state itself has changed, not all components are interested in the new mutated state. Instead, you want a global state with nested reactivity.

Other frameworks solve this in their own way, for instance, Solid and its [Stores](https://www.solidjs.com/tutorial/stores_createstore) allow you to mutate the state granularly by requiring you to specify the path to the part of the state you want to mutate, this allows Solid to then rerender components that are reading from that specific part of the State.

This doesn't translate well to rust neither Dioxus, but luckily, there are other ways.

`dioxus-radio` presents a different approach, in order to have granular subscription with a global state you indicate a `Channel`, this way, whenever you mutate the state only other subscribers of the same `Channel` will be notified. This particular pattern translates quite well to Rust thanks to the usage of en Enums as `Channel`s.

## Example

Let's imagine we want an app where there might be `N` elements with each one having it's own state, at first you might think of simply using local signals in each component instance. But there is a constraint to this example, the state must be global so other components can read the state of those `N` elements.

Here is an example:

```rs

// Global state
#[derive(Default)]
struct Data {
    pub lists: Vec<Vec<String>>,
}

// Channels used to identify the subscribers of the State
#[derive(PartialEq, Eq, Clone, Debug)]
pub enum DataChannel {
    ListCreated,
    ListN(usize),
}

impl RadioChannel<Data> for DataChannel {}

fn main() {
    dioxus::launch(|| {
        // Initialize the global state
        use_init_radio_station::<Data, DataChannel>(Data::default);
        // Subscribe to the state with the channel `DataChannel::ListCreated`
        // This way whenever a writer using the `DataChannel::ListCreated` mutates the state
        // This component will rerun
        let mut radio = use_radio::<Data, DataChannel>(DataChannel::ListCreated);

        let onclick = move |_| {
            radio.write().lists.push(Vec::default());
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
    // Subscribe the state using the `DataChannel::ListN(list_n)` channel, where `list_n` is index of this element 
    // Whenever a mutation (in this case just this component) occurs only 
    // this component will rerun as it is the only one that is subscribed to this channel
    let mut radio = use_radio::<Data, DataChannel>(DataChannel::ListN(list_n));

    println!("Running DataChannel::ListCreated({list_n})");

    rsx!(
        div {
            button {
                onclick: move |_| radio.write().lists[list_n].push("Hello World".to_string()),
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
```

## Origins

The idea of `dioxus-radio` originally started when I was working in [`freya-editor`](https://github.com/marc2332/freya-editor). I struggled to optimize the state management as I was doing many unnecessary reruns, so I started working in a topic-subscription state management. Some time passed and eventually, I realized I could export this to a separate library. So I made `dioxus-radio` and it now actually powers `freya-editor` as well!

## License

MIT License