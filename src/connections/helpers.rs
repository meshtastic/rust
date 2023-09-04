use rand::{distributions::Standard, prelude::Distribution, Rng};
use std::time::UNIX_EPOCH;

/// A helper method to generate random numbers using the `rand` crate.
///
/// This method is intended to be used to generate random id values. This method
/// is generic, and will generate a random value within the range of the passed generic type.
///
/// # Arguments
///
/// None
///
/// # Returns
///
/// A random value of the passed generic type.
///
/// # Examples
///
/// ```
/// let packet_id = utils::generate_rand_id::<u32>();
/// println!("Generated random id: {}", packet_id);
/// ```
///
/// # Errors
///
/// None
///
/// # Panics
///
/// None
///
pub fn generate_rand_id<T>() -> T
where
    Standard: Distribution<T>,
{
    let mut rng = rand::thread_rng();
    rng.gen::<T>()
}

/// A helper function that takes a vector of bytes (u8) representing an encoded packet, and
/// reuturns a new vector of bytes representing the encoded packet with the required packet
/// header attached.
///
/// The header format is shown below:
///
/// ```text
/// | 0x94 (1 byte) | 0xc3 (1 byte) | MSB (1 byte) | LSB (1 byte) | DATA ((MSB << 8) | LSB bytes) |
/// ```
///
/// * `0x94` and `0xc3` are the magic bytes that are required to be at the start of every packet.
/// * `(MSB << 8) | LSB` represents the length of the packet data (`DATA`) in bytes.
/// * `DATA` is the encoded packet data that is passed to this function.
///
/// # Arguments
///
/// * `data` - A vector of bytes representing the encoded packet data.
///
/// # Returns
///
/// A vector of bytes representing the encoded packet with the required packet header attached.
///
/// # Examples
///
/// ```
/// let packet = protobufs::ToRadio { payload_variant };
///
/// let mut packet_buf: Vec<u8> = vec![];
/// packet.encode::<Vec<u8>>(&mut packet_buf)?;
///
/// let packet_buf_with_header = utils::format_data_packet(packet_buf);
/// ```
///
/// # Errors
///
/// None
///
/// # Panics
///
/// None
///
pub fn format_data_packet(data: Vec<u8>) -> Vec<u8> {
    let (msb, _) = data.len().overflowing_shr(8);
    let lsb = (data.len() & 0xff) as u8;

    let magic_buffer = [0x94, 0xc3, msb as u8, lsb];
    let packet_slice = data.as_slice();

    [&magic_buffer, packet_slice].concat()
}

pub(crate) fn current_time_u32() -> u32 {
    std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Could not get time since unix epoch")
        .as_secs()
        .try_into()
        .expect("Could not convert u128 to u32")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_empty_packet() {
        let data = vec![];
        let serial_data = format_data_packet(data);

        assert_eq!(serial_data, vec![0x94, 0xc3, 0x00, 0x00]);
    }

    #[test]
    fn valid_non_empty_packet() {
        let data = vec![0x00, 0xff, 0x88];
        let serial_data = format_data_packet(data);

        assert_eq!(serial_data, vec![0x94, 0xc3, 0x00, 0x03, 0x00, 0xff, 0x88]);
    }

    #[test]
    fn valid_large_packet() {
        let data = vec![0x00; 0x100];
        let serial_data = format_data_packet(data);

        assert_eq!(serial_data[..4], vec![0x94, 0xc3, 0x01, 0x00]);
    }
}
