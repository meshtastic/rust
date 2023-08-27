use std::fmt::Display;

use crate::protobufs;

pub(crate) mod handlers;
pub(crate) mod helpers;
pub mod stream_api;
pub(crate) mod stream_buffer;

#[derive(Clone, Copy, Debug, Default)]
pub enum PacketDestination {
    Local,
    #[default]
    Broadcast,
    Node(u32),
}

pub trait PacketRouter<M, E: Display> {
    fn handle_packet_from_radio(&mut self, packet: protobufs::FromRadio) -> Result<M, E>;
    fn handle_mesh_packet(&mut self, packet: protobufs::MeshPacket) -> Result<M, E>;
    fn get_source_node_id(&self) -> u32;
}
