pub mod connections;

pub mod protobufs {
    #![allow(non_snake_case)]
    include!(concat!(env!("OUT_DIR"), "/meshtastic.rs"));
}

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
