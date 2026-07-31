#![no_main]

use libfuzzer_sys::fuzz_target;
use stellarlend_contract_fuzz::encoding::ACTION_BYTES_LEN;

fuzz_target!(|data: &[u8]| {
    if data.len() < ACTION_BYTES_LEN {
        return;
    }

    stellarlend_contract_fuzz::lending::run_critical(data);
});
