pub use shared_events::*;
use soroban_sdk::{Address, Env, Symbol};

pub const EVENT_VERSION: u32 = 1;

/// Emits a standardized event across the protocol.
/// Standard indexed fields: caller, asset, amount
#[allow(deprecated)]
pub fn emit_protocol_event(
    env: &Env,
    action_name: &str,
    caller: Address,
    asset: Address,
    amount: i128,
) {
    let topics = (
        Symbol::new(env, "PROTOCOL_EVENT"),
        Symbol::new(env, action_name),
        caller,
        asset,
    );

    let data = (amount, EVENT_VERSION);

    env.events().publish(topics, data);
}

#[macro_export]
macro_rules! emit_event {
    ($env:expr, $module:expr, $action:expr, $caller:expr, $asset:expr, $amount:expr) => {
        $crate::events::emit_protocol_event(
            $env,
            concat!($module, "_", $action),
            $caller,
            $asset,
            $amount,
        )
    };
}
