use btleplug::api::{
    BDAddr, Central, CentralEvent, Characteristic, Manager as _, Peripheral as _, ScanFilter,
    ValueNotification, WriteType,
};
use btleplug::platform::{Adapter, Manager, Peripheral};
use futures::stream::StreamExt;
use futures_util::stream::BoxStream;
use log::error;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::fmt::Display;
use std::future;
use std::str::FromStr;
use std::time::Duration;
use uuid::Uuid;

use crate::errors_internal::{BleConnectionError, Error, InternalStreamError};
use crate::types::EncodedToRadioPacketWithHeader;
use crate::utils::format_data_packet;

const MSH_SERVICE: Uuid = Uuid::from_u128(0x6ba1b218_15a8_461f_9fa8_5dcae273eafd);
const FROMRADIO: Uuid = Uuid::from_u128(0x2c55e69e_4993_11ed_b878_0242ac120002);
const TORADIO: Uuid = Uuid::from_u128(0xf75c76d2_129e_4dad_a1dd_7866124401e7);
const FROMNUM: Uuid = Uuid::from_u128(0xed9da18c_a800_4f66_a670_aa7547e34453);

pub struct BleHandler {
    radio: Peripheral,
    adapter: Adapter,
    toradio_char: Characteristic,
    fromradio_char: Characteristic,
    fromnum_char: Characteristic,
}

#[derive(PartialEq)]
pub enum AdapterEvent {
    Disconnected,
}

pub enum RadioMessage {
    Eof,
    Packet(EncodedToRadioPacketWithHeader),
}

/// Bluetooth Low Energy ID, used to filter available devices.
#[derive(Debug, Clone, PartialEq)]
pub enum BleId {
    /// A Meshtastic device identified by its broadcast name.
    Name(String),
    /// A Meshtastic device identified by its MAC address.
    MacAddress(BDAddr),
}

impl BleId {
    /// Constructs BLE ID from the name used by Meshtastic.
    ///
    /// The first parts of the name is the Meshtastic short name and it ends with `_abcd`, where
    /// `abcd` are the last 4 hex-digits of the MAC address.
    ///
    /// A device with a short name "ZG1" and a MAC address ending with `2ef4` has name "ZG1_2ef4".
    pub fn from_name(name: &str) -> BleId {
        BleId::Name(name.to_owned())
    }

    /// Constructs a BLE ID from a string MAC address.
    ///
    /// Both `aa:bb:cc:dd:ee:ff` and `aabbccddeeff` formats are acceptable.
    pub fn from_mac_address(mac: &str) -> Result<BleId, Error> {
        let bdaddr = BDAddr::from_str(mac).map_err(|e| Error::InvalidParameter {
            source: Box::new(e),
            description: "Error while parsing a MAC address".to_owned(),
        })?;
        Ok(BleId::MacAddress(bdaddr))
    }
}

impl TryFrom<u64> for BleId {
    type Error = Error;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        let mac_address = BDAddr::try_from(value)?;
        Ok(Self::MacAddress(mac_address))
    }
}

impl From<BDAddr> for BleId {
    fn from(mac: BDAddr) -> Self {
        BleId::MacAddress(mac)
    }
}

impl Display for BleId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BleId::Name(name) => write!(f, "name={name}"),
            BleId::MacAddress(mac) => write!(f, "MAC={mac}"),
        }
    }
}

/// A Meshtastic device discovered via Bluetooth LE.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct BleDevice {
    /// The broadcast name of the device.
    pub name: Option<String>,
    /// The MAC address of the device.
    pub mac_address: BDAddr,
}

impl<'a> From<BleId> for Cow<'a, BleId> {
    fn from(ble_id: BleId) -> Self {
        Cow::Owned(ble_id)
    }
}

impl<'a> From<&'a BleId> for Cow<'a, BleId> {
    fn from(ble_id: &'a BleId) -> Self {
        Cow::Borrowed(ble_id)
    }
}

impl<'a> From<&BleDevice> for Cow<'a, BleId> {
    fn from(device: &BleDevice) -> Self {
        Cow::Owned(device.mac_address.into())
    }
}

impl<'a> From<BleDevice> for Cow<'a, BleId> {
    fn from(device: BleDevice) -> Self {
        Cow::Owned(device.mac_address.into())
    }
}

impl BleHandler {
    pub async fn new(ble_id: &BleId, scan_duration: Duration) -> Result<Self, Error> {
        let (radio, adapter) = Self::find_ble_radio(ble_id, scan_duration).await?;
        radio.connect().await.map_err(|e| Error::StreamBuildError {
            source: Box::new(e),
            description: format!("Failed to connect to the device {ble_id}"),
        })?;
        let [toradio_char, fromnum_char, fromradio_char] =
            Self::find_characteristics(&radio).await?;
        Ok(BleHandler {
            radio,
            adapter,
            toradio_char,
            fromradio_char,
            fromnum_char,
        })
    }

    async fn scan_peripherals(
        adapter: &Adapter,
        scan_duration: Duration,
    ) -> Result<Vec<Peripheral>, btleplug::Error> {
        adapter
            .start_scan(ScanFilter {
                services: vec![MSH_SERVICE],
            })
            .await?;
        tokio::time::sleep(scan_duration).await;
        adapter.peripherals().await
    }

    /// Scans for nearby Meshtastic devices and returns a list of peripherals that expose the
    /// Meshtastic service.
    ///
    /// This function searches for BLE devices that have the `MSH_SERVICE` UUID, which identifies
    /// them as Meshtastic devices. For each device found, it returns a tuple containing the
    /// `Peripheral` and the `Adapter` that can be used to connect to it.
    async fn available_peripherals(
        scan_duration: Duration,
    ) -> Result<Vec<(Peripheral, Adapter)>, Error> {
        let scan_error_fn = |e: btleplug::Error| Error::StreamBuildError {
            source: Box::new(e),
            description: "Failed to scan for BLE devices".to_owned(),
        };
        let manager = Manager::new().await.map_err(scan_error_fn)?;
        let adapters = manager.adapters().await.map_err(scan_error_fn)?;
        let mut available_peripherals = Vec::new();
        for adapter in &adapters {
            let peripherals = Self::scan_peripherals(adapter, scan_duration).await;
            match peripherals {
                Err(e) => {
                    error!("Error while scanning for meshtastic peripherals: {e:?}");
                    // We continue, as there can be another adapter that works
                    continue;
                }
                Ok(peripherals) => {
                    for peripheral in peripherals {
                        available_peripherals.push((peripheral, adapter.clone()));
                    }
                }
            }
        }

        Ok(available_peripherals)
    }

    /// Returns a list of all available Meshtastic BLE devices.
    ///
    /// This function scans for devices that expose the Meshtastic service UUID
    /// (`6ba1b218-15a8-461f-9fa8-5dcae273eafd`) and returns a list of [`BleDevice`]s
    /// that can be used to connect to them.
    pub async fn available_ble_devices(scan_duration: Duration) -> Result<Vec<BleDevice>, Error> {
        let peripherals = Self::available_peripherals(scan_duration).await?;
        let mut devices = Vec::new();
        for (p, _) in &peripherals {
            if let Ok(Some(properties)) = p.properties().await {
                devices.push(BleDevice {
                    name: properties.local_name,
                    mac_address: properties.address,
                });
            }
        }
        Ok(devices)
    }

    /// Finds a specific Meshtastic BLE radio matching the provided `BleId`.
    ///
    /// This function scans for available Meshtastic devices and attempts to find one that matches
    /// the given `BleId`. If a matching device is found, it returns a tuple containing the
    /// `Peripheral` and the `Adapter` required for connection.
    async fn find_ble_radio(
        ble_id: &BleId,
        scan_duration: Duration,
    ) -> Result<(Peripheral, Adapter), Error> {
        for (peripheral, adapter) in Self::available_peripherals(scan_duration).await? {
            if let Ok(Some(peripheral_properties)) = peripheral.properties().await {
                let matches = match ble_id {
                    BleId::Name(name) => peripheral_properties.local_name.as_ref() == Some(name),
                    BleId::MacAddress(mac) => peripheral_properties.address == *mac,
                };
                if matches {
                    return Ok((peripheral, adapter.clone()));
                }
            }
        }
        Err(Error::StreamBuildError {
            source: Box::new(BleConnectionError()),
            description: format!(
                "Failed to find {ble_id}, or meshtastic is not running on the device"
            ) + ", or it's already connected to a client.",
        })
    }

    /// Finds the 3 meshtastic characteristics: toradio, fromnum and fromradio. It returns them in
    /// this order.
    async fn find_characteristics(radio: &Peripheral) -> Result<[Characteristic; 3], Error> {
        radio
            .discover_services()
            .await
            .map_err(|e| Error::StreamBuildError {
                source: Box::new(e),
                description: "Failed to discover services".to_owned(),
            })?;
        let characteristics = radio.characteristics();
        let find_characteristic = |uuid| {
            characteristics
                .iter()
                .find(|c| c.uuid == uuid)
                .ok_or(Error::StreamBuildError {
                    source: Box::new(BleConnectionError()), // TODO
                    description: format!("Failed to find characteristic {uuid}"),
                })
        };

        Ok([
            find_characteristic(TORADIO)?.clone(),
            find_characteristic(FROMNUM)?.clone(),
            find_characteristic(FROMRADIO)?.clone(),
        ])
    }

    /// Writes a data buffer to the radio, skipping the first 4 bytes.
    ///
    /// The first 4 bytes of the buffer are ignored because they are not used in BLE communication.
    pub async fn write_to_radio(&self, buffer: &[u8]) -> Result<(), Error> {
        self.radio
            // TODO: remove the skipping of the first 4 bytes
            .write(&self.toradio_char, &buffer[4..], WriteType::WithResponse)
            .await
            .map_err(|e: btleplug::Error| {
                Error::InternalStreamError(InternalStreamError::StreamWriteError {
                    source: Box::new(e),
                })
            })
    }

    fn ble_read_error_fn(e: btleplug::Error) -> Error {
        Error::InternalStreamError(InternalStreamError::StreamReadError {
            source: Box::new(e),
        })
    }

    /// Reads the next message from the radio.
    ///
    /// This function reads data from the `fromradio` characteristic and returns it as a
    /// `RadioMessage`. A `RadioMessage` can be either a `Packet` containing the data or an `Eof`
    /// marker to indicate the end of the stream.
    pub async fn read_from_radio(&self) -> Result<RadioMessage, Error> {
        self.radio
            .read(&self.fromradio_char)
            .await
            .map_err(Self::ble_read_error_fn)
            .and_then(|data| {
                if data.is_empty() {
                    Ok(RadioMessage::Eof)
                } else {
                    format_data_packet(data.into()).map(RadioMessage::Packet)
                }
            })
    }

    fn parse_u32(data: Vec<u8>) -> Result<u32, Error> {
        let data = data.as_slice().try_into().map_err(|e| {
            Error::InternalStreamError(InternalStreamError::StreamReadError {
                source: Box::new(e),
            })
        })?;
        Ok(u32::from_le_bytes(data))
    }

    /// Reads a `u32` value from the `fromnum` characteristic.
    ///
    /// This characteristic indicates the number of packets available to be read from the
    /// `fromradio` characteristic.
    pub async fn read_fromnum(&self) -> Result<u32, Error> {
        let data = self
            .radio
            .read(&self.fromnum_char)
            .await
            .map_err(Self::ble_read_error_fn)?;
        if data.is_empty() {
            return Ok(0);
        }
        Self::parse_u32(data)
    }

    /// Returns an asynchronous stream of notifications from the `fromnum` characteristic.
    ///
    /// The stream contains `u32` values that indicate the number of packets available to be read.
    pub async fn notifications(&self) -> Result<BoxStream<'_, u32>, Error> {
        self.radio
            .subscribe(&self.fromnum_char)
            .await
            .map_err(Self::ble_read_error_fn)?;
        let notification_stream = self
            .radio
            .notifications()
            .await
            .map_err(Self::ble_read_error_fn)?;

        Ok(Box::pin(notification_stream.filter_map(
            |notification| match notification {
                ValueNotification {
                    uuid: FROMNUM,
                    value,
                } => future::ready(Self::parse_u32(value).ok()),
                _ => future::ready(None),
            },
        )))
    }

    /// Returns a stream of `AdapterEvent`s.
    ///
    /// Currently, the only supported event is `Disconnected`.
    pub async fn adapter_events(&self) -> Result<BoxStream<'_, AdapterEvent>, Error> {
        let stream = self
            .adapter
            .events()
            .await
            .map_err(|e| Error::StreamBuildError {
                source: Box::new(e),
                description: "Failed to listen to device events".to_owned(),
            })?;
        let id = self.radio.id();
        Ok(Box::pin(stream.filter_map(move |event| {
            if let CentralEvent::DeviceDisconnected(peripheral_id) = event {
                if id == peripheral_id {
                    return future::ready(Some(AdapterEvent::Disconnected));
                }
            }
            future::ready(None)
        })))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ble_id_to_cow() {
        let ble_id_name = BleId::from_name("TestDevice");
        let cow_from_ref: Cow<'_, BleId> = (&ble_id_name).into();
        assert_eq!(*cow_from_ref, ble_id_name);

        let cow_from_owned: Cow<'_, BleId> = ble_id_name.into();
        assert!(matches!(cow_from_owned, Cow::Owned(_)));

        let ble_id_mac = BleId::try_from(0x001122334455).unwrap();
        let cow_from_owned_mac: Cow<'_, BleId> = ble_id_mac.clone().into();
        assert_eq!(*cow_from_owned_mac, ble_id_mac);
    }

    #[test]
    fn test_ble_device_to_cow() {
        let ble_device = BleDevice {
            name: None,
            mac_address: BDAddr::try_from(0x001122334455).unwrap(),
        };

        let cow_from_ble_device_ref: Cow<'_, BleId> = (&ble_device).into();
        let ble_id_mac = BleId::try_from(0x001122334455).unwrap();
        assert_eq!(*cow_from_ble_device_ref, ble_id_mac);

        let cow_from_ble_device_owned: Cow<'_, BleId> = ble_device.into();
        assert_eq!(*cow_from_ble_device_owned, ble_id_mac);
    }
}
