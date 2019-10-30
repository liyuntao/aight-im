**aight-im**
---

## usage
```
rustup override add nightly

# terminal 1
cargo run --bin server

# terminal 2
cargo run --bin client lucy

# terminal 3
cargo run --bin client tom
```

## client-side commmand

| command | description |
| ------ | ------ | 
| `:exit` | exit the client | 
| `:echo <content>` | echo | 
| `:to <id> <content>` | send msg to user by id | 
