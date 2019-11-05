fn main() {
    prost_build::compile_protos(&["src/msg_types.proto"],
                                &["src/"]).unwrap();
}