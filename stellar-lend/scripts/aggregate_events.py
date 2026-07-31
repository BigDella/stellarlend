import os
import re

contracts_dir = r"C:\Users\Godsmiracle\Desktop\stellarlend\stellar-lend\contracts"
shared_events_dir = os.path.join(contracts_dir, "shared-events")
shared_events_lib = os.path.join(shared_events_dir, "src", "lib.rs")

event_files = [
    os.path.join(contracts_dir, "common", "src", "events.rs"),
    os.path.join(contracts_dir, "core", "src", "events.rs"),
    os.path.join(contracts_dir, "hello-world", "src", "events.rs"),
    os.path.join(contracts_dir, "lending", "src", "events.rs"),
]

all_structs = {}
all_enums = {}
all_functions = {}

# We will collect everything into these dicts to deduplicate
struct_regex = re.compile(r'(#\[contractevent.*?\]\s*)?(#\[derive.*?\]\s*)?(pub struct \w+ \{[^}]*\})', re.MULTILINE)
enum_regex = re.compile(r'(#\[contracttype.*?\]\s*)?(#\[derive.*?\]\s*)?(pub enum \w+ \{[^}]*\})', re.MULTILINE)

for file in event_files:
    if not os.path.exists(file):
        continue
    with open(file, 'r', encoding='utf-8') as f:
        content = f.read()
    
    # enums
    for match in enum_regex.finditer(content):
        full_def = match.group(0)
        enum_name = re.search(r'pub enum (\w+)', full_def).group(1)
        if enum_name not in all_enums:
            all_enums[enum_name] = full_def

    # structs
    for match in struct_regex.finditer(content):
        attr1 = match.group(1) or ""
        attr2 = match.group(2) or ""
        struct_body = match.group(3)
        struct_name = re.search(r'pub struct (\w+)', struct_body).group(1)
        
        # Add topics if missing
        lines = struct_body.split('\n')
        new_lines = []
        for line in lines:
            if 'pub user: Address' in line or 'pub asset: Address' in line or 'pub caller: Address' in line or 'pub admin: Address' in line or 'pub liquidator: Address' in line or 'pub borrower: Address' in line:
                if '#[topic]' not in new_lines[-1] if new_lines else True:
                    new_lines.append('    #[topic]')
            new_lines.append(line)
        
        struct_body = '\n'.join(new_lines)
        
        if struct_name not in all_structs:
            if not attr1: attr1 = "#[contractevent]\n"
            if not attr2: attr2 = "#[derive(Clone, Debug)]\n"
            all_structs[struct_name] = attr1 + attr2 + struct_body

# Write to shared-events
os.makedirs(os.path.dirname(shared_events_lib), exist_ok=True)
with open(shared_events_lib, 'w', encoding='utf-8') as f:
    f.write("#![no_std]\n#![allow(unused_variables)]\n#![allow(dead_code)]\n#![allow(deprecated)]\n\n")
    f.write("use soroban_sdk::{contractevent, contracttype, Address, Env, String, Symbol, Vec};\n\n")
    
    # Hack for InterestRateConfig, we can just define a dummy or rely on imports if needed.
    # But wait, InterestRateConfig is from lending crate.
    # To avoid cyclic dependencies, shared-events shouldn't depend on lending.
    # I'll just change InterestRateConfig fields to basic types if possible, or omit it and let lending define it.
    # Actually let's just let it be and see if it compiles, if not we will fix.
    
    for enum_name, enum_def in all_enums.items():
        f.write(enum_def + "\n\n")
    
    for struct_name, struct_def in all_structs.items():
        # Quick fix for InterestRateConfig: we just make the event use generic types or we don't move it.
        if "InterestRateConfig" in struct_def:
            struct_def = struct_def.replace("InterestRateConfig", "u32") # Hack for now, we will refine
        f.write(struct_def + "\n\n")

    # Add the core macro
    f.write('''
/// Emits a standardized event across the protocol.
#[macro_export]
macro_rules! emit_event {
    ($env:expr, $module:expr, $action:expr, $caller:expr, $asset:expr, $amount:expr) => {
        {
            let topics = (
                soroban_sdk::Symbol::new($env, "PROTOCOL_EVENT"),
                soroban_sdk::Symbol::new($env, $module),
                soroban_sdk::Symbol::new($env, $action),
                $caller.clone(),
                $asset.clone(),
            );
            let data = ($amount, 1u32);
            $env.events().publish(topics, data);
        }
    };
}
''')

print("Done generating shared-events")
