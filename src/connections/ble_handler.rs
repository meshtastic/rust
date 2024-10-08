use btleplug::api::{
    Central, CentralEvent, Characteristic, Manager as _, Peripheral as _, ScanFilter,
    ValueNotification, WriteType,
};
use btleplug::platform::{Adapter, Manager, Peripheral};
use futures_util::stream::BoxStream;
use log::error;
use uuid::Uuid;

use crate::errors_internal::{BleConnectionError, Error, InternalStreamError};

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

#[allow(dead_code)]
impl BleHandler {
    pub async fn new(name: String) -> Result<Self, Error> {
        let (radio, adapter) = Self::find_ble_radio(&name).await?;
        radio.connect().await.map_err(|e| Error::StreamBuildError {
            source: Box::new(e),
            description: format!("Failed to connect to the device {name}"),
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

    async fn scan_peripherals(adapter: &Adapter) -> Result<Vec<Peripheral>, btleplug::Error> {
        adapter
            .start_scan(ScanFilter {
                services: vec![MSH_SERVICE],
            })
            .await?;
        adapter.peripherals().await
    }

    /// Finds a BLE radio matching a given name and running meshtastic.
    /// It searches for the 'MSH_SERVICE' running on the device.
    ///
    /// It also returns the associated adapter that can reach this radio.
    async fn find_ble_radio(name: &str) -> Result<(Peripheral, Adapter), Error> {
        //TODO: support searching both by a name and by a MAC address
        let scan_error_fn = |e: btleplug::Error| Error::StreamBuildError {
            source: Box::new(e),
            description: "Failed to scan for BLE devices".to_owned(),
        };
        let manager = Manager::new().await.map_err(scan_error_fn)?;
        let adapters = manager.adapters().await.map_err(scan_error_fn)?;

        for adapter in &adapters {
            let peripherals = Self::scan_peripherals(&adapter).await;
            match peripherals {
                Err(e) => {
                    error!("Error while scanning for meshtastic peripherals: {e:?}");
                    // We continue, as there can be another adapter that can work
                    continue;
                }
                Ok(peripherals) => {
                    let needle = Some(name.to_owned());
                    for peripheral in peripherals {
                        if let Ok(Some(peripheral_properties)) = peripheral.properties().await {
                            if peripheral_properties.local_name == needle {
                                return Ok((peripheral, adapter.clone()));
                            }
                        }
                    }
                }
            }
        }
        Err(Error::StreamBuildError {
            source: Box::new(BleConnectionError()),
            description: format!(
                "Failed to find {name}, or meshtastic is not running on the device"
            ) + ", or it's already connected.",
        })
    }

    /// Finds the 3 meshtastic characteristics: toradio, fromnum and fromradio. It returns them in this
    /// order.
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

    pub async fn read_from_radio(&self) -> Result<Vec<u8>, Error> {
        self.radio
            .read(&self.fromradio_char)
            .await
            .map_err(Self::ble_read_error_fn)
    }

    fn parse_u32(data: Vec<u8>) -> Result<u32, Error> {
        let parsed_value = u32::from_le_bytes(data.as_slice().try_into().map_err(|e| {
            Error::InternalStreamError(InternalStreamError::StreamReadError {
                source: Box::new(e),
            })
        })?);
        Ok(parsed_value)
    }

    pub async fn read_fromnum(&self) -> Result<u32, Error> {
        let data = self
            .radio
            .read(&self.fromnum_char)
            .await
            .map_err(Self::ble_read_error_fn)?;
        Self::parse_u32(data)
    }

    pub async fn notifications(&self) -> Result<BoxStream<ValueNotification>, Error> {
        self.radio
            .subscribe(&self.fromnum_char)
            .await
            .map_err(Self::ble_read_error_fn)?;
        self.radio
            .notifications()
            .await
            .map_err(Self::ble_read_error_fn)
    }

    pub async fn filter_map(notification: ValueNotification) -> Option<u32> {
        match notification {
            ValueNotification {
                uuid: FROMNUM,
                value,
            } => Some(Self::parse_u32(value).unwrap()),
            _ => None,
        }
    }

    pub async fn adapter_events(&self) -> Result<BoxStream<CentralEvent>, Error> {
        self.adapter
            .events()
            .await
            .map_err(|e| Error::StreamBuildError {
                source: Box::new(e),
                description: format!("Failed to listen to device events"),
            })
    }

    pub fn is_disconnected_event(&self, event: Option<CentralEvent>) -> bool {
        if let Some(CentralEvent::DeviceDisconnected(peripheral_id)) = event {
            return self.radio.id() == peripheral_id;
        }
        return false;
    }
}
