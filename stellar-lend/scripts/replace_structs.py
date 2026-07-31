import os
import re

contracts_dir = r"C:\Users\Godsmiracle\Desktop\stellarlend\stellar-lend\contracts"
event_files = [
    os.path.join(contracts_dir, "common", "src", "events.rs"),
    os.path.join(contracts_dir, "core", "src", "events.rs"),
    os.path.join(contracts_dir, "hello-world", "src", "events.rs"),
    os.path.join(contracts_dir, "lending", "src", "events.rs"),
]

# regex to remove #[contractevent] ... pub struct { ... } and #[contracttype] pub enum { ... }
remove_struct_regex = re.compile(r'(#\[contractevent.*?\]\s*)?(#\[derive.*?\]\s*)?(pub struct \w+\s*\{[^}]*\})', re.MULTILINE)
remove_enum_regex = re.compile(r'(#\[contracttype.*?\]\s*)?(#\[derive.*?\]\s*)?(pub enum \w+\s*\{[^}]*\})', re.MULTILINE)

for file in event_files:
    if not os.path.exists(file):
        continue
    
    with open(file, 'r', encoding='utf-8') as f:
        content = f.read()
    
    new_content = remove_struct_regex.sub('', content)
    new_content = remove_enum_regex.sub('', new_content)
    
    # Add pub use shared_events::*; at the top (after #![...] if present)
    lines = new_content.split('\n')
    insert_idx = 0
    for i, line in enumerate(lines):
        if not line.startswith('#!'):
            insert_idx = i
            break
            
    lines.insert(insert_idx, "pub use shared_events::*;")
    
    with open(file, 'w', encoding='utf-8') as f:
        f.write('\n'.join(lines))

# We also need to add shared-events to their Cargo.toml
cargo_files = [
    os.path.join(contracts_dir, "common", "Cargo.toml"),
    os.path.join(contracts_dir, "core", "Cargo.toml"),
    os.path.join(contracts_dir, "hello-world", "Cargo.toml"),
    os.path.join(contracts_dir, "lending", "Cargo.toml"),
]

for cfile in cargo_files:
    if not os.path.exists(cfile):
        continue
    with open(cfile, 'r', encoding='utf-8') as f:
        cargo_content = f.read()
        
    if "shared-events" not in cargo_content:
        cargo_content = cargo_content.replace("[dependencies]", "[dependencies]\nshared-events = { path = \"../shared-events\" }")
        with open(cfile, 'w', encoding='utf-8') as f:
            f.write(cargo_content)

print("Replacement complete.")
