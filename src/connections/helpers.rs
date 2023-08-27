use rand::{distributions::Standard, prelude::Distribution, Rng};
use std::time::UNIX_EPOCH;

pub fn get_current_time_u32() -> u32 {
    std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Could not get time since unix epoch")
        .as_secs()
        .try_into()
        .expect("Could not convert u128 to u32")
}

pub fn generate_rand_id<T>() -> T
where
    Standard: Distribution<T>,
{
    let mut rng = rand::thread_rng();
    rng.gen::<T>()
}

pub fn format_data_packet(data: Vec<u8>) -> Vec<u8> {
    let (msb, _) = data.len().overflowing_shr(8);
    let lsb = (data.len() & 0xff) as u8;

    let magic_buffer = [0x94, 0xc3, msb as u8, lsb];
    let packet_slice = data.as_slice();

    [&magic_buffer, packet_slice].concat()
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
