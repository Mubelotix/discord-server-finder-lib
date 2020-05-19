# discord-finder

This crate contains tools you can use to manage discord invite links.

You can search google for every web page referring discord.gg in the last hour with google::search().
After you got these links, you can load the pages and parse them to get discord invite links with intermediary::resolve().
You can parse a discord invitation page with the Invite struct.

## Examples

```rust
use discord_finder::*;

for page in 0..4 {
    for link in google::search(page).unwrap() {
        println!("resolving {}", link);
        for invite_link in intermediary::resolve(&link).unwrap() {
            println!("invite link found: {}", invite_link);
        }
    }
}
```

License: MIT
