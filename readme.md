**aight-im**
---

## usage
```
rustup override add nightly

# terminal 1
cd aight-server
cargo run --bin server

# terminal 2
cd aight-client
cargo run --bin client lucy

# terminal 3
cd aight-client
cargo run --bin client tom
```

## client-side commmand

| command | description |
| ------ | ------ | 
| `:exit` | exit the client | 
| `:echo <content>` | echo | 
| `:to <id> <content>` | send msg to user by id | 
