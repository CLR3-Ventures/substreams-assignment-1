// Hand-written prost types matching proto/transfers.proto (avoids needing protoc).
// `substreams protogen` would generate equivalent code.

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Transfers {
    #[prost(message, repeated, tag = "1")]
    pub transfers: ::prost::alloc::vec::Vec<Transfer>,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Transfer {
    #[prost(string, tag = "1")]
    pub from: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub to: ::prost::alloc::string::String,
    #[prost(uint64, tag = "3")]
    pub amount: u64,
}
