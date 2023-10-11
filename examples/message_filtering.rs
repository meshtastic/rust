/// This example connects to a radio via serial, and demonstrates how to
/// configure handlers for different types of decoded radio packets.
/// https://meshtastic.org/docs/supported-hardware
extern crate meshtastic;

use std::io::{self, BufRead};

use meshtastic::api::StreamApi;
use meshtastic::utils;

// This import allows for decoding of mesh packets
// Re-export of prost::Message
use meshtastic::Message;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let stream_api = StreamApi::new();

    let available_ports = utils::stream::available_serial_ports()?;
    println!("Available ports: {:?}", available_ports);
    println!("Enter the name of a port to connect to:");

    let stdin = io::stdin();
    let entered_port = stdin
        .lock()
        .lines()
        .next()
        .expect("Failed to find next line")
        .expect("Could not read next line");

    let serial_stream = utils::stream::build_serial_stream(entered_port, None, None, None)?;
    let (mut decoded_listener, stream_api) = stream_api.connect(serial_stream).await;

    let config_id = utils::generate_rand_id();
    let stream_api = stream_api.configure(config_id).await?;

    // This loop can be broken with ctrl+c, or by disconnecting
    // the attached serial port.
    while let Some(decoded_packet) = decoded_listener.recv().await {
        handle_from_radio_packet(decoded_packet)
    }

    // Note that in this specific example, this will only be called when
    // the radio is disconnected, as the above loop will never exit.
    // Typically you would allow the user to manually kill the loop,
    // for example with tokio::select!.
    let _stream_api = stream_api.disconnect().await?;

    Ok(())
}

/// A helper function to handle packets coming directly from the radio connection.
/// The Meshtastic `PhoneAPI` will return decoded `FromRadio` packets, which
/// can then be handled based on their payload variant. Note that the payload
/// variant can be `None`, in which case the packet should be ignored.
fn handle_from_radio_packet(from_radio_packet: meshtastic::protobufs::FromRadio) {
    // Remove `None` variants to get the payload variant
    let payload_variant = match from_radio_packet.payload_variant {
        Some(payload_variant) => payload_variant,
        None => {
            println!("Received FromRadio packet with no payload variant, not handling...");
            return;
        }
    };

    // `FromRadio` packets can be differentiated based on their payload variant,
    // which in Rust is represented as an enum. This means the payload variant
    // can be matched on, and the appropriate user-defined action can be taken.
    match payload_variant {
        meshtastic::protobufs::from_radio::PayloadVariant::Channel(channel) => {
            println!("Received channel packet: {:?}", channel);
        }
        meshtastic::protobufs::from_radio::PayloadVariant::NodeInfo(node_info) => {
            println!("Received node info packet: {:?}", node_info);
        }
        meshtastic::protobufs::from_radio::PayloadVariant::Packet(mesh_packet) => {
            handle_mesh_packet(mesh_packet);
        }
        _ => {
            println!("Received other FromRadio packet, not handling...");
        }
    };
}

/// A helper function to handle `MeshPacket` messages, which are a subset
/// of all `FromRadio` messages. Note that the payload variant can be `None`,
/// and that the payload variant can be `Encrypted`, in which case the packet
/// should be ignored within client applications.
///
/// Mesh packets are the most commonly used type of packet, and are usually
/// what people are referring to when they talk about "packets."
fn handle_mesh_packet(mesh_packet: meshtastic::protobufs::MeshPacket) {
    // Remove `None` variants to get the payload variant
    let payload_variant = match mesh_packet.payload_variant {
        Some(payload_variant) => payload_variant,
        None => {
            println!("Received mesh packet with no payload variant, not handling...");
            return;
        }
    };

    // Only handle decoded (unencrypted) mesh packets
    let packet_data = match payload_variant {
        meshtastic::protobufs::mesh_packet::PayloadVariant::Decoded(decoded_mesh_packet) => {
            decoded_mesh_packet
        }
        meshtastic::protobufs::mesh_packet::PayloadVariant::Encrypted(_encrypted_mesh_packet) => {
            println!("Received encrypted mesh packet, not handling...");
            return;
        }
    };

    // Meshtastic differentiates mesh packets based on a field called `portnum`.
    // Meshtastic defines a set of standard port numbers [here](https://meshtastic.org/docs/development/firmware/portnum),
    // but also allows for custom port numbers to be used.
    match packet_data.portnum() {
        meshtastic::protobufs::PortNum::PositionApp => {
            // Note that `Data` structs contain a `payload` field, which is a vector of bytes.
            // This data needs to be decoded into a protobuf struct, which is shown below.
            // The `decode` function is provided by the `prost` crate, which is re-exported
            // by the `meshtastic` crate.
            let decoded_position =
                meshtastic::protobufs::Position::decode(packet_data.payload.as_slice()).unwrap();

            println!("Received position packet: {:?}", decoded_position);
        }
        meshtastic::protobufs::PortNum::TextMessageApp => {
            let decoded_text_message = String::from_utf8(packet_data.payload).unwrap();

            println!("Received text message packet: {:?}", decoded_text_message);
        }
        meshtastic::protobufs::PortNum::WaypointApp => {
            let decoded_waypoint =
                meshtastic::protobufs::Waypoint::decode(packet_data.payload.as_slice()).unwrap();

            println!("Received waypoint packet: {:?}", decoded_waypoint);
        }
        _ => {
            println!(
                "Received mesh packet on port {:?}, not handling...",
                packet_data.portnum
            );
        }
    }
}
