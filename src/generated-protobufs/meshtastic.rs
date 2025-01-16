///
/// This information can be encoded as a QRcode/url so that other users can configure
/// their radio to join the same channel.
/// A note about how channel names are shown to users: channelname-X
/// poundsymbol is a prefix used to indicate this is a channel name (idea from @professr).
/// Where X is a letter from A-Z (base 26) representing a hash of the PSK for this
/// channel - so that if the user changes anything about the channel (which does
/// force a new PSK) this letter will also change. Thus preventing user confusion if
/// two friends try to type in a channel name of "BobsChan" and then can't talk
/// because their PSKs will be different.
/// The PSK is hashed into this letter by "0x41 + [xor all bytes of the psk ] modulo 26"
/// This also allows the option of someday if people have the PSK off (zero), the
/// users COULD type in a channel name and be able to talk.
/// FIXME: Add description of multi-channel support and how primary vs secondary channels are used.
/// FIXME: explain how apps use channels for security.
/// explain how remote settings and remote gpio are managed as an example
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::doc_lazy_continuation)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ChannelSettings {
    ///
    /// Deprecated in favor of LoraConfig.channel_num
    #[deprecated]
    #[prost(uint32, tag = "1")]
    pub channel_num: u32,
    ///
    /// A simple pre-shared key for now for crypto.
    /// Must be either 0 bytes (no crypto), 16 bytes (AES128), or 32 bytes (AES256).
    /// A special shorthand is used for 1 byte long psks.
    /// These psks should be treated as only minimally secure,
    /// because they are listed in this source code.
    /// Those bytes are mapped using the following scheme:
    /// `0` = No crypto
    /// `1` = The special "default" channel key: {0xd4, 0xf1, 0xbb, 0x3a, 0x20, 0x29, 0x07, 0x59, 0xf0, 0xbc, 0xff, 0xab, 0xcf, 0x4e, 0x69, 0x01}
    /// `2` through 10 = The default channel key, except with 1 through 9 added to the last byte.
    /// Shown to user as simple1 through 10
    #[prost(bytes = "vec", tag = "2")]
    pub psk: ::prost::alloc::vec::Vec<u8>,
    ///
    /// A SHORT name that will be packed into the URL.
    /// Less than 12 bytes.
    /// Something for end users to call the channel
    /// If this is the empty string it is assumed that this channel
    /// is the special (minimally secure) "Default"channel.
    /// In user interfaces it should be rendered as a local language translation of "X".
    /// For channel_num hashing empty string will be treated as "X".
    /// Where "X" is selected based on the English words listed above for ModemPreset
    #[prost(string, tag = "3")]
    pub name: ::prost::alloc::string::String,
    ///
    /// Used to construct a globally unique channel ID.
    /// The full globally unique ID will be: "name.id" where ID is shown as base36.
    /// Assuming that the number of meshtastic users is below 20K (true for a long time)
    /// the chance of this 64 bit random number colliding with anyone else is super low.
    /// And the penalty for collision is low as well, it just means that anyone trying to decrypt channel messages might need to
    /// try multiple candidate channels.
    /// Any time a non wire compatible change is made to a channel, this field should be regenerated.
    /// There are a small number of 'special' globally known (and fairly) insecure standard channels.
    /// Those channels do not have a numeric id included in the settings, but instead it is pulled from
    /// a table of well known IDs.
    /// (see Well Known Channels FIXME)
    #[prost(fixed32, tag = "4")]
    pub id: u32,
    ///
    /// If true, messages on the mesh will be sent to the *public* internet by any gateway ndoe
    #[prost(bool, tag = "5")]
    pub uplink_enabled: bool,
    ///
    /// If true, messages seen on the internet will be forwarded to the local mesh.
    #[prost(bool, tag = "6")]
    pub downlink_enabled: bool,
    ///
    /// Per-channel module settings.
    #[prost(message, optional, tag = "7")]
    pub module_settings: ::core::option::Option<ModuleSettings>,
}
///
/// This message is specifically for modules to store per-channel configuration data.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::doc_lazy_continuation)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ModuleSettings {
    ///
    /// Bits of precision for the location sent in position packets.
    #[prost(uint32, tag = "1")]
    pub position_precision: u32,
}
///
/// A pair of a channel number, mode and the (sharable) settings for that channel
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::doc_lazy_continuation)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Channel {
    ///
    /// The index of this channel in the channel table (from 0 to MAX_NUM_CHANNELS-1)
    /// (Someday - not currently implemented) An index of -1 could be used to mean "set by name",
    /// in which case the target node will find and set the channel by settings.name.
    #[prost(int32, tag = "1")]
    pub index: i32,
    ///
    /// The new settings, or NULL to disable that channel
    #[prost(message, optional, tag = "2")]
    pub settings: ::core::option::Option<ChannelSettings>,
    ///
    /// TODO: REPLACE
    #[prost(enumeration = "channel::Role", tag = "3")]
    pub role: i32,
}
/// Nested message and enum types in `Channel`.
pub mod channel {
    ///
    /// How this channel is being used (or not).
    /// Note: this field is an enum to give us options for the future.
    /// In particular, someday we might make a 'SCANNING' option.
    /// SCANNING channels could have different frequencies and the radio would
    /// occasionally check that freq to see if anything is being transmitted.
    /// For devices that have multiple physical radios attached, we could keep multiple PRIMARY/SCANNING channels active at once to allow
    /// cross band routing as needed.
    /// If a device has only a single radio (the common case) only one channel can be PRIMARY at a time
    /// (but any number of SECONDARY channels can't be sent received on that common frequency)
    #[derive(serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    #[allow(clippy::doc_lazy_continuation)]
    #[derive(
        Clone,
        Copy,
        Debug,
        PartialEq,
        Eq,
        Hash,
        PartialOrd,
        Ord,
        ::prost::Enumeration
    )]
    #[repr(i32)]
    pub enum Role {
        ///
        /// This channel is not in use right now
        Disabled = 0,
        ///
        /// This channel is used to set the frequency for the radio - all other enabled channels must be SECONDARY
        Primary = 1,
        ///
        /// Secondary channels are only used for encryption/decryption/authentication purposes.
        /// Their radio settings (freq etc) are ignored, only psk is used.
        Secondary = 2,
    }
    impl Role {
        /// String value of the enum field names used in the ProtoBuf definition.
        ///
        /// The values are not transformed in any way and thus are considered stable
        /// (if the ProtoBuf definition does not change) and safe for programmatic use.
        pub fn as_str_name(&self) -> &'static str {
            match self {
                Role::Disabled => "DISABLED",
                Role::Primary => "PRIMARY",
                Role::Secondary => "SECONDARY",
            }
        }
        /// Creates an enum from field names used in the ProtoBuf definition.
        pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
            match value {
                "DISABLED" => Some(Self::Disabled),
                "PRIMARY" => Some(Self::Primary),
                "SECONDARY" => Some(Self::Secondary),
                _ => None,
            }
        }
    }
}
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::doc_lazy_continuation)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Config {
    ///
    /// Payload Variant
    #[prost(oneof = "config::PayloadVariant", tags = "1, 2, 3, 4, 5, 6, 7")]
    pub payload_variant: ::core::option::Option<config::PayloadVariant>,
}
/// Nested message and enum types in `Config`.
pub mod config {
    ///
    /// Configuration
    #[derive(serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    #[allow(clippy::doc_lazy_continuation)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct DeviceConfig {
        ///
        /// Sets the role of node
        #[prost(enumeration = "device_config::Role", tag = "1")]
        pub role: i32,
        ///
        /// Disabling this will disable the SerialConsole by not initilizing the StreamAPI
        #[prost(bool, tag = "2")]
        pub serial_enabled: bool,
        ///
        /// By default we turn off logging as soon as an API client connects (to keep shared serial link quiet).
        /// Set this to true to leave the debug log outputting even when API is active.
        #[prost(bool, tag = "3")]
        pub debug_log_enabled: bool,
        ///
        /// For boards without a hard wired button, this is the pin number that will be used
        /// Boards that have more than one button can swap the function with this one. defaults to BUTTON_PIN if defined.
        #[prost(uint32, tag = "4")]
        pub button_gpio: u32,
        ///
        /// For boards without a PWM buzzer, this is the pin number that will be used
        /// Defaults to PIN_BUZZER if defined.
        #[prost(uint32, tag = "5")]
        pub buzzer_gpio: u32,
        ///
        /// Sets the role of node
        #[prost(enumeration = "device_config::RebroadcastMode", tag = "6")]
        pub rebroadcast_mode: i32,
        ///
        /// Send our nodeinfo this often
        /// Defaults to 900 Seconds (15 minutes)
        #[prost(uint32, tag = "7")]
        pub node_info_broadcast_secs: u32,
        ///
        /// Treat double tap interrupt on supported accelerometers as a button press if set to true
        #[prost(bool, tag = "8")]
        pub double_tap_as_button_press: bool,
        ///
        /// If true, device is considered to be "managed" by a mesh administrator
        /// Clients should then limit available configuration and administrative options inside the user interface
        #[prost(bool, tag = "9")]
        pub is_managed: bool,
        ///
        /// Disables the triple-press of user button to enable or disable GPS
        #[prost(bool, tag = "10")]
        pub disable_triple_click: bool,
        ///
        /// POSIX Timezone definition string from <https://github.com/nayarsystems/posix_tz_db/blob/master/zones.csv.>
        #[prost(string, tag = "11")]
        pub tzdef: ::prost::alloc::string::String,
    }
    /// Nested message and enum types in `DeviceConfig`.
    pub mod device_config {
        ///
        /// Defines the device's role on the Mesh network
        #[derive(serde::Serialize, serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        #[allow(clippy::doc_lazy_continuation)]
        #[derive(
            Clone,
            Copy,
            Debug,
            PartialEq,
            Eq,
            Hash,
            PartialOrd,
            Ord,
            ::prost::Enumeration
        )]
        #[repr(i32)]
        pub enum Role {
            ///
            /// Description: App connected or stand alone messaging device.
            /// Technical Details: Default Role
            Client = 0,
            ///
            ///   Description: Device that does not forward packets from other devices.
            ClientMute = 1,
            ///
            /// Description: Infrastructure node for extending network coverage by relaying messages. Visible in Nodes list.
            /// Technical Details: Mesh packets will prefer to be routed over this node. This node will not be used by client apps.
            ///    The wifi radio and the oled screen will be put to sleep.
            ///    This mode may still potentially have higher power usage due to it's preference in message rebroadcasting on the mesh.
            Router = 2,
            ///
            /// Description: Combination of both ROUTER and CLIENT. Not for mobile devices.
            RouterClient = 3,
            ///
            /// Description: Infrastructure node for extending network coverage by relaying messages with minimal overhead. Not visible in Nodes list.
            /// Technical Details: Mesh packets will simply be rebroadcasted over this node. Nodes configured with this role will not originate NodeInfo, Position, Telemetry
            ///    or any other packet type. They will simply rebroadcast any mesh packets on the same frequency, channel num, spread factor, and coding rate.
            Repeater = 4,
            ///
            /// Description: Broadcasts GPS position packets as priority.
            /// Technical Details: Position Mesh packets will be prioritized higher and sent more frequently by default.
            ///    When used in conjunction with power.is_power_saving = true, nodes will wake up,
            ///    send position, and then sleep for position.position_broadcast_secs seconds.
            Tracker = 5,
            ///
            /// Description: Broadcasts telemetry packets as priority.
            /// Technical Details: Telemetry Mesh packets will be prioritized higher and sent more frequently by default.
            ///    When used in conjunction with power.is_power_saving = true, nodes will wake up,
            ///    send environment telemetry, and then sleep for telemetry.environment_update_interval seconds.
            Sensor = 6,
            ///
            /// Description: Optimized for ATAK system communication and reduces routine broadcasts.
            /// Technical Details: Used for nodes dedicated for connection to an ATAK EUD.
            ///     Turns off many of the routine broadcasts to favor CoT packet stream
            ///     from the Meshtastic ATAK plugin -> IMeshService -> Node
            Tak = 7,
            ///
            /// Description: Device that only broadcasts as needed for stealth or power savings.
            /// Technical Details: Used for nodes that "only speak when spoken to"
            ///     Turns all of the routine broadcasts but allows for ad-hoc communication
            ///     Still rebroadcasts, but with local only rebroadcast mode (known meshes only)
            ///     Can be used for clandestine operation or to dramatically reduce airtime / power consumption
            ClientHidden = 8,
            ///
            /// Description: Broadcasts location as message to default channel regularly for to assist with device recovery.
            /// Technical Details: Used to automatically send a text message to the mesh
            ///     with the current position of the device on a frequent interval:
            ///     "I'm lost! Position: lat / long"
            LostAndFound = 9,
            ///
            /// Description: Enables automatic TAK PLI broadcasts and reduces routine broadcasts.
            /// Technical Details: Turns off many of the routine broadcasts to favor ATAK CoT packet stream
            ///     and automatic TAK PLI (position location information) broadcasts.
            ///     Uses position module configuration to determine TAK PLI broadcast interval.
            TakTracker = 10,
        }
        impl Role {
            /// String value of the enum field names used in the ProtoBuf definition.
            ///
            /// The values are not transformed in any way and thus are considered stable
            /// (if the ProtoBuf definition does not change) and safe for programmatic use.
            pub fn as_str_name(&self) -> &'static str {
                match self {
                    Role::Client => "CLIENT",
                    Role::ClientMute => "CLIENT_MUTE",
                    Role::Router => "ROUTER",
                    Role::RouterClient => "ROUTER_CLIENT",
                    Role::Repeater => "REPEATER",
                    Role::Tracker => "TRACKER",
                    Role::Sensor => "SENSOR",
                    Role::Tak => "TAK",
                    Role::ClientHidden => "CLIENT_HIDDEN",
                    Role::LostAndFound => "LOST_AND_FOUND",
                    Role::TakTracker => "TAK_TRACKER",
                }
            }
            /// Creates an enum from field names used in the ProtoBuf definition.
            pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
                match value {
                    "CLIENT" => Some(Self::Client),
                    "CLIENT_MUTE" => Some(Self::ClientMute),
                    "ROUTER" => Some(Self::Router),
                    "ROUTER_CLIENT" => Some(Self::RouterClient),
                    "REPEATER" => Some(Self::Repeater),
                    "TRACKER" => Some(Self::Tracker),
                    "SENSOR" => Some(Self::Sensor),
                    "TAK" => Some(Self::Tak),
                    "CLIENT_HIDDEN" => Some(Self::ClientHidden),
                    "LOST_AND_FOUND" => Some(Self::LostAndFound),
                    "TAK_TRACKER" => Some(Self::TakTracker),
                    _ => None,
                }
            }
        }
        ///
        /// Defines the device's behavior for how messages are rebroadcast
        #[derive(serde::Serialize, serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        #[allow(clippy::doc_lazy_continuation)]
        #[derive(
            Clone,
            Copy,
            Debug,
            PartialEq,
            Eq,
            Hash,
            PartialOrd,
            Ord,
            ::prost::Enumeration
        )]
        #[repr(i32)]
        pub enum RebroadcastMode {
            ///
            /// Default behavior.
            /// Rebroadcast any observed message, if it was on our private channel or from another mesh with the same lora params.
            All = 0,
            ///
            /// Same as behavior as ALL but skips packet decoding and simply rebroadcasts them.
            /// Only available in Repeater role. Setting this on any other roles will result in ALL behavior.
            AllSkipDecoding = 1,
            ///
            /// Ignores observed messages from foreign meshes that are open or those which it cannot decrypt.
            /// Only rebroadcasts message on the nodes local primary / secondary channels.
            LocalOnly = 2,
            ///
            /// Ignores observed messages from foreign meshes like LOCAL_ONLY,
            /// but takes it step further by also ignoring messages from nodenums not in the node's known list (NodeDB)
            KnownOnly = 3,
        }
        impl RebroadcastMode {
            /// String value of the enum field names used in the ProtoBuf definition.
            ///
            /// The values are not transformed in any way and thus are considered stable
            /// (if the ProtoBuf definition does not change) and safe for programmatic use.
            pub fn as_str_name(&self) -> &'static str {
                match self {
                    RebroadcastMode::All => "ALL",
                    RebroadcastMode::AllSkipDecoding => "ALL_SKIP_DECODING",
                    RebroadcastMode::LocalOnly => "LOCAL_ONLY",
                    RebroadcastMode::KnownOnly => "KNOWN_ONLY",
                }
            }
            /// Creates an enum from field names used in the ProtoBuf definition.
            pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
                match value {
                    "ALL" => Some(Self::All),
                    "ALL_SKIP_DECODING" => Some(Self::AllSkipDecoding),
                    "LOCAL_ONLY" => Some(Self::LocalOnly),
                    "KNOWN_ONLY" => Some(Self::KnownOnly),
                    _ => None,
                }
            }
        }
    }
    ///
    /// Position Config
    #[derive(serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    #[allow(clippy::doc_lazy_continuation)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct PositionConfig {
        ///
        /// We should send our position this often (but only if it has changed significantly)
        /// Defaults to 15 minutes
        #[prost(uint32, tag = "1")]
        pub position_broadcast_secs: u32,
        ///
        /// Adaptive position braoadcast, which is now the default.
        #[prost(bool, tag = "2")]
        pub position_broadcast_smart_enabled: bool,
        ///
        /// If set, this node is at a fixed position.
        /// We will generate GPS position updates at the regular interval, but use whatever the last lat/lon/alt we have for the node.
        /// The lat/lon/alt can be set by an internal GPS or with the help of the app.
        #[prost(bool, tag = "3")]
        pub fixed_position: bool,
        ///
        /// Is GPS enabled for this node?
        #[deprecated]
        #[prost(bool, tag = "4")]
        pub gps_enabled: bool,
        ///
        /// How often should we try to get GPS position (in seconds)
        /// or zero for the default of once every 30 seconds
        /// or a very large value (maxint) to update only once at boot.
        #[prost(uint32, tag = "5")]
        pub gps_update_interval: u32,
        ///
        /// Deprecated in favor of using smart / regular broadcast intervals as implicit attempt time
        #[deprecated]
        #[prost(uint32, tag = "6")]
        pub gps_attempt_time: u32,
        ///
        /// Bit field of boolean configuration options for POSITION messages
        /// (bitwise OR of PositionFlags)
        #[prost(uint32, tag = "7")]
        pub position_flags: u32,
        ///
        /// (Re)define GPS_RX_PIN for your board.
        #[prost(uint32, tag = "8")]
        pub rx_gpio: u32,
        ///
        /// (Re)define GPS_TX_PIN for your board.
        #[prost(uint32, tag = "9")]
        pub tx_gpio: u32,
        ///
        /// The minimum distance in meters traveled (since the last send) before we can send a position to the mesh if position_broadcast_smart_enabled
        #[prost(uint32, tag = "10")]
        pub broadcast_smart_minimum_distance: u32,
        ///
        /// The minimum number of seconds (since the last send) before we can send a position to the mesh if position_broadcast_smart_enabled
        #[prost(uint32, tag = "11")]
        pub broadcast_smart_minimum_interval_secs: u32,
        ///
        /// (Re)define PIN_GPS_EN for your board.
        #[prost(uint32, tag = "12")]
        pub gps_en_gpio: u32,
        ///
        /// Set where GPS is enabled, disabled, or not present
        #[prost(enumeration = "position_config::GpsMode", tag = "13")]
        pub gps_mode: i32,
    }
    /// Nested message and enum types in `PositionConfig`.
    pub mod position_config {
        ///
        /// Bit field of boolean configuration options, indicating which optional
        /// fields to include when assembling POSITION messages.
        /// Longitude, latitude, altitude, speed, heading, and DOP
        /// are always included (also time if GPS-synced)
        /// NOTE: the more fields are included, the larger the message will be -
        ///    leading to longer airtime and a higher risk of packet loss
        #[derive(serde::Serialize, serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        #[allow(clippy::doc_lazy_continuation)]
        #[derive(
            Clone,
            Copy,
            Debug,
            PartialEq,
            Eq,
            Hash,
            PartialOrd,
            Ord,
            ::prost::Enumeration
        )]
        #[repr(i32)]
        pub enum PositionFlags {
            ///
            /// Required for compilation
            Unset = 0,
            ///
            /// Include an altitude value (if available)
            Altitude = 1,
            ///
            /// Altitude value is MSL
            AltitudeMsl = 2,
            ///
            /// Include geoidal separation
            GeoidalSeparation = 4,
            ///
            /// Include the DOP value ; PDOP used by default, see below
            Dop = 8,
            ///
            /// If POS_DOP set, send separate HDOP / VDOP values instead of PDOP
            Hvdop = 16,
            ///
            /// Include number of "satellites in view"
            Satinview = 32,
            ///
            /// Include a sequence number incremented per packet
            SeqNo = 64,
            ///
            /// Include positional timestamp (from GPS solution)
            Timestamp = 128,
            ///
            /// Include positional heading
            /// Intended for use with vehicle not walking speeds
            /// walking speeds are likely to be error prone like the compass
            Heading = 256,
            ///
            /// Include positional speed
            /// Intended for use with vehicle not walking speeds
            /// walking speeds are likely to be error prone like the compass
            Speed = 512,
        }
        impl PositionFlags {
            /// String value of the enum field names used in the ProtoBuf definition.
            ///
            /// The values are not transformed in any way and thus are considered stable
            /// (if the ProtoBuf definition does not change) and safe for programmatic use.
            pub fn as_str_name(&self) -> &'static str {
                match self {
                    PositionFlags::Unset => "UNSET",
                    PositionFlags::Altitude => "ALTITUDE",
                    PositionFlags::AltitudeMsl => "ALTITUDE_MSL",
                    PositionFlags::GeoidalSeparation => "GEOIDAL_SEPARATION",
                    PositionFlags::Dop => "DOP",
                    PositionFlags::Hvdop => "HVDOP",
                    PositionFlags::Satinview => "SATINVIEW",
                    PositionFlags::SeqNo => "SEQ_NO",
                    PositionFlags::Timestamp => "TIMESTAMP",
                    PositionFlags::Heading => "HEADING",
                    PositionFlags::Speed => "SPEED",
                }
            }
            /// Creates an enum from field names used in the ProtoBuf definition.
            pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
                match value {
                    "UNSET" => Some(Self::Unset),
                    "ALTITUDE" => Some(Self::Altitude),
                    "ALTITUDE_MSL" => Some(Self::AltitudeMsl),
                    "GEOIDAL_SEPARATION" => Some(Self::GeoidalSeparation),
                    "DOP" => Some(Self::Dop),
                    "HVDOP" => Some(Self::Hvdop),
                    "SATINVIEW" => Some(Self::Satinview),
                    "SEQ_NO" => Some(Self::SeqNo),
                    "TIMESTAMP" => Some(Self::Timestamp),
                    "HEADING" => Some(Self::Heading),
                    "SPEED" => Some(Self::Speed),
                    _ => None,
                }
            }
        }
        #[derive(serde::Serialize, serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        #[allow(clippy::doc_lazy_continuation)]
        #[derive(
            Clone,
            Copy,
            Debug,
            PartialEq,
            Eq,
            Hash,
            PartialOrd,
            Ord,
            ::prost::Enumeration
        )]
        #[repr(i32)]
        pub enum GpsMode {
            ///
            /// GPS is present but disabled
            Disabled = 0,
            ///
            /// GPS is present and enabled
            Enabled = 1,
            ///
            /// GPS is not present on the device
            NotPresent = 2,
        }
        impl GpsMode {
            /// String value of the enum field names used in the ProtoBuf definition.
            ///
            /// The values are not transformed in any way and thus are considered stable
            /// (if the ProtoBuf definition does not change) and safe for programmatic use.
            pub fn as_str_name(&self) -> &'static str {
                match self {
                    GpsMode::Disabled => "DISABLED",
                    GpsMode::Enabled => "ENABLED",
                    GpsMode::NotPresent => "NOT_PRESENT",
                }
            }
            /// Creates an enum from field names used in the ProtoBuf definition.
            pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
                match value {
                    "DISABLED" => Some(Self::Disabled),
                    "ENABLED" => Some(Self::Enabled),
                    "NOT_PRESENT" => Some(Self::NotPresent),
                    _ => None,
                }
            }
        }
    }
    ///
    /// Power Config\
    /// See [Power Config](/docs/settings/config/power) for additional power config details.
    #[derive(serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    #[allow(clippy::doc_lazy_continuation)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct PowerConfig {
        ///
        /// Description: Will sleep everything as much as possible, for the tracker and sensor role this will also include the lora radio.
        /// Don't use this setting if you want to use your device with the phone apps or are using a device without a user button.
        /// Technical Details: Works for ESP32 devices and NRF52 devices in the Sensor or Tracker roles
        #[prost(bool, tag = "1")]
        pub is_power_saving: bool,
        ///
        ///   Description: If non-zero, the device will fully power off this many seconds after external power is removed.
        #[prost(uint32, tag = "2")]
        pub on_battery_shutdown_after_secs: u32,
        ///
        /// Ratio of voltage divider for battery pin eg. 3.20 (R1=100k, R2=220k)
        /// Overrides the ADC_MULTIPLIER defined in variant for battery voltage calculation.
        /// <https://meshtastic.org/docs/configuration/radio/power/#adc-multiplier-override>
        /// Should be set to floating point value between 2 and 6
        #[prost(float, tag = "3")]
        pub adc_multiplier_override: f32,
        ///
        ///   Description: The number of seconds for to wait before turning off BLE in No Bluetooth states
        ///   Technical Details: ESP32 Only 0 for default of 1 minute
        #[prost(uint32, tag = "4")]
        pub wait_bluetooth_secs: u32,
        ///
        /// Super Deep Sleep Seconds
        /// While in Light Sleep if mesh_sds_timeout_secs is exceeded we will lower into super deep sleep
        /// for this value (default 1 year) or a button press
        /// 0 for default of one year
        #[prost(uint32, tag = "6")]
        pub sds_secs: u32,
        ///
        /// Description: In light sleep the CPU is suspended, LoRa radio is on, BLE is off an GPS is on
        /// Technical Details: ESP32 Only 0 for default of 300
        #[prost(uint32, tag = "7")]
        pub ls_secs: u32,
        ///
        /// Description: While in light sleep when we receive packets on the LoRa radio we will wake and handle them and stay awake in no BLE mode for this value
        /// Technical Details: ESP32 Only 0 for default of 10 seconds
        #[prost(uint32, tag = "8")]
        pub min_wake_secs: u32,
        ///
        /// I2C address of INA_2XX to use for reading device battery voltage
        #[prost(uint32, tag = "9")]
        pub device_battery_ina_address: u32,
    }
    ///
    /// Network Config
    #[derive(serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    #[allow(clippy::doc_lazy_continuation)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct NetworkConfig {
        ///
        /// Enable WiFi (disables Bluetooth)
        #[prost(bool, tag = "1")]
        pub wifi_enabled: bool,
        ///
        /// If set, this node will try to join the specified wifi network and
        /// acquire an address via DHCP
        #[prost(string, tag = "3")]
        pub wifi_ssid: ::prost::alloc::string::String,
        ///
        /// If set, will be use to authenticate to the named wifi
        #[prost(string, tag = "4")]
        pub wifi_psk: ::prost::alloc::string::String,
        ///
        /// NTP server to use if WiFi is conneced, defaults to `0.pool.ntp.org`
        #[prost(string, tag = "5")]
        pub ntp_server: ::prost::alloc::string::String,
        ///
        /// Enable Ethernet
        #[prost(bool, tag = "6")]
        pub eth_enabled: bool,
        ///
        /// acquire an address via DHCP or assign static
        #[prost(enumeration = "network_config::AddressMode", tag = "7")]
        pub address_mode: i32,
        ///
        /// struct to keep static address
        #[prost(message, optional, tag = "8")]
        pub ipv4_config: ::core::option::Option<network_config::IpV4Config>,
        ///
        /// rsyslog Server and Port
        #[prost(string, tag = "9")]
        pub rsyslog_server: ::prost::alloc::string::String,
    }
    /// Nested message and enum types in `NetworkConfig`.
    pub mod network_config {
        #[derive(serde::Serialize, serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        #[allow(clippy::doc_lazy_continuation)]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Message)]
        pub struct IpV4Config {
            ///
            /// Static IP address
            #[prost(fixed32, tag = "1")]
            pub ip: u32,
            ///
            /// Static gateway address
            #[prost(fixed32, tag = "2")]
            pub gateway: u32,
            ///
            /// Static subnet mask
            #[prost(fixed32, tag = "3")]
            pub subnet: u32,
            ///
            /// Static DNS server address
            #[prost(fixed32, tag = "4")]
            pub dns: u32,
        }
        #[derive(serde::Serialize, serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        #[allow(clippy::doc_lazy_continuation)]
        #[derive(
            Clone,
            Copy,
            Debug,
            PartialEq,
            Eq,
            Hash,
            PartialOrd,
            Ord,
            ::prost::Enumeration
        )]
        #[repr(i32)]
        pub enum AddressMode {
            ///
            /// obtain ip address via DHCP
            Dhcp = 0,
            ///
            /// use static ip address
            Static = 1,
        }
        impl AddressMode {
            /// String value of the enum field names used in the ProtoBuf definition.
            ///
            /// The values are not transformed in any way and thus are considered stable
            /// (if the ProtoBuf definition does not change) and safe for programmatic use.
            pub fn as_str_name(&self) -> &'static str {
                match self {
                    AddressMode::Dhcp => "DHCP",
                    AddressMode::Static => "STATIC",
                }
            }
            /// Creates an enum from field names used in the ProtoBuf definition.
            pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
                match value {
                    "DHCP" => Some(Self::Dhcp),
                    "STATIC" => Some(Self::Static),
                    _ => None,
                }
            }
        }
    }
    ///
    /// Display Config
    #[derive(serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    #[allow(clippy::doc_lazy_continuation)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct DisplayConfig {
        ///
        /// Number of seconds the screen stays on after pressing the user button or receiving a message
        /// 0 for default of one minute MAXUINT for always on
        #[prost(uint32, tag = "1")]
        pub screen_on_secs: u32,
        ///
        /// How the GPS coordinates are formatted on the OLED screen.
        #[prost(enumeration = "display_config::GpsCoordinateFormat", tag = "2")]
        pub gps_format: i32,
        ///
        /// Automatically toggles to the next page on the screen like a carousel, based the specified interval in seconds.
        /// Potentially useful for devices without user buttons.
        #[prost(uint32, tag = "3")]
        pub auto_screen_carousel_secs: u32,
        ///
        /// If this is set, the displayed compass will always point north. if unset, the old behaviour
        /// (top of display is heading direction) is used.
        #[prost(bool, tag = "4")]
        pub compass_north_top: bool,
        ///
        /// Flip screen vertically, for cases that mount the screen upside down
        #[prost(bool, tag = "5")]
        pub flip_screen: bool,
        ///
        /// Perferred display units
        #[prost(enumeration = "display_config::DisplayUnits", tag = "6")]
        pub units: i32,
        ///
        /// Override auto-detect in screen
        #[prost(enumeration = "display_config::OledType", tag = "7")]
        pub oled: i32,
        ///
        /// Display Mode
        #[prost(enumeration = "display_config::DisplayMode", tag = "8")]
        pub displaymode: i32,
        ///
        /// Print first line in pseudo-bold? FALSE is original style, TRUE is bold
        #[prost(bool, tag = "9")]
        pub heading_bold: bool,
        ///
        /// Should we wake the screen up on accelerometer detected motion or tap
        #[prost(bool, tag = "10")]
        pub wake_on_tap_or_motion: bool,
    }
    /// Nested message and enum types in `DisplayConfig`.
    pub mod display_config {
        ///
        /// How the GPS coordinates are displayed on the OLED screen.
        #[derive(serde::Serialize, serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        #[allow(clippy::doc_lazy_continuation)]
        #[derive(
            Clone,
            Copy,
            Debug,
            PartialEq,
            Eq,
            Hash,
            PartialOrd,
            Ord,
            ::prost::Enumeration
        )]
        #[repr(i32)]
        pub enum GpsCoordinateFormat {
            ///
            /// GPS coordinates are displayed in the normal decimal degrees format:
            /// DD.DDDDDD DDD.DDDDDD
            Dec = 0,
            ///
            /// GPS coordinates are displayed in the degrees minutes seconds format:
            /// DD°MM'SS"C DDD°MM'SS"C, where C is the compass point representing the locations quadrant
            Dms = 1,
            ///
            /// Universal Transverse Mercator format:
            /// ZZB EEEEEE NNNNNNN, where Z is zone, B is band, E is easting, N is northing
            Utm = 2,
            ///
            /// Military Grid Reference System format:
            /// ZZB CD EEEEE NNNNN, where Z is zone, B is band, C is the east 100k square, D is the north 100k square,
            /// E is easting, N is northing
            Mgrs = 3,
            ///
            /// Open Location Code (aka Plus Codes).
            Olc = 4,
            ///
            /// Ordnance Survey Grid Reference (the National Grid System of the UK).
            /// Format: AB EEEEE NNNNN, where A is the east 100k square, B is the north 100k square,
            /// E is the easting, N is the northing
            Osgr = 5,
        }
        impl GpsCoordinateFormat {
            /// String value of the enum field names used in the ProtoBuf definition.
            ///
            /// The values are not transformed in any way and thus are considered stable
            /// (if the ProtoBuf definition does not change) and safe for programmatic use.
            pub fn as_str_name(&self) -> &'static str {
                match self {
                    GpsCoordinateFormat::Dec => "DEC",
                    GpsCoordinateFormat::Dms => "DMS",
                    GpsCoordinateFormat::Utm => "UTM",
                    GpsCoordinateFormat::Mgrs => "MGRS",
                    GpsCoordinateFormat::Olc => "OLC",
                    GpsCoordinateFormat::Osgr => "OSGR",
                }
            }
            /// Creates an enum from field names used in the ProtoBuf definition.
            pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
                match value {
                    "DEC" => Some(Self::Dec),
                    "DMS" => Some(Self::Dms),
                    "UTM" => Some(Self::Utm),
                    "MGRS" => Some(Self::Mgrs),
                    "OLC" => Some(Self::Olc),
                    "OSGR" => Some(Self::Osgr),
                    _ => None,
                }
            }
        }
        ///
        /// Unit display preference
        #[derive(serde::Serialize, serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        #[allow(clippy::doc_lazy_continuation)]
        #[derive(
            Clone,
            Copy,
            Debug,
            PartialEq,
            Eq,
            Hash,
            PartialOrd,
            Ord,
            ::prost::Enumeration
        )]
        #[repr(i32)]
        pub enum DisplayUnits {
            ///
            /// Metric (Default)
            Metric = 0,
            ///
            /// Imperial
            Imperial = 1,
        }
        impl DisplayUnits {
            /// String value of the enum field names used in the ProtoBuf definition.
            ///
            /// The values are not transformed in any way and thus are considered stable
            /// (if the ProtoBuf definition does not change) and safe for programmatic use.
            pub fn as_str_name(&self) -> &'static str {
                match self {
                    DisplayUnits::Metric => "METRIC",
                    DisplayUnits::Imperial => "IMPERIAL",
                }
            }
            /// Creates an enum from field names used in the ProtoBuf definition.
            pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
                match value {
                    "METRIC" => Some(Self::Metric),
                    "IMPERIAL" => Some(Self::Imperial),
                    _ => None,
                }
            }
        }
        ///
        /// Override OLED outo detect with this if it fails.
        #[derive(serde::Serialize, serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        #[allow(clippy::doc_lazy_continuation)]
        #[derive(
            Clone,
            Copy,
            Debug,
            PartialEq,
            Eq,
            Hash,
            PartialOrd,
            Ord,
            ::prost::Enumeration
        )]
        #[repr(i32)]
        pub enum OledType {
            ///
            /// Default / Auto
            OledAuto = 0,
            ///
            /// Default / Auto
            OledSsd1306 = 1,
            ///
            /// Default / Auto
            OledSh1106 = 2,
            ///
            /// Can not be auto detected but set by proto. Used for 128x128 screens
            OledSh1107 = 3,
        }
        impl OledType {
            /// String value of the enum field names used in the ProtoBuf definition.
            ///
            /// The values are not transformed in any way and thus are considered stable
            /// (if the ProtoBuf definition does not change) and safe for programmatic use.
            pub fn as_str_name(&self) -> &'static str {
                match self {
                    OledType::OledAuto => "OLED_AUTO",
                    OledType::OledSsd1306 => "OLED_SSD1306",
                    OledType::OledSh1106 => "OLED_SH1106",
                    OledType::OledSh1107 => "OLED_SH1107",
                }
            }
            /// Creates an enum from field names used in the ProtoBuf definition.
            pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
                match value {
                    "OLED_AUTO" => Some(Self::OledAuto),
                    "OLED_SSD1306" => Some(Self::OledSsd1306),
                    "OLED_SH1106" => Some(Self::OledSh1106),
                    "OLED_SH1107" => Some(Self::OledSh1107),
                    _ => None,
                }
            }
        }
        #[derive(serde::Serialize, serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        #[allow(clippy::doc_lazy_continuation)]
        #[derive(
            Clone,
            Copy,
            Debug,
            PartialEq,
            Eq,
            Hash,
            PartialOrd,
            Ord,
            ::prost::Enumeration
        )]
        #[repr(i32)]
        pub enum DisplayMode {
            ///
            /// Default. The old style for the 128x64 OLED screen
            Default = 0,
            ///
            /// Rearrange display elements to cater for bicolor OLED displays
            Twocolor = 1,
            ///
            /// Same as TwoColor, but with inverted top bar. Not so good for Epaper displays
            Inverted = 2,
            ///
            /// TFT Full Color Displays (not implemented yet)
            Color = 3,
        }
        impl DisplayMode {
            /// String value of the enum field names used in the ProtoBuf definition.
            ///
            /// The values are not transformed in any way and thus are considered stable
            /// (if the ProtoBuf definition does not change) and safe for programmatic use.
            pub fn as_str_name(&self) -> &'static str {
                match self {
                    DisplayMode::Default => "DEFAULT",
                    DisplayMode::Twocolor => "TWOCOLOR",
                    DisplayMode::Inverted => "INVERTED",
                    DisplayMode::Color => "COLOR",
                }
            }
            /// Creates an enum from field names used in the ProtoBuf definition.
            pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
                match value {
                    "DEFAULT" => Some(Self::Default),
                    "TWOCOLOR" => Some(Self::Twocolor),
                    "INVERTED" => Some(Self::Inverted),
                    "COLOR" => Some(Self::Color),
                    _ => None,
                }
            }
        }
    }
    ///
    /// Lora Config
    #[derive(serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    #[allow(clippy::doc_lazy_continuation)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct LoRaConfig {
        ///
        /// When enabled, the `modem_preset` fields will be adhered to, else the `bandwidth`/`spread_factor`/`coding_rate`
        /// will be taked from their respective manually defined fields
        #[prost(bool, tag = "1")]
        pub use_preset: bool,
        ///
        /// Either modem_config or bandwidth/spreading/coding will be specified - NOT BOTH.
        /// As a heuristic: If bandwidth is specified, do not use modem_config.
        /// Because protobufs take ZERO space when the value is zero this works out nicely.
        /// This value is replaced by bandwidth/spread_factor/coding_rate.
        /// If you'd like to experiment with other options add them to MeshRadio.cpp in the device code.
        #[prost(enumeration = "lo_ra_config::ModemPreset", tag = "2")]
        pub modem_preset: i32,
        ///
        /// Bandwidth in MHz
        /// Certain bandwidth numbers are 'special' and will be converted to the
        /// appropriate floating point value: 31 -> 31.25MHz
        #[prost(uint32, tag = "3")]
        pub bandwidth: u32,
        ///
        /// A number from 7 to 12.
        /// Indicates number of chirps per symbol as 1<<spread_factor.
        #[prost(uint32, tag = "4")]
        pub spread_factor: u32,
        ///
        /// The denominator of the coding rate.
        /// ie for 4/5, the value is 5. 4/8 the value is 8.
        #[prost(uint32, tag = "5")]
        pub coding_rate: u32,
        ///
        /// This parameter is for advanced users with advanced test equipment, we do not recommend most users use it.
        /// A frequency offset that is added to to the calculated band center frequency.
        /// Used to correct for crystal calibration errors.
        #[prost(float, tag = "6")]
        pub frequency_offset: f32,
        ///
        /// The region code for the radio (US, CN, EU433, etc...)
        #[prost(enumeration = "lo_ra_config::RegionCode", tag = "7")]
        pub region: i32,
        ///
        /// Maximum number of hops. This can't be greater than 7.
        /// Default of 3
        /// Attempting to set a value > 7 results in the default
        #[prost(uint32, tag = "8")]
        pub hop_limit: u32,
        ///
        /// Disable TX from the LoRa radio. Useful for hot-swapping antennas and other tests.
        /// Defaults to false
        #[prost(bool, tag = "9")]
        pub tx_enabled: bool,
        ///
        /// If zero, then use default max legal continuous power (ie. something that won't
        /// burn out the radio hardware)
        /// In most cases you should use zero here.
        /// Units are in dBm.
        #[prost(int32, tag = "10")]
        pub tx_power: i32,
        ///
        /// This controls the actual hardware frequency the radio transmits on.
        /// Most users should never need to be exposed to this field/concept.
        /// A channel number between 1 and NUM_CHANNELS (whatever the max is in the current region).
        /// If ZERO then the rule is "use the old channel name hash based
        /// algorithm to derive the channel number")
        /// If using the hash algorithm the channel number will be: hash(channel_name) %
        /// NUM_CHANNELS (Where num channels depends on the regulatory region).
        #[prost(uint32, tag = "11")]
        pub channel_num: u32,
        ///
        /// If true, duty cycle limits will be exceeded and thus you're possibly not following
        /// the local regulations if you're not a HAM.
        /// Has no effect if the duty cycle of the used region is 100%.
        #[prost(bool, tag = "12")]
        pub override_duty_cycle: bool,
        ///
        /// If true, sets RX boosted gain mode on SX126X based radios
        #[prost(bool, tag = "13")]
        pub sx126x_rx_boosted_gain: bool,
        ///
        /// This parameter is for advanced users and licensed HAM radio operators.
        /// Ignore Channel Calculation and use this frequency instead. The frequency_offset
        /// will still be applied. This will allow you to use out-of-band frequencies.
        /// Please respect your local laws and regulations. If you are a HAM, make sure you
        /// enable HAM mode and turn off encryption.
        #[prost(float, tag = "14")]
        pub override_frequency: f32,
        ///
        /// For testing it is useful sometimes to force a node to never listen to
        /// particular other nodes (simulating radio out of range). All nodenums listed
        /// in ignore_incoming will have packets they send dropped on receive (by router.cpp)
        #[prost(uint32, repeated, tag = "103")]
        pub ignore_incoming: ::prost::alloc::vec::Vec<u32>,
        ///
        /// If true, the device will not process any packets received via LoRa that passed via MQTT anywhere on the path towards it.
        #[prost(bool, tag = "104")]
        pub ignore_mqtt: bool,
    }
    /// Nested message and enum types in `LoRaConfig`.
    pub mod lo_ra_config {
        #[derive(serde::Serialize, serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        #[allow(clippy::doc_lazy_continuation)]
        #[derive(
            Clone,
            Copy,
            Debug,
            PartialEq,
            Eq,
            Hash,
            PartialOrd,
            Ord,
            ::prost::Enumeration
        )]
        #[repr(i32)]
        pub enum RegionCode {
            ///
            /// Region is not set
            Unset = 0,
            ///
            /// United States
            Us = 1,
            ///
            /// European Union 433mhz
            Eu433 = 2,
            ///
            /// European Union 868mhz
            Eu868 = 3,
            ///
            /// China
            Cn = 4,
            ///
            /// Japan
            Jp = 5,
            ///
            /// Australia / New Zealand
            Anz = 6,
            ///
            /// Korea
            Kr = 7,
            ///
            /// Taiwan
            Tw = 8,
            ///
            /// Russia
            Ru = 9,
            ///
            /// India
            In = 10,
            ///
            /// New Zealand 865mhz
            Nz865 = 11,
            ///
            /// Thailand
            Th = 12,
            ///
            /// WLAN Band
            Lora24 = 13,
            ///
            /// Ukraine 433mhz
            Ua433 = 14,
            ///
            /// Ukraine 868mhz
            Ua868 = 15,
            ///
            /// Malaysia 433mhz
            My433 = 16,
            ///
            /// Malaysia 919mhz
            My919 = 17,
            ///
            /// Singapore 923mhz
            Sg923 = 18,
        }
        impl RegionCode {
            /// String value of the enum field names used in the ProtoBuf definition.
            ///
            /// The values are not transformed in any way and thus are considered stable
            /// (if the ProtoBuf definition does not change) and safe for programmatic use.
            pub fn as_str_name(&self) -> &'static str {
                match self {
                    RegionCode::Unset => "UNSET",
                    RegionCode::Us => "US",
                    RegionCode::Eu433 => "EU_433",
                    RegionCode::Eu868 => "EU_868",
                    RegionCode::Cn => "CN",
                    RegionCode::Jp => "JP",
                    RegionCode::Anz => "ANZ",
                    RegionCode::Kr => "KR",
                    RegionCode::Tw => "TW",
                    RegionCode::Ru => "RU",
                    RegionCode::In => "IN",
                    RegionCode::Nz865 => "NZ_865",
                    RegionCode::Th => "TH",
                    RegionCode::Lora24 => "LORA_24",
                    RegionCode::Ua433 => "UA_433",
                    RegionCode::Ua868 => "UA_868",
                    RegionCode::My433 => "MY_433",
                    RegionCode::My919 => "MY_919",
                    RegionCode::Sg923 => "SG_923",
                }
            }
            /// Creates an enum from field names used in the ProtoBuf definition.
            pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
                match value {
                    "UNSET" => Some(Self::Unset),
                    "US" => Some(Self::Us),
                    "EU_433" => Some(Self::Eu433),
                    "EU_868" => Some(Self::Eu868),
                    "CN" => Some(Self::Cn),
                    "JP" => Some(Self::Jp),
                    "ANZ" => Some(Self::Anz),
                    "KR" => Some(Self::Kr),
                    "TW" => Some(Self::Tw),
                    "RU" => Some(Self::Ru),
                    "IN" => Some(Self::In),
                    "NZ_865" => Some(Self::Nz865),
                    "TH" => Some(Self::Th),
                    "LORA_24" => Some(Self::Lora24),
                    "UA_433" => Some(Self::Ua433),
                    "UA_868" => Some(Self::Ua868),
                    "MY_433" => Some(Self::My433),
                    "MY_919" => Some(Self::My919),
                    "SG_923" => Some(Self::Sg923),
                    _ => None,
                }
            }
        }
        ///
        /// Standard predefined channel settings
        /// Note: these mappings must match ModemPreset Choice in the device code.
        #[derive(serde::Serialize, serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        #[allow(clippy::doc_lazy_continuation)]
        #[derive(
            Clone,
            Copy,
            Debug,
            PartialEq,
            Eq,
            Hash,
            PartialOrd,
            Ord,
            ::prost::Enumeration
        )]
        #[repr(i32)]
        pub enum ModemPreset {
            ///
            /// Long Range - Fast
            LongFast = 0,
            ///
            /// Long Range - Slow
            LongSlow = 1,
            ///
            /// Very Long Range - Slow
            VeryLongSlow = 2,
            ///
            /// Medium Range - Slow
            MediumSlow = 3,
            ///
            /// Medium Range - Fast
            MediumFast = 4,
            ///
            /// Short Range - Slow
            ShortSlow = 5,
            ///
            /// Short Range - Fast
            ShortFast = 6,
            ///
            /// Long Range - Moderately Fast
            LongModerate = 7,
        }
        impl ModemPreset {
            /// String value of the enum field names used in the ProtoBuf definition.
            ///
            /// The values are not transformed in any way and thus are considered stable
            /// (if the ProtoBuf definition does not change) and safe for programmatic use.
            pub fn as_str_name(&self) -> &'static str {
                match self {
                    ModemPreset::LongFast => "LONG_FAST",
                    ModemPreset::LongSlow => "LONG_SLOW",
                    ModemPreset::VeryLongSlow => "VERY_LONG_SLOW",
                    ModemPreset::MediumSlow => "MEDIUM_SLOW",
                    ModemPreset::MediumFast => "MEDIUM_FAST",
                    ModemPreset::ShortSlow => "SHORT_SLOW",
                    ModemPreset::ShortFast => "SHORT_FAST",
                    ModemPreset::LongModerate => "LONG_MODERATE",
                }
            }
            /// Creates an enum from field names used in the ProtoBuf definition.
            pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
                match value {
                    "LONG_FAST" => Some(Self::LongFast),
                    "LONG_SLOW" => Some(Self::LongSlow),
                    "VERY_LONG_SLOW" => Some(Self::VeryLongSlow),
                    "MEDIUM_SLOW" => Some(Self::MediumSlow),
                    "MEDIUM_FAST" => Some(Self::MediumFast),
                    "SHORT_SLOW" => Some(Self::ShortSlow),
                    "SHORT_FAST" => Some(Self::ShortFast),
                    "LONG_MODERATE" => Some(Self::LongModerate),
                    _ => None,
                }
            }
        }
    }
    #[derive(serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    #[allow(clippy::doc_lazy_continuation)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct BluetoothConfig {
        ///
        /// Enable Bluetooth on the device
        #[prost(bool, tag = "1")]
        pub enabled: bool,
        ///
        /// Determines the pairing strategy for the device
        #[prost(enumeration = "bluetooth_config::PairingMode", tag = "2")]
        pub mode: i32,
        ///
        /// Specified PIN for PairingMode.FixedPin
        #[prost(uint32, tag = "3")]
        pub fixed_pin: u32,
    }
    /// Nested message and enum types in `BluetoothConfig`.
    pub mod bluetooth_config {
        #[derive(serde::Serialize, serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        #[allow(clippy::doc_lazy_continuation)]
        #[derive(
            Clone,
            Copy,
            Debug,
            PartialEq,
            Eq,
            Hash,
            PartialOrd,
            Ord,
            ::prost::Enumeration
        )]
        #[repr(i32)]
        pub enum PairingMode {
            ///
            /// Device generates a random PIN that will be shown on the screen of the device for pairing
            RandomPin = 0,
            ///
            /// Device requires a specified fixed PIN for pairing
            FixedPin = 1,
            ///
            /// Device requires no PIN for pairing
            NoPin = 2,
        }
        impl PairingMode {
            /// String value of the enum field names used in the ProtoBuf definition.
            ///
            /// The values are not transformed in any way and thus are considered stable
            /// (if the ProtoBuf definition does not change) and safe for programmatic use.
            pub fn as_str_name(&self) -> &'static str {
                match self {
                    PairingMode::RandomPin => "RANDOM_PIN",
                    PairingMode::FixedPin => "FIXED_PIN",
                    PairingMode::NoPin => "NO_PIN",
                }
            }
            /// Creates an enum from field names used in the ProtoBuf definition.
            pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
                match value {
                    "RANDOM_PIN" => Some(Self::RandomPin),
                    "FIXED_PIN" => Some(Self::FixedPin),
                    "NO_PIN" => Some(Self::NoPin),
                    _ => None,
                }
            }
        }
    }
    ///
    /// Payload Variant
    #[derive(serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    #[allow(clippy::doc_lazy_continuation)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum PayloadVariant {
        #[prost(message, tag = "1")]
        Device(DeviceConfig),
        #[prost(message, tag = "2")]
        Position(PositionConfig),
        #[prost(message, tag = "3")]
        Power(PowerConfig),
        #[prost(message, tag = "4")]
        Network(NetworkConfig),
        #[prost(message, tag = "5")]
        Display(DisplayConfig),
        #[prost(message, tag = "6")]
        Lora(LoRaConfig),
        #[prost(message, tag = "7")]
        Bluetooth(BluetoothConfig),
    }
}
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::doc_lazy_continuation)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DeviceConnectionStatus {
    ///
    /// WiFi Status
    #[prost(message, optional, tag = "1")]
    pub wifi: ::core::option::Option<WifiConnectionStatus>,
    ///
    /// WiFi Status
    #[prost(message, optional, tag = "2")]
    pub ethernet: ::core::option::Option<EthernetConnectionStatus>,
    ///
    /// Bluetooth Status
    #[prost(message, optional, tag = "3")]
    pub bluetooth: ::core::option::Option<BluetoothConnectionStatus>,
    ///
    /// Serial Status
    #[prost(message, optional, tag = "4")]
    pub serial: ::core::option::Option<SerialConnectionStatus>,
}
///
/// WiFi connection status
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::doc_lazy_continuation)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WifiConnectionStatus {
    ///
    /// Connection status
    #[prost(message, optional, tag = "1")]
    pub status: ::core::option::Option<NetworkConnectionStatus>,
    ///
    /// WiFi access point SSID
    #[prost(string, tag = "2")]
    pub ssid: ::prost::alloc::string::String,
    ///
    /// RSSI of wireless connection
    #[prost(int32, tag = "3")]
    pub rssi: i32,
}
///
/// Ethernet connection status
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::doc_lazy_continuation)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct EthernetConnectionStatus {
    ///
    /// Connection status
    #[prost(message, optional, tag = "1")]
    pub status: ::core::option::Option<NetworkConnectionStatus>,
}
///
/// Ethernet or WiFi connection status
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::doc_lazy_continuation)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct NetworkConnectionStatus {
    ///
    /// IP address of device
    #[prost(fixed32, tag = "1")]
    pub ip_address: u32,
    ///
    /// Whether the device has an active connection or not
    #[prost(bool, tag = "2")]
    pub is_connected: bool,
    ///
    /// Whether the device has an active connection to an MQTT broker or not
    #[prost(bool, tag = "3")]
    pub is_mqtt_connected: bool,
    ///
    /// Whether the device is actively remote syslogging or not
    #[prost(bool, tag = "4")]
    pub is_syslog_connected: bool,
}
///
/// Bluetooth connection status
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::doc_lazy_continuation)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BluetoothConnectionStatus {
    ///
    /// The pairing PIN for bluetooth
    #[prost(uint32, tag = "1")]
    pub pin: u32,
    ///
    /// RSSI of bluetooth connection
    #[prost(int32, tag = "2")]
    pub rssi: i32,
    ///
    /// Whether the device has an active connection or not
    #[prost(bool, tag = "3")]
    pub is_connected: bool,
}
///
/// Serial connection status
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::doc_lazy_continuation)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SerialConnectionStatus {
    ///
    /// Serial baud rate
    #[prost(uint32, tag = "1")]
    pub baud: u32,
    ///
    /// Whether the device has an active connection or not
    #[prost(bool, tag = "2")]
    pub is_connected: bool,
}
///
/// Module Config
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::doc_lazy_continuation)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ModuleConfig {
    ///
    /// TODO: REPLACE
    #[prost(
        oneof = "module_config::PayloadVariant",
        tags = "1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13"
    )]
    pub payload_variant: ::core::option::Option<module_config::PayloadVariant>,
}
/// Nested message and enum types in `ModuleConfig`.
pub mod module_config {
    ///
    /// MQTT Client Config
    #[derive(serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    #[allow(clippy::doc_lazy_continuation)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct MqttConfig {
        ///
        /// If a meshtastic node is able to reach the internet it will normally attempt to gateway any channels that are marked as
        /// is_uplink_enabled or is_downlink_enabled.
        #[prost(bool, tag = "1")]
        pub enabled: bool,
        ///
        /// The server to use for our MQTT global message gateway feature.
        /// If not set, the default server will be used
        #[prost(string, tag = "2")]
        pub address: ::prost::alloc::string::String,
        ///
        /// MQTT username to use (most useful for a custom MQTT server).
        /// If using a custom server, this will be honoured even if empty.
        /// If using the default server, this will only be honoured if set, otherwise the device will use the default username
        #[prost(string, tag = "3")]
        pub username: ::prost::alloc::string::String,
        ///
        /// MQTT password to use (most useful for a custom MQTT server).
        /// If using a custom server, this will be honoured even if empty.
        /// If using the default server, this will only be honoured if set, otherwise the device will use the default password
        #[prost(string, tag = "4")]
        pub password: ::prost::alloc::string::String,
        ///
        /// Whether to send encrypted or decrypted packets to MQTT.
        /// This parameter is only honoured if you also set server
        /// (the default official mqtt.meshtastic.org server can handle encrypted packets)
        /// Decrypted packets may be useful for external systems that want to consume meshtastic packets
        #[prost(bool, tag = "5")]
        pub encryption_enabled: bool,
        ///
        /// Whether to send / consume json packets on MQTT
        #[prost(bool, tag = "6")]
        pub json_enabled: bool,
        ///
        /// If true, we attempt to establish a secure connection using TLS
        #[prost(bool, tag = "7")]
        pub tls_enabled: bool,
        ///
        /// The root topic to use for MQTT messages. Default is "msh".
        /// This is useful if you want to use a single MQTT server for multiple meshtastic networks and separate them via ACLs
        #[prost(string, tag = "8")]
        pub root: ::prost::alloc::string::String,
        ///
        /// If true, we can use the connected phone / client to proxy messages to MQTT instead of a direct connection
        #[prost(bool, tag = "9")]
        pub proxy_to_client_enabled: bool,
        ///
        /// If true, we will periodically report unencrypted information about our node to a map via MQTT
        #[prost(bool, tag = "10")]
        pub map_reporting_enabled: bool,
        ///
        /// Settings for reporting information about our node to a map via MQTT
        #[prost(message, optional, tag = "11")]
        pub map_report_settings: ::core::option::Option<MapReportSettings>,
    }
    ///
    /// Settings for reporting unencrypted information about our node to a map via MQTT
    #[derive(serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    #[allow(clippy::doc_lazy_continuation)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct MapReportSettings {
        ///
        /// How often we should report our info to the map (in seconds)
        #[prost(uint32, tag = "1")]
        pub publish_interval_secs: u32,
        ///
        /// Bits of precision for the location sent (default of 32 is full precision).
        #[prost(uint32, tag = "2")]
        pub position_precision: u32,
    }
    ///
    /// RemoteHardwareModule Config
    #[derive(serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    #[allow(clippy::doc_lazy_continuation)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct RemoteHardwareConfig {
        ///
        /// Whether the Module is enabled
        #[prost(bool, tag = "1")]
        pub enabled: bool,
        ///
        /// Whether the Module allows consumers to read / write to pins not defined in available_pins
        #[prost(bool, tag = "2")]
        pub allow_undefined_pin_access: bool,
        ///
        /// Exposes the available pins to the mesh for reading and writing
        #[prost(message, repeated, tag = "3")]
        pub available_pins: ::prost::alloc::vec::Vec<super::RemoteHardwarePin>,
    }
    ///
    /// NeighborInfoModule Config
    #[derive(serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    #[allow(clippy::doc_lazy_continuation)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct NeighborInfoConfig {
        ///
        /// Whether the Module is enabled
        #[prost(bool, tag = "1")]
        pub enabled: bool,
        ///
        /// Interval in seconds of how often we should try to send our
        /// Neighbor Info to the mesh
        #[prost(uint32, tag = "2")]
        pub update_interval: u32,
    }
    ///
    /// Detection Sensor Module Config
    #[derive(serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    #[allow(clippy::doc_lazy_continuation)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct DetectionSensorConfig {
        ///
        /// Whether the Module is enabled
        #[prost(bool, tag = "1")]
        pub enabled: bool,
        ///
        /// Interval in seconds of how often we can send a message to the mesh when a state change is detected
        #[prost(uint32, tag = "2")]
        pub minimum_broadcast_secs: u32,
        ///
        /// Interval in seconds of how often we should send a message to the mesh with the current state regardless of changes
        /// When set to 0, only state changes will be broadcasted
        /// Works as a sort of status heartbeat for peace of mind
        #[prost(uint32, tag = "3")]
        pub state_broadcast_secs: u32,
        ///
        /// Send ASCII bell with alert message
        /// Useful for triggering ext. notification on bell
        #[prost(bool, tag = "4")]
        pub send_bell: bool,
        ///
        /// Friendly name used to format message sent to mesh
        /// Example: A name "Motion" would result in a message "Motion detected"
        /// Maximum length of 20 characters
        #[prost(string, tag = "5")]
        pub name: ::prost::alloc::string::String,
        ///
        /// GPIO pin to monitor for state changes
        #[prost(uint32, tag = "6")]
        pub monitor_pin: u32,
        ///
        /// Whether or not the GPIO pin state detection is triggered on HIGH (1)
        /// Otherwise LOW (0)
        #[prost(bool, tag = "7")]
        pub detection_triggered_high: bool,
        ///
        /// Whether or not use INPUT_PULLUP mode for GPIO pin
        /// Only applicable if the board uses pull-up resistors on the pin
        #[prost(bool, tag = "8")]
        pub use_pullup: bool,
    }
    ///
    /// Audio Config for codec2 voice
    #[derive(serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    #[allow(clippy::doc_lazy_continuation)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct AudioConfig {
        ///
        /// Whether Audio is enabled
        #[prost(bool, tag = "1")]
        pub codec2_enabled: bool,
        ///
        /// PTT Pin
        #[prost(uint32, tag = "2")]
        pub ptt_pin: u32,
        ///
        /// The audio sample rate to use for codec2
        #[prost(enumeration = "audio_config::AudioBaud", tag = "3")]
        pub bitrate: i32,
        ///
        /// I2S Word Select
        #[prost(uint32, tag = "4")]
        pub i2s_ws: u32,
        ///
        /// I2S Data IN
        #[prost(uint32, tag = "5")]
        pub i2s_sd: u32,
        ///
        /// I2S Data OUT
        #[prost(uint32, tag = "6")]
        pub i2s_din: u32,
        ///
        /// I2S Clock
        #[prost(uint32, tag = "7")]
        pub i2s_sck: u32,
    }
    /// Nested message and enum types in `AudioConfig`.
    pub mod audio_config {
        ///
        /// Baudrate for codec2 voice
        #[derive(serde::Serialize, serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        #[allow(clippy::doc_lazy_continuation)]
        #[derive(
            Clone,
            Copy,
            Debug,
            PartialEq,
            Eq,
            Hash,
            PartialOrd,
            Ord,
            ::prost::Enumeration
        )]
        #[repr(i32)]
        pub enum AudioBaud {
            Codec2Default = 0,
            Codec23200 = 1,
            Codec22400 = 2,
            Codec21600 = 3,
            Codec21400 = 4,
            Codec21300 = 5,
            Codec21200 = 6,
            Codec2700 = 7,
            Codec2700b = 8,
        }
        impl AudioBaud {
            /// String value of the enum field names used in the ProtoBuf definition.
            ///
            /// The values are not transformed in any way and thus are considered stable
            /// (if the ProtoBuf definition does not change) and safe for programmatic use.
            pub fn as_str_name(&self) -> &'static str {
                match self {
                    AudioBaud::Codec2Default => "CODEC2_DEFAULT",
                    AudioBaud::Codec23200 => "CODEC2_3200",
                    AudioBaud::Codec22400 => "CODEC2_2400",
                    AudioBaud::Codec21600 => "CODEC2_1600",
                    AudioBaud::Codec21400 => "CODEC2_1400",
                    AudioBaud::Codec21300 => "CODEC2_1300",
                    AudioBaud::Codec21200 => "CODEC2_1200",
                    AudioBaud::Codec2700 => "CODEC2_700",
                    AudioBaud::Codec2700b => "CODEC2_700B",
                }
            }
            /// Creates an enum from field names used in the ProtoBuf definition.
            pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
                match value {
                    "CODEC2_DEFAULT" => Some(Self::Codec2Default),
                    "CODEC2_3200" => Some(Self::Codec23200),
                    "CODEC2_2400" => Some(Self::Codec22400),
                    "CODEC2_1600" => Some(Self::Codec21600),
                    "CODEC2_1400" => Some(Self::Codec21400),
                    "CODEC2_1300" => Some(Self::Codec21300),
                    "CODEC2_1200" => Some(Self::Codec21200),
                    "CODEC2_700" => Some(Self::Codec2700),
                    "CODEC2_700B" => Some(Self::Codec2700b),
                    _ => None,
                }
            }
        }
    }
    ///
    /// Config for the Paxcounter Module
    #[derive(serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    #[allow(clippy::doc_lazy_continuation)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct PaxcounterConfig {
        ///
        /// Enable the Paxcounter Module
        #[prost(bool, tag = "1")]
        pub enabled: bool,
        #[prost(uint32, tag = "2")]
        pub paxcounter_update_interval: u32,
    }
    ///
    /// Serial Config
    #[derive(serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    #[allow(clippy::doc_lazy_continuation)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct SerialConfig {
        ///
        /// Preferences for the SerialModule
        #[prost(bool, tag = "1")]
        pub enabled: bool,
        ///
        /// TODO: REPLACE
        #[prost(bool, tag = "2")]
        pub echo: bool,
        ///
        /// RX pin (should match Arduino gpio pin number)
        #[prost(uint32, tag = "3")]
        pub rxd: u32,
        ///
        /// TX pin (should match Arduino gpio pin number)
        #[prost(uint32, tag = "4")]
        pub txd: u32,
        ///
        /// Serial baud rate
        #[prost(enumeration = "serial_config::SerialBaud", tag = "5")]
        pub baud: i32,
        ///
        /// TODO: REPLACE
        #[prost(uint32, tag = "6")]
        pub timeout: u32,
        ///
        /// Mode for serial module operation
        #[prost(enumeration = "serial_config::SerialMode", tag = "7")]
        pub mode: i32,
        ///
        /// Overrides the platform's defacto Serial port instance to use with Serial module config settings
        /// This is currently only usable in output modes like NMEA / CalTopo and may behave strangely or not work at all in other modes
        /// Existing logging over the Serial Console will still be present
        #[prost(bool, tag = "8")]
        pub override_console_serial_port: bool,
    }
    /// Nested message and enum types in `SerialConfig`.
    pub mod serial_config {
        ///
        /// TODO: REPLACE
        #[derive(serde::Serialize, serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        #[allow(clippy::doc_lazy_continuation)]
        #[derive(
            Clone,
            Copy,
            Debug,
            PartialEq,
            Eq,
            Hash,
            PartialOrd,
            Ord,
            ::prost::Enumeration
        )]
        #[repr(i32)]
        pub enum SerialBaud {
            BaudDefault = 0,
            Baud110 = 1,
            Baud300 = 2,
            Baud600 = 3,
            Baud1200 = 4,
            Baud2400 = 5,
            Baud4800 = 6,
            Baud9600 = 7,
            Baud19200 = 8,
            Baud38400 = 9,
            Baud57600 = 10,
            Baud115200 = 11,
            Baud230400 = 12,
            Baud460800 = 13,
            Baud576000 = 14,
            Baud921600 = 15,
        }
        impl SerialBaud {
            /// String value of the enum field names used in the ProtoBuf definition.
            ///
            /// The values are not transformed in any way and thus are considered stable
            /// (if the ProtoBuf definition does not change) and safe for programmatic use.
            pub fn as_str_name(&self) -> &'static str {
                match self {
                    SerialBaud::BaudDefault => "BAUD_DEFAULT",
                    SerialBaud::Baud110 => "BAUD_110",
                    SerialBaud::Baud300 => "BAUD_300",
                    SerialBaud::Baud600 => "BAUD_600",
                    SerialBaud::Baud1200 => "BAUD_1200",
                    SerialBaud::Baud2400 => "BAUD_2400",
                    SerialBaud::Baud4800 => "BAUD_4800",
                    SerialBaud::Baud9600 => "BAUD_9600",
                    SerialBaud::Baud19200 => "BAUD_19200",
                    SerialBaud::Baud38400 => "BAUD_38400",
                    SerialBaud::Baud57600 => "BAUD_57600",
                    SerialBaud::Baud115200 => "BAUD_115200",
                    SerialBaud::Baud230400 => "BAUD_230400",
                    SerialBaud::Baud460800 => "BAUD_460800",
                    SerialBaud::Baud576000 => "BAUD_576000",
                    SerialBaud::Baud921600 => "BAUD_921600",
                }
            }
            /// Creates an enum from field names used in the ProtoBuf definition.
            pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
                match value {
                    "BAUD_DEFAULT" => Some(Self::BaudDefault),
                    "BAUD_110" => Some(Self::Baud110),
                    "BAUD_300" => Some(Self::Baud300),
                    "BAUD_600" => Some(Self::Baud600),
                    "BAUD_1200" => Some(Self::Baud1200),
                    "BAUD_2400" => Some(Self::Baud2400),
                    "BAUD_4800" => Some(Self::Baud4800),
                    "BAUD_9600" => Some(Self::Baud9600),
                    "BAUD_19200" => Some(Self::Baud19200),
                    "BAUD_38400" => Some(Self::Baud38400),
                    "BAUD_57600" => Some(Self::Baud57600),
                    "BAUD_115200" => Some(Self::Baud115200),
                    "BAUD_230400" => Some(Self::Baud230400),
                    "BAUD_460800" => Some(Self::Baud460800),
                    "BAUD_576000" => Some(Self::Baud576000),
                    "BAUD_921600" => Some(Self::Baud921600),
                    _ => None,
                }
            }
        }
        ///
        /// TODO: REPLACE
        #[derive(serde::Serialize, serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        #[allow(clippy::doc_lazy_continuation)]
        #[derive(
            Clone,
            Copy,
            Debug,
            PartialEq,
            Eq,
            Hash,
            PartialOrd,
            Ord,
            ::prost::Enumeration
        )]
        #[repr(i32)]
        pub enum SerialMode {
            Default = 0,
            Simple = 1,
            Proto = 2,
            Textmsg = 3,
            Nmea = 4,
            /// NMEA messages specifically tailored for CalTopo
            Caltopo = 5,
        }
        impl SerialMode {
            /// String value of the enum field names used in the ProtoBuf definition.
            ///
            /// The values are not transformed in any way and thus are considered stable
            /// (if the ProtoBuf definition does not change) and safe for programmatic use.
            pub fn as_str_name(&self) -> &'static str {
                match self {
                    SerialMode::Default => "DEFAULT",
                    SerialMode::Simple => "SIMPLE",
                    SerialMode::Proto => "PROTO",
                    SerialMode::Textmsg => "TEXTMSG",
                    SerialMode::Nmea => "NMEA",
                    SerialMode::Caltopo => "CALTOPO",
                }
            }
            /// Creates an enum from field names used in the ProtoBuf definition.
            pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
                match value {
                    "DEFAULT" => Some(Self::Default),
                    "SIMPLE" => Some(Self::Simple),
                    "PROTO" => Some(Self::Proto),
                    "TEXTMSG" => Some(Self::Textmsg),
                    "NMEA" => Some(Self::Nmea),
                    "CALTOPO" => Some(Self::Caltopo),
                    _ => None,
                }
            }
        }
    }
    ///
    /// External Notifications Config
    #[derive(serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    #[allow(clippy::doc_lazy_continuation)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct ExternalNotificationConfig {
        ///
        /// Enable the ExternalNotificationModule
        #[prost(bool, tag = "1")]
        pub enabled: bool,
        ///
        /// When using in On/Off mode, keep the output on for this many
        /// milliseconds. Default 1000ms (1 second).
        #[prost(uint32, tag = "2")]
        pub output_ms: u32,
        ///
        /// Define the output pin GPIO setting Defaults to
        /// EXT_NOTIFY_OUT if set for the board.
        /// In standalone devices this pin should drive the LED to match the UI.
        #[prost(uint32, tag = "3")]
        pub output: u32,
        ///
        /// Optional: Define a secondary output pin for a vibra motor
        /// This is used in standalone devices to match the UI.
        #[prost(uint32, tag = "8")]
        pub output_vibra: u32,
        ///
        /// Optional: Define a tertiary output pin for an active buzzer
        /// This is used in standalone devices to to match the UI.
        #[prost(uint32, tag = "9")]
        pub output_buzzer: u32,
        ///
        /// IF this is true, the 'output' Pin will be pulled active high, false
        /// means active low.
        #[prost(bool, tag = "4")]
        pub active: bool,
        ///
        /// True: Alert when a text message arrives (output)
        #[prost(bool, tag = "5")]
        pub alert_message: bool,
        ///
        /// True: Alert when a text message arrives (output_vibra)
        #[prost(bool, tag = "10")]
        pub alert_message_vibra: bool,
        ///
        /// True: Alert when a text message arrives (output_buzzer)
        #[prost(bool, tag = "11")]
        pub alert_message_buzzer: bool,
        ///
        /// True: Alert when the bell character is received (output)
        #[prost(bool, tag = "6")]
        pub alert_bell: bool,
        ///
        /// True: Alert when the bell character is received (output_vibra)
        #[prost(bool, tag = "12")]
        pub alert_bell_vibra: bool,
        ///
        /// True: Alert when the bell character is received (output_buzzer)
        #[prost(bool, tag = "13")]
        pub alert_bell_buzzer: bool,
        ///
        /// use a PWM output instead of a simple on/off output. This will ignore
        /// the 'output', 'output_ms' and 'active' settings and use the
        /// device.buzzer_gpio instead.
        #[prost(bool, tag = "7")]
        pub use_pwm: bool,
        ///
        /// The notification will toggle with 'output_ms' for this time of seconds.
        /// Default is 0 which means don't repeat at all. 60 would mean blink
        /// and/or beep for 60 seconds
        #[prost(uint32, tag = "14")]
        pub nag_timeout: u32,
        ///
        /// When true, enables devices with native I2S audio output to use the RTTTL over speaker like a buzzer
        /// T-Watch S3 and T-Deck for example have this capability
        #[prost(bool, tag = "15")]
        pub use_i2s_as_buzzer: bool,
    }
    ///
    /// Store and Forward Module Config
    #[derive(serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    #[allow(clippy::doc_lazy_continuation)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct StoreForwardConfig {
        ///
        /// Enable the Store and Forward Module
        #[prost(bool, tag = "1")]
        pub enabled: bool,
        ///
        /// TODO: REPLACE
        #[prost(bool, tag = "2")]
        pub heartbeat: bool,
        ///
        /// TODO: REPLACE
        #[prost(uint32, tag = "3")]
        pub records: u32,
        ///
        /// TODO: REPLACE
        #[prost(uint32, tag = "4")]
        pub history_return_max: u32,
        ///
        /// TODO: REPLACE
        #[prost(uint32, tag = "5")]
        pub history_return_window: u32,
    }
    ///
    /// Preferences for the RangeTestModule
    #[derive(serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    #[allow(clippy::doc_lazy_continuation)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct RangeTestConfig {
        ///
        /// Enable the Range Test Module
        #[prost(bool, tag = "1")]
        pub enabled: bool,
        ///
        /// Send out range test messages from this node
        #[prost(uint32, tag = "2")]
        pub sender: u32,
        ///
        /// Bool value indicating that this node should save a RangeTest.csv file.
        /// ESP32 Only
        #[prost(bool, tag = "3")]
        pub save: bool,
    }
    ///
    /// Configuration for both device and environment metrics
    #[derive(serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    #[allow(clippy::doc_lazy_continuation)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct TelemetryConfig {
        ///
        /// Interval in seconds of how often we should try to send our
        /// device metrics to the mesh
        #[prost(uint32, tag = "1")]
        pub device_update_interval: u32,
        #[prost(uint32, tag = "2")]
        pub environment_update_interval: u32,
        ///
        /// Preferences for the Telemetry Module (Environment)
        /// Enable/Disable the telemetry measurement module measurement collection
        #[prost(bool, tag = "3")]
        pub environment_measurement_enabled: bool,
        ///
        /// Enable/Disable the telemetry measurement module on-device display
        #[prost(bool, tag = "4")]
        pub environment_screen_enabled: bool,
        ///
        /// We'll always read the sensor in Celsius, but sometimes we might want to
        /// display the results in Fahrenheit as a "user preference".
        #[prost(bool, tag = "5")]
        pub environment_display_fahrenheit: bool,
        ///
        /// Enable/Disable the air quality metrics
        #[prost(bool, tag = "6")]
        pub air_quality_enabled: bool,
        ///
        /// Interval in seconds of how often we should try to send our
        /// air quality metrics to the mesh
        #[prost(uint32, tag = "7")]
        pub air_quality_interval: u32,
        ///
        /// Interval in seconds of how often we should try to send our
        /// air quality metrics to the mesh
        #[prost(bool, tag = "8")]
        pub power_measurement_enabled: bool,
        ///
        /// Interval in seconds of how often we should try to send our
        /// air quality metrics to the mesh
        #[prost(uint32, tag = "9")]
        pub power_update_interval: u32,
        ///
        /// Interval in seconds of how often we should try to send our
        /// air quality metrics to the mesh
        #[prost(bool, tag = "10")]
        pub power_screen_enabled: bool,
    }
    ///
    /// TODO: REPLACE
    #[derive(serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    #[allow(clippy::doc_lazy_continuation)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct CannedMessageConfig {
        ///
        /// Enable the rotary encoder #1. This is a 'dumb' encoder sending pulses on both A and B pins while rotating.
        #[prost(bool, tag = "1")]
        pub rotary1_enabled: bool,
        ///
        /// GPIO pin for rotary encoder A port.
        #[prost(uint32, tag = "2")]
        pub inputbroker_pin_a: u32,
        ///
        /// GPIO pin for rotary encoder B port.
        #[prost(uint32, tag = "3")]
        pub inputbroker_pin_b: u32,
        ///
        /// GPIO pin for rotary encoder Press port.
        #[prost(uint32, tag = "4")]
        pub inputbroker_pin_press: u32,
        ///
        /// Generate input event on CW of this kind.
        #[prost(enumeration = "canned_message_config::InputEventChar", tag = "5")]
        pub inputbroker_event_cw: i32,
        ///
        /// Generate input event on CCW of this kind.
        #[prost(enumeration = "canned_message_config::InputEventChar", tag = "6")]
        pub inputbroker_event_ccw: i32,
        ///
        /// Generate input event on Press of this kind.
        #[prost(enumeration = "canned_message_config::InputEventChar", tag = "7")]
        pub inputbroker_event_press: i32,
        ///
        /// Enable the Up/Down/Select input device. Can be RAK rotary encoder or 3 buttons. Uses the a/b/press definitions from inputbroker.
        #[prost(bool, tag = "8")]
        pub updown1_enabled: bool,
        ///
        /// Enable/disable CannedMessageModule.
        #[prost(bool, tag = "9")]
        pub enabled: bool,
        ///
        /// Input event origin accepted by the canned message module.
        /// Can be e.g. "rotEnc1", "upDownEnc1" or keyword "_any"
        #[prost(string, tag = "10")]
        pub allow_input_source: ::prost::alloc::string::String,
        ///
        /// CannedMessageModule also sends a bell character with the messages.
        /// ExternalNotificationModule can benefit from this feature.
        #[prost(bool, tag = "11")]
        pub send_bell: bool,
    }
    /// Nested message and enum types in `CannedMessageConfig`.
    pub mod canned_message_config {
        ///
        /// TODO: REPLACE
        #[derive(serde::Serialize, serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        #[allow(clippy::doc_lazy_continuation)]
        #[derive(
            Clone,
            Copy,
            Debug,
            PartialEq,
            Eq,
            Hash,
            PartialOrd,
            Ord,
            ::prost::Enumeration
        )]
        #[repr(i32)]
        pub enum InputEventChar {
            ///
            /// TODO: REPLACE
            None = 0,
            ///
            /// TODO: REPLACE
            Up = 17,
            ///
            /// TODO: REPLACE
            Down = 18,
            ///
            /// TODO: REPLACE
            Left = 19,
            ///
            /// TODO: REPLACE
            Right = 20,
            ///
            /// '\n'
            Select = 10,
            ///
            /// TODO: REPLACE
            Back = 27,
            ///
            /// TODO: REPLACE
            Cancel = 24,
        }
        impl InputEventChar {
            /// String value of the enum field names used in the ProtoBuf definition.
            ///
            /// The values are not transformed in any way and thus are considered stable
            /// (if the ProtoBuf definition does not change) and safe for programmatic use.
            pub fn as_str_name(&self) -> &'static str {
                match self {
                    InputEventChar::None => "NONE",
                    InputEventChar::Up => "UP",
                    InputEventChar::Down => "DOWN",
                    InputEventChar::Left => "LEFT",
                    InputEventChar::Right => "RIGHT",
                    InputEventChar::Select => "SELECT",
                    InputEventChar::Back => "BACK",
                    InputEventChar::Cancel => "CANCEL",
                }
            }
            /// Creates an enum from field names used in the ProtoBuf definition.
            pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
                match value {
                    "NONE" => Some(Self::None),
                    "UP" => Some(Self::Up),
                    "DOWN" => Some(Self::Down),
                    "LEFT" => Some(Self::Left),
                    "RIGHT" => Some(Self::Right),
                    "SELECT" => Some(Self::Select),
                    "BACK" => Some(Self::Back),
                    "CANCEL" => Some(Self::Cancel),
                    _ => None,
                }
            }
        }
    }
    ///
    /// Ambient Lighting Module - Settings for control of onboard LEDs to allow users to adjust the brightness levels and respective color levels.
    /// Initially created for the RAK14001 RGB LED module.
    #[derive(serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    #[allow(clippy::doc_lazy_continuation)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct AmbientLightingConfig {
        ///
        /// Sets LED to on or off.
        #[prost(bool, tag = "1")]
        pub led_state: bool,
        ///
        /// Sets the current for the LED output. Default is 10.
        #[prost(uint32, tag = "2")]
        pub current: u32,
        ///
        /// Sets the red LED level. Values are 0-255.
        #[prost(uint32, tag = "3")]
        pub red: u32,
        ///
        /// Sets the green LED level. Values are 0-255.
        #[prost(uint32, tag = "4")]
        pub green: u32,
        ///
        /// Sets the blue LED level. Values are 0-255.
        #[prost(uint32, tag = "5")]
        pub blue: u32,
    }
    ///
    /// TODO: REPLACE
    #[derive(serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    #[allow(clippy::doc_lazy_continuation)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum PayloadVariant {
        ///
        /// TODO: REPLACE
        #[prost(message, tag = "1")]
        Mqtt(MqttConfig),
        ///
        /// TODO: REPLACE
        #[prost(message, tag = "2")]
        Serial(SerialConfig),
        ///
        /// TODO: REPLACE
        #[prost(message, tag = "3")]
        ExternalNotification(ExternalNotificationConfig),
        ///
        /// TODO: REPLACE
        #[prost(message, tag = "4")]
        StoreForward(StoreForwardConfig),
        ///
        /// TODO: REPLACE
        #[prost(message, tag = "5")]
        RangeTest(RangeTestConfig),
        ///
        /// TODO: REPLACE
        #[prost(message, tag = "6")]
        Telemetry(TelemetryConfig),
        ///
        /// TODO: REPLACE
        #[prost(message, tag = "7")]
        CannedMessage(CannedMessageConfig),
        ///
        /// TODO: REPLACE
        #[prost(message, tag = "8")]
        Audio(AudioConfig),
        ///
        /// TODO: REPLACE
        #[prost(message, tag = "9")]
        RemoteHardware(RemoteHardwareConfig),
        ///
        /// TODO: REPLACE
        #[prost(message, tag = "10")]
        NeighborInfo(NeighborInfoConfig),
        ///
        /// TODO: REPLACE
        #[prost(message, tag = "11")]
        AmbientLighting(AmbientLightingConfig),
        ///
        /// TODO: REPLACE
        #[prost(message, tag = "12")]
        DetectionSensor(DetectionSensorConfig),
        ///
        /// TODO: REPLACE
        #[prost(message, tag = "13")]
        Paxcounter(PaxcounterConfig),
    }
}
///
/// A GPIO pin definition for remote hardware module
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::doc_lazy_continuation)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RemoteHardwarePin {
    ///
    /// GPIO Pin number (must match Arduino)
    #[prost(uint32, tag = "1")]
    pub gpio_pin: u32,
    ///
    /// Name for the GPIO pin (i.e. Front gate, mailbox, etc)
    #[prost(string, tag = "2")]
    pub name: ::prost::alloc::string::String,
    ///
    /// Type of GPIO access available to consumers on the mesh
    #[prost(enumeration = "RemoteHardwarePinType", tag = "3")]
    pub r#type: i32,
}
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::doc_lazy_continuation)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum RemoteHardwarePinType {
    ///
    /// Unset/unused
    Unknown = 0,
    ///
    /// GPIO pin can be read (if it is high / low)
    DigitalRead = 1,
    ///
    /// GPIO pin can be written to (high / low)
    DigitalWrite = 2,
}
impl RemoteHardwarePinType {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            RemoteHardwarePinType::Unknown => "UNKNOWN",
            RemoteHardwarePinType::DigitalRead => "DIGITAL_READ",
            RemoteHardwarePinType::DigitalWrite => "DIGITAL_WRITE",
        }
    }
    /// Creates an enum from field names used in the ProtoBuf definition.
    pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
        match value {
            "UNKNOWN" => Some(Self::Unknown),
            "DIGITAL_READ" => Some(Self::DigitalRead),
            "DIGITAL_WRITE" => Some(Self::DigitalWrite),
            _ => None,
        }
    }
}
///
/// For any new 'apps' that run on the device or via sister apps on phones/PCs they should pick and use a
/// unique 'portnum' for their application.
/// If you are making a new app using meshtastic, please send in a pull request to add your 'portnum' to this
/// master table.
/// PortNums should be assigned in the following range:
/// 0-63   Core Meshtastic use, do not use for third party apps
/// 64-127 Registered 3rd party apps, send in a pull request that adds a new entry to portnums.proto to  register your application
/// 256-511 Use one of these portnums for your private applications that you don't want to register publically
/// All other values are reserved.
/// Note: This was formerly a Type enum named 'typ' with the same id #
/// We have change to this 'portnum' based scheme for specifying app handlers for particular payloads.
/// This change is backwards compatible by treating the legacy OPAQUE/CLEAR_TEXT values identically.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::doc_lazy_continuation)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum PortNum {
    ///
    /// Deprecated: do not use in new code (formerly called OPAQUE)
    /// A message sent from a device outside of the mesh, in a form the mesh does not understand
    /// NOTE: This must be 0, because it is documented in IMeshService.aidl to be so
    /// ENCODING: binary undefined
    UnknownApp = 0,
    ///
    /// A simple UTF-8 text message, which even the little micros in the mesh
    /// can understand and show on their screen eventually in some circumstances
    /// even signal might send messages in this form (see below)
    /// ENCODING: UTF-8 Plaintext (?)
    TextMessageApp = 1,
    ///
    /// Reserved for built-in GPIO/example app.
    /// See remote_hardware.proto/HardwareMessage for details on the message sent/received to this port number
    /// ENCODING: Protobuf
    RemoteHardwareApp = 2,
    ///
    /// The built-in position messaging app.
    /// Payload is a Position message.
    /// ENCODING: Protobuf
    PositionApp = 3,
    ///
    /// The built-in user info app.
    /// Payload is a User message.
    /// ENCODING: Protobuf
    NodeinfoApp = 4,
    ///
    /// Protocol control packets for mesh protocol use.
    /// Payload is a Routing message.
    /// ENCODING: Protobuf
    RoutingApp = 5,
    ///
    /// Admin control packets.
    /// Payload is a AdminMessage message.
    /// ENCODING: Protobuf
    AdminApp = 6,
    ///
    /// Compressed TEXT_MESSAGE payloads.
    /// ENCODING: UTF-8 Plaintext (?) with Unishox2 Compression
    /// NOTE: The Device Firmware converts a TEXT_MESSAGE_APP to TEXT_MESSAGE_COMPRESSED_APP if the compressed
    /// payload is shorter. There's no need for app developers to do this themselves. Also the firmware will decompress
    /// any incoming TEXT_MESSAGE_COMPRESSED_APP payload and convert to TEXT_MESSAGE_APP.
    TextMessageCompressedApp = 7,
    ///
    /// Waypoint payloads.
    /// Payload is a Waypoint message.
    /// ENCODING: Protobuf
    WaypointApp = 8,
    ///
    /// Audio Payloads.
    /// Encapsulated codec2 packets. On 2.4 GHZ Bandwidths only for now
    /// ENCODING: codec2 audio frames
    /// NOTE: audio frames contain a 3 byte header (0xc0 0xde 0xc2) and a one byte marker for the decompressed bitrate.
    /// This marker comes from the 'moduleConfig.audio.bitrate' enum minus one.
    AudioApp = 9,
    ///
    /// Same as Text Message but originating from Detection Sensor Module.
    /// NOTE: This portnum traffic is not sent to the public MQTT starting at firmware version 2.2.9
    DetectionSensorApp = 10,
    ///
    /// Provides a 'ping' service that replies to any packet it receives.
    /// Also serves as a small example module.
    /// ENCODING: ASCII Plaintext
    ReplyApp = 32,
    ///
    /// Used for the python IP tunnel feature
    /// ENCODING: IP Packet. Handled by the python API, firmware ignores this one and pases on.
    IpTunnelApp = 33,
    ///
    /// Paxcounter lib included in the firmware
    /// ENCODING: protobuf
    PaxcounterApp = 34,
    ///
    /// Provides a hardware serial interface to send and receive from the Meshtastic network.
    /// Connect to the RX/TX pins of a device with 38400 8N1. Packets received from the Meshtastic
    /// network is forwarded to the RX pin while sending a packet to TX will go out to the Mesh network.
    /// Maximum packet size of 240 bytes.
    /// Module is disabled by default can be turned on by setting SERIAL_MODULE_ENABLED = 1 in SerialPlugh.cpp.
    /// ENCODING: binary undefined
    SerialApp = 64,
    ///
    /// STORE_FORWARD_APP (Work in Progress)
    /// Maintained by Jm Casler (MC Hamster) : jm@casler.org
    /// ENCODING: Protobuf
    StoreForwardApp = 65,
    ///
    /// Optional port for messages for the range test module.
    /// ENCODING: ASCII Plaintext
    /// NOTE: This portnum traffic is not sent to the public MQTT starting at firmware version 2.2.9
    RangeTestApp = 66,
    ///
    /// Provides a format to send and receive telemetry data from the Meshtastic network.
    /// Maintained by Charles Crossan (crossan007) : crossan007@gmail.com
    /// ENCODING: Protobuf
    TelemetryApp = 67,
    ///
    /// Experimental tools for estimating node position without a GPS
    /// Maintained by Github user a-f-G-U-C (a Meshtastic contributor)
    /// Project files at <https://github.com/a-f-G-U-C/Meshtastic-ZPS>
    /// ENCODING: arrays of int64 fields
    ZpsApp = 68,
    ///
    /// Used to let multiple instances of Linux native applications communicate
    /// as if they did using their LoRa chip.
    /// Maintained by GitHub user GUVWAF.
    /// Project files at <https://github.com/GUVWAF/Meshtasticator>
    /// ENCODING: Protobuf (?)
    SimulatorApp = 69,
    ///
    /// Provides a traceroute functionality to show the route a packet towards
    /// a certain destination would take on the mesh.
    /// ENCODING: Protobuf
    TracerouteApp = 70,
    ///
    /// Aggregates edge info for the network by sending out a list of each node's neighbors
    /// ENCODING: Protobuf
    NeighborinfoApp = 71,
    ///
    /// ATAK Plugin
    /// Portnum for payloads from the official Meshtastic ATAK plugin
    AtakPlugin = 72,
    ///
    /// Provides unencrypted information about a node for consumption by a map via MQTT
    MapReportApp = 73,
    ///
    /// Private applications should use portnums >= 256.
    /// To simplify initial development and testing you can use "PRIVATE_APP"
    /// in your code without needing to rebuild protobuf files (via \[regen-protos.sh\](<https://github.com/meshtastic/firmware/blob/master/bin/regen-protos.sh>))
    PrivateApp = 256,
    ///
    /// ATAK Forwarder Module <https://github.com/paulmandal/atak-forwarder>
    /// ENCODING: libcotshrink
    AtakForwarder = 257,
    ///
    /// Currently we limit port nums to no higher than this value
    Max = 511,
}
impl PortNum {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            PortNum::UnknownApp => "UNKNOWN_APP",
            PortNum::TextMessageApp => "TEXT_MESSAGE_APP",
            PortNum::RemoteHardwareApp => "REMOTE_HARDWARE_APP",
            PortNum::PositionApp => "POSITION_APP",
            PortNum::NodeinfoApp => "NODEINFO_APP",
            PortNum::RoutingApp => "ROUTING_APP",
            PortNum::AdminApp => "ADMIN_APP",
            PortNum::TextMessageCompressedApp => "TEXT_MESSAGE_COMPRESSED_APP",
            PortNum::WaypointApp => "WAYPOINT_APP",
            PortNum::AudioApp => "AUDIO_APP",
            PortNum::DetectionSensorApp => "DETECTION_SENSOR_APP",
            PortNum::ReplyApp => "REPLY_APP",
            PortNum::IpTunnelApp => "IP_TUNNEL_APP",
            PortNum::PaxcounterApp => "PAXCOUNTER_APP",
            PortNum::SerialApp => "SERIAL_APP",
            PortNum::StoreForwardApp => "STORE_FORWARD_APP",
            PortNum::RangeTestApp => "RANGE_TEST_APP",
            PortNum::TelemetryApp => "TELEMETRY_APP",
            PortNum::ZpsApp => "ZPS_APP",
            PortNum::SimulatorApp => "SIMULATOR_APP",
            PortNum::TracerouteApp => "TRACEROUTE_APP",
            PortNum::NeighborinfoApp => "NEIGHBORINFO_APP",
            PortNum::AtakPlugin => "ATAK_PLUGIN",
            PortNum::MapReportApp => "MAP_REPORT_APP",
            PortNum::PrivateApp => "PRIVATE_APP",
            PortNum::AtakForwarder => "ATAK_FORWARDER",
            PortNum::Max => "MAX",
        }
    }
    /// Creates an enum from field names used in the ProtoBuf definition.
    pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
        match value {
            "UNKNOWN_APP" => Some(Self::UnknownApp),
            "TEXT_MESSAGE_APP" => Some(Self::TextMessageApp),
            "REMOTE_HARDWARE_APP" => Some(Self::RemoteHardwareApp),
            "POSITION_APP" => Some(Self::PositionApp),
            "NODEINFO_APP" => Some(Self::NodeinfoApp),
            "ROUTING_APP" => Some(Self::RoutingApp),
            "ADMIN_APP" => Some(Self::AdminApp),
            "TEXT_MESSAGE_COMPRESSED_APP" => Some(Self::TextMessageCompressedApp),
            "WAYPOINT_APP" => Some(Self::WaypointApp),
            "AUDIO_APP" => Some(Self::AudioApp),
            "DETECTION_SENSOR_APP" => Some(Self::DetectionSensorApp),
            "REPLY_APP" => Some(Self::ReplyApp),
            "IP_TUNNEL_APP" => Some(Self::IpTunnelApp),
            "PAXCOUNTER_APP" => Some(Self::PaxcounterApp),
            "SERIAL_APP" => Some(Self::SerialApp),
            "STORE_FORWARD_APP" => Some(Self::StoreForwardApp),
            "RANGE_TEST_APP" => Some(Self::RangeTestApp),
            "TELEMETRY_APP" => Some(Self::TelemetryApp),
            "ZPS_APP" => Some(Self::ZpsApp),
            "SIMULATOR_APP" => Some(Self::SimulatorApp),
            "TRACEROUTE_APP" => Some(Self::TracerouteApp),
            "NEIGHBORINFO_APP" => Some(Self::NeighborinfoApp),
            "ATAK_PLUGIN" => Some(Self::AtakPlugin),
            "MAP_REPORT_APP" => Some(Self::MapReportApp),
            "PRIVATE_APP" => Some(Self::PrivateApp),
            "ATAK_FORWARDER" => Some(Self::AtakForwarder),
            "MAX" => Some(Self::Max),
            _ => None,
        }
    }
}
///
/// Key native device metrics such as battery level
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::doc_lazy_continuation)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DeviceMetrics {
    ///
    /// 0-100 (>100 means powered)
    #[prost(uint32, tag = "1")]
    pub battery_level: u32,
    ///
    /// Voltage measured
    #[prost(float, tag = "2")]
    pub voltage: f32,
    ///
    /// Utilization for the current channel, including well formed TX, RX and malformed RX (aka noise).
    #[prost(float, tag = "3")]
    pub channel_utilization: f32,
    ///
    /// Percent of airtime for transmission used within the last hour.
    #[prost(float, tag = "4")]
    pub air_util_tx: f32,
    ///
    /// How long the device has been running since the last reboot (in seconds)
    #[prost(uint32, tag = "5")]
    pub uptime_seconds: u32,
}
///
/// Weather station or other environmental metrics
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::doc_lazy_continuation)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct EnvironmentMetrics {
    ///
    /// Temperature measured
    #[prost(float, tag = "1")]
    pub temperature: f32,
    ///
    /// Relative humidity percent measured
    #[prost(float, tag = "2")]
    pub relative_humidity: f32,
    ///
    /// Barometric pressure in hPA measured
    #[prost(float, tag = "3")]
    pub barometric_pressure: f32,
    ///
    /// Gas resistance in MOhm measured
    #[prost(float, tag = "4")]
    pub gas_resistance: f32,
    ///
    /// Voltage measured (To be depreciated in favor of PowerMetrics in Meshtastic 3.x)
    #[prost(float, tag = "5")]
    pub voltage: f32,
    ///
    /// Current measured (To be depreciated in favor of PowerMetrics in Meshtastic 3.x)
    #[prost(float, tag = "6")]
    pub current: f32,
    ///
    /// relative scale IAQ value as measured by Bosch BME680 . value 0-500.
    /// Belongs to Air Quality but is not particle but VOC measurement. Other VOC values can also be put in here.
    #[prost(uint32, tag = "7")]
    pub iaq: u32,
}
///
/// Power Metrics (voltage / current / etc)
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::doc_lazy_continuation)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PowerMetrics {
    ///
    /// Voltage (Ch1)
    #[prost(float, tag = "1")]
    pub ch1_voltage: f32,
    ///
    /// Current (Ch1)
    #[prost(float, tag = "2")]
    pub ch1_current: f32,
    ///
    /// Voltage (Ch2)
    #[prost(float, tag = "3")]
    pub ch2_voltage: f32,
    ///
    /// Current (Ch2)
    #[prost(float, tag = "4")]
    pub ch2_current: f32,
    ///
    /// Voltage (Ch3)
    #[prost(float, tag = "5")]
    pub ch3_voltage: f32,
    ///
    /// Current (Ch3)
    #[prost(float, tag = "6")]
    pub ch3_current: f32,
}
///
/// Air quality metrics
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::doc_lazy_continuation)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct AirQualityMetrics {
    ///
    /// Concentration Units Standard PM1.0
    #[prost(uint32, tag = "1")]
    pub pm10_standard: u32,
    ///
    /// Concentration Units Standard PM2.5
    #[prost(uint32, tag = "2")]
    pub pm25_standard: u32,
    ///
    /// Concentration Units Standard PM10.0
    #[prost(uint32, tag = "3")]
    pub pm100_standard: u32,
    ///
    /// Concentration Units Environmental PM1.0
    #[prost(uint32, tag = "4")]
    pub pm10_environmental: u32,
    ///
    /// Concentration Units Environmental PM2.5
    #[prost(uint32, tag = "5")]
    pub pm25_environmental: u32,
    ///
    /// Concentration Units Environmental PM10.0
    #[prost(uint32, tag = "6")]
    pub pm100_environmental: u32,
    ///
    /// 0.3um Particle Count
    #[prost(uint32, tag = "7")]
    pub particles_03um: u32,
    ///
    /// 0.5um Particle Count
    #[prost(uint32, tag = "8")]
    pub particles_05um: u32,
    ///
    /// 1.0um Particle Count
    #[prost(uint32, tag = "9")]
    pub particles_10um: u32,
    ///
    /// 2.5um Particle Count
    #[prost(uint32, tag = "10")]
    pub particles_25um: u32,
    ///
    /// 5.0um Particle Count
    #[prost(uint32, tag = "11")]
    pub particles_50um: u32,
    ///
    /// 10.0um Particle Count
    #[prost(uint32, tag = "12")]
    pub particles_100um: u32,
}
///
/// Types of Measurements the telemetry module is equipped to handle
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::doc_lazy_continuation)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Telemetry {
    ///
    /// Seconds since 1970 - or 0 for unknown/unset
    #[prost(fixed32, tag = "1")]
    pub time: u32,
    #[prost(oneof = "telemetry::Variant", tags = "2, 3, 4, 5")]
    pub variant: ::core::option::Option<telemetry::Variant>,
}
/// Nested message and enum types in `Telemetry`.
pub mod telemetry {
    #[derive(serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    #[allow(clippy::doc_lazy_continuation)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Variant {
        ///
        /// Key native device metrics such as battery level
        #[prost(message, tag = "2")]
        DeviceMetrics(super::DeviceMetrics),
        ///
        /// Weather station or other environmental metrics
        #[prost(message, tag = "3")]
        EnvironmentMetrics(super::EnvironmentMetrics),
        ///
        /// Air quality metrics
        #[prost(message, tag = "4")]
        AirQualityMetrics(super::AirQualityMetrics),
        ///
        /// Power Metrics
        #[prost(message, tag = "5")]
        PowerMetrics(super::PowerMetrics),
    }
}
///
/// Supported I2C Sensors for telemetry in Meshtastic
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::doc_lazy_continuation)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum TelemetrySensorType {
    ///
    /// No external telemetry sensor explicitly set
    SensorUnset = 0,
    ///
    /// High accuracy temperature, pressure, humidity
    Bme280 = 1,
    ///
    /// High accuracy temperature, pressure, humidity, and air resistance
    Bme680 = 2,
    ///
    /// Very high accuracy temperature
    Mcp9808 = 3,
    ///
    /// Moderate accuracy current and voltage
    Ina260 = 4,
    ///
    /// Moderate accuracy current and voltage
    Ina219 = 5,
    ///
    /// High accuracy temperature and pressure
    Bmp280 = 6,
    ///
    /// High accuracy temperature and humidity
    Shtc3 = 7,
    ///
    /// High accuracy pressure
    Lps22 = 8,
    ///
    /// 3-Axis magnetic sensor
    Qmc6310 = 9,
    ///
    /// 6-Axis inertial measurement sensor
    Qmi8658 = 10,
    ///
    /// 3-Axis magnetic sensor
    Qmc5883l = 11,
    ///
    /// High accuracy temperature and humidity
    Sht31 = 12,
    ///
    /// PM2.5 air quality sensor
    Pmsa003i = 13,
    ///
    /// INA3221 3 Channel Voltage / Current Sensor
    Ina3221 = 14,
    ///
    /// BMP085/BMP180 High accuracy temperature and pressure (older Version of BMP280)
    Bmp085 = 15,
}
impl TelemetrySensorType {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            TelemetrySensorType::SensorUnset => "SENSOR_UNSET",
            TelemetrySensorType::Bme280 => "BME280",
            TelemetrySensorType::Bme680 => "BME680",
            TelemetrySensorType::Mcp9808 => "MCP9808",
            TelemetrySensorType::Ina260 => "INA260",
            TelemetrySensorType::Ina219 => "INA219",
            TelemetrySensorType::Bmp280 => "BMP280",
            TelemetrySensorType::Shtc3 => "SHTC3",
            TelemetrySensorType::Lps22 => "LPS22",
            TelemetrySensorType::Qmc6310 => "QMC6310",
            TelemetrySensorType::Qmi8658 => "QMI8658",
            TelemetrySensorType::Qmc5883l => "QMC5883L",
            TelemetrySensorType::Sht31 => "SHT31",
            TelemetrySensorType::Pmsa003i => "PMSA003I",
            TelemetrySensorType::Ina3221 => "INA3221",
            TelemetrySensorType::Bmp085 => "BMP085",
        }
    }
    /// Creates an enum from field names used in the ProtoBuf definition.
    pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
        match value {
            "SENSOR_UNSET" => Some(Self::SensorUnset),
            "BME280" => Some(Self::Bme280),
            "BME680" => Some(Self::Bme680),
            "MCP9808" => Some(Self::Mcp9808),
            "INA260" => Some(Self::Ina260),
            "INA219" => Some(Self::Ina219),
            "BMP280" => Some(Self::Bmp280),
            "SHTC3" => Some(Self::Shtc3),
            "LPS22" => Some(Self::Lps22),
            "QMC6310" => Some(Self::Qmc6310),
            "QMI8658" => Some(Self::Qmi8658),
            "QMC5883L" => Some(Self::Qmc5883l),
            "SHT31" => Some(Self::Sht31),
            "PMSA003I" => Some(Self::Pmsa003i),
            "INA3221" => Some(Self::Ina3221),
            "BMP085" => Some(Self::Bmp085),
            _ => None,
        }
    }
}
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::doc_lazy_continuation)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct XModem {
    #[prost(enumeration = "x_modem::Control", tag = "1")]
    pub control: i32,
    #[prost(uint32, tag = "2")]
    pub seq: u32,
    #[prost(uint32, tag = "3")]
    pub crc16: u32,
    #[prost(bytes = "vec", tag = "4")]
    pub buffer: ::prost::alloc::vec::Vec<u8>,
}
/// Nested message and enum types in `XModem`.
pub mod x_modem {
    #[derive(serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    #[allow(clippy::doc_lazy_continuation)]
    #[derive(
        Clone,
        Copy,
        Debug,
        PartialEq,
        Eq,
        Hash,
        PartialOrd,
        Ord,
        ::prost::Enumeration
    )]
    #[repr(i32)]
    pub enum Control {
        Nul = 0,
        Soh = 1,
        Stx = 2,
        Eot = 4,
        Ack = 6,
        Nak = 21,
        Can = 24,
        Ctrlz = 26,
    }
    impl Control {
        /// String value of the enum field names used in the ProtoBuf definition.
        ///
        /// The values are not transformed in any way and thus are considered stable
        /// (if the ProtoBuf definition does not change) and safe for programmatic use.
        pub fn as_str_name(&self) -> &'static str {
            match self {
                Control::Nul => "NUL",
                Control::Soh => "SOH",
                Control::Stx => "STX",
                Control::Eot => "EOT",
                Control::Ack => "ACK",
                Control::Nak => "NAK",
                Control::Can => "CAN",
                Control::Ctrlz => "CTRLZ",
            }
        }
        /// Creates an enum from field names used in the ProtoBuf definition.
        pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
            match value {
                "NUL" => Some(Self::Nul),
                "SOH" => Some(Self::Soh),
                "STX" => Some(Self::Stx),
                "EOT" => Some(Self::Eot),
                "ACK" => Some(Self::Ack),
                "NAK" => Some(Self::Nak),
                "CAN" => Some(Self::Can),
                "CTRLZ" => Some(Self::Ctrlz),
                _ => None,
            }
        }
    }
}
///
/// a gps position
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::doc_lazy_continuation)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Position {
    ///
    /// The new preferred location encoding, multiply by 1e-7 to get degrees
    /// in floating point
    #[prost(sfixed32, tag = "1")]
    pub latitude_i: i32,
    ///
    /// TODO: REPLACE
    #[prost(sfixed32, tag = "2")]
    pub longitude_i: i32,
    ///
    /// In meters above MSL (but see issue #359)
    #[prost(int32, tag = "3")]
    pub altitude: i32,
    ///
    /// This is usually not sent over the mesh (to save space), but it is sent
    /// from the phone so that the local device can set its time if it is sent over
    /// the mesh (because there are devices on the mesh without GPS or RTC).
    /// seconds since 1970
    #[prost(fixed32, tag = "4")]
    pub time: u32,
    ///
    /// TODO: REPLACE
    #[prost(enumeration = "position::LocSource", tag = "5")]
    pub location_source: i32,
    ///
    /// TODO: REPLACE
    #[prost(enumeration = "position::AltSource", tag = "6")]
    pub altitude_source: i32,
    ///
    /// Positional timestamp (actual timestamp of GPS solution) in integer epoch seconds
    #[prost(fixed32, tag = "7")]
    pub timestamp: u32,
    ///
    /// Pos. timestamp milliseconds adjustment (rarely available or required)
    #[prost(int32, tag = "8")]
    pub timestamp_millis_adjust: i32,
    ///
    /// HAE altitude in meters - can be used instead of MSL altitude
    #[prost(sint32, tag = "9")]
    pub altitude_hae: i32,
    ///
    /// Geoidal separation in meters
    #[prost(sint32, tag = "10")]
    pub altitude_geoidal_separation: i32,
    ///
    /// Horizontal, Vertical and Position Dilution of Precision, in 1/100 units
    /// - PDOP is sufficient for most cases
    /// - for higher precision scenarios, HDOP and VDOP can be used instead,
    ///    in which case PDOP becomes redundant (PDOP=sqrt(HDOP^2 + VDOP^2))
    /// TODO: REMOVE/INTEGRATE
    #[prost(uint32, tag = "11")]
    pub pdop: u32,
    ///
    /// TODO: REPLACE
    #[prost(uint32, tag = "12")]
    pub hdop: u32,
    ///
    /// TODO: REPLACE
    #[prost(uint32, tag = "13")]
    pub vdop: u32,
    ///
    /// GPS accuracy (a hardware specific constant) in mm
    ///    multiplied with DOP to calculate positional accuracy
    /// Default: "'bout three meters-ish" :)
    #[prost(uint32, tag = "14")]
    pub gps_accuracy: u32,
    ///
    /// Ground speed in m/s and True North TRACK in 1/100 degrees
    /// Clarification of terms:
    /// - "track" is the direction of motion (measured in horizontal plane)
    /// - "heading" is where the fuselage points (measured in horizontal plane)
    /// - "yaw" indicates a relative rotation about the vertical axis
    /// TODO: REMOVE/INTEGRATE
    #[prost(uint32, tag = "15")]
    pub ground_speed: u32,
    ///
    /// TODO: REPLACE
    #[prost(uint32, tag = "16")]
    pub ground_track: u32,
    ///
    /// GPS fix quality (from NMEA GxGGA statement or similar)
    #[prost(uint32, tag = "17")]
    pub fix_quality: u32,
    ///
    /// GPS fix type 2D/3D (from NMEA GxGSA statement)
    #[prost(uint32, tag = "18")]
    pub fix_type: u32,
    ///
    /// GPS "Satellites in View" number
    #[prost(uint32, tag = "19")]
    pub sats_in_view: u32,
    ///
    /// Sensor ID - in case multiple positioning sensors are being used
    #[prost(uint32, tag = "20")]
    pub sensor_id: u32,
    ///
    /// Estimated/expected time (in seconds) until next update:
    /// - if we update at fixed intervals of X seconds, use X
    /// - if we update at dynamic intervals (based on relative movement etc),
    ///    but "AT LEAST every Y seconds", use Y
    #[prost(uint32, tag = "21")]
    pub next_update: u32,
    ///
    /// A sequence number, incremented with each Position message to help
    ///    detect lost updates if needed
    #[prost(uint32, tag = "22")]
    pub seq_number: u32,
    ///
    /// Indicates the bits of precision set by the sending node
    #[prost(uint32, tag = "23")]
    pub precision_bits: u32,
}
/// Nested message and enum types in `Position`.
pub mod position {
    ///
    /// How the location was acquired: manual, onboard GPS, external (EUD) GPS
    #[derive(serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    #[allow(clippy::doc_lazy_continuation)]
    #[derive(
        Clone,
        Copy,
        Debug,
        PartialEq,
        Eq,
        Hash,
        PartialOrd,
        Ord,
        ::prost::Enumeration
    )]
    #[repr(i32)]
    pub enum LocSource {
        ///
        /// TODO: REPLACE
        LocUnset = 0,
        ///
        /// TODO: REPLACE
        LocManual = 1,
        ///
        /// TODO: REPLACE
        LocInternal = 2,
        ///
        /// TODO: REPLACE
        LocExternal = 3,
    }
    impl LocSource {
        /// String value of the enum field names used in the ProtoBuf definition.
        ///
        /// The values are not transformed in any way and thus are considered stable
        /// (if the ProtoBuf definition does not change) and safe for programmatic use.
        pub fn as_str_name(&self) -> &'static str {
            match self {
                LocSource::LocUnset => "LOC_UNSET",
                LocSource::LocManual => "LOC_MANUAL",
                LocSource::LocInternal => "LOC_INTERNAL",
                LocSource::LocExternal => "LOC_EXTERNAL",
            }
        }
        /// Creates an enum from field names used in the ProtoBuf definition.
        pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
            match value {
                "LOC_UNSET" => Some(Self::LocUnset),
                "LOC_MANUAL" => Some(Self::LocManual),
                "LOC_INTERNAL" => Some(Self::LocInternal),
                "LOC_EXTERNAL" => Some(Self::LocExternal),
                _ => None,
            }
        }
    }
    ///
    /// How the altitude was acquired: manual, GPS int/ext, etc
    /// Default: same as location_source if present
    #[derive(serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    #[allow(clippy::doc_lazy_continuation)]
    #[derive(
        Clone,
        Copy,
        Debug,
        PartialEq,
        Eq,
        Hash,
        PartialOrd,
        Ord,
        ::prost::Enumeration
    )]
    #[repr(i32)]
    pub enum AltSource {
        ///
        /// TODO: REPLACE
        AltUnset = 0,
        ///
        /// TODO: REPLACE
        AltManual = 1,
        ///
        /// TODO: REPLACE
        AltInternal = 2,
        ///
        /// TODO: REPLACE
        AltExternal = 3,
        ///
        /// TODO: REPLACE
        AltBarometric = 4,
    }
    impl AltSource {
        /// String value of the enum field names used in the ProtoBuf definition.
        ///
        /// The values are not transformed in any way and thus are considered stable
        /// (if the ProtoBuf definition does not change) and safe for programmatic use.
        pub fn as_str_name(&self) -> &'static str {
            match self {
                AltSource::AltUnset => "ALT_UNSET",
                AltSource::AltManual => "ALT_MANUAL",
                AltSource::AltInternal => "ALT_INTERNAL",
                AltSource::AltExternal => "ALT_EXTERNAL",
                AltSource::AltBarometric => "ALT_BAROMETRIC",
            }
        }
        /// Creates an enum from field names used in the ProtoBuf definition.
        pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
            match value {
                "ALT_UNSET" => Some(Self::AltUnset),
                "ALT_MANUAL" => Some(Self::AltManual),
                "ALT_INTERNAL" => Some(Self::AltInternal),
                "ALT_EXTERNAL" => Some(Self::AltExternal),
                "ALT_BAROMETRIC" => Some(Self::AltBarometric),
                _ => None,
            }
        }
    }
}
///
/// Broadcast when a newly powered mesh node wants to find a node num it can use
/// Sent from the phone over bluetooth to set the user id for the owner of this node.
/// Also sent from nodes to each other when a new node signs on (so all clients can have this info)
/// The algorithm is as follows:
/// when a node starts up, it broadcasts their user and the normal flow is for all
/// other nodes to reply with their User as well (so the new node can build its nodedb)
/// If a node ever receives a User (not just the first broadcast) message where
/// the sender node number equals our node number, that indicates a collision has
/// occurred and the following steps should happen:
/// If the receiving node (that was already in the mesh)'s macaddr is LOWER than the
/// new User who just tried to sign in: it gets to keep its nodenum.
/// We send a broadcast message of OUR User (we use a broadcast so that the other node can
/// receive our message, considering we have the same id - it also serves to let
/// observers correct their nodedb) - this case is rare so it should be okay.
/// If any node receives a User where the macaddr is GTE than their local macaddr,
/// they have been vetoed and should pick a new random nodenum (filtering against
/// whatever it knows about the nodedb) and rebroadcast their User.
/// A few nodenums are reserved and will never be requested:
/// 0xff - broadcast
/// 0 through 3 - for future use
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::doc_lazy_continuation)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct User {
    ///
    /// A globally unique ID string for this user.
    /// In the case of Signal that would mean +16504442323, for the default macaddr derived id it would be !<8 hexidecimal bytes>.
    /// Note: app developers are encouraged to also use the following standard
    /// node IDs "^all" (for broadcast), "^local" (for the locally connected node)
    #[prost(string, tag = "1")]
    pub id: ::prost::alloc::string::String,
    ///
    /// A full name for this user, i.e. "Kevin Hester"
    #[prost(string, tag = "2")]
    pub long_name: ::prost::alloc::string::String,
    ///
    /// A VERY short name, ideally two characters.
    /// Suitable for a tiny OLED screen
    #[prost(string, tag = "3")]
    pub short_name: ::prost::alloc::string::String,
    ///
    /// Deprecated in Meshtastic 2.1.x
    /// This is the addr of the radio.
    /// Not populated by the phone, but added by the esp32 when broadcasting
    #[deprecated]
    #[prost(bytes = "vec", tag = "4")]
    pub macaddr: ::prost::alloc::vec::Vec<u8>,
    ///
    /// TBEAM, HELTEC, etc...
    /// Starting in 1.2.11 moved to hw_model enum in the NodeInfo object.
    /// Apps will still need the string here for older builds
    /// (so OTA update can find the right image), but if the enum is available it will be used instead.
    #[prost(enumeration = "HardwareModel", tag = "5")]
    pub hw_model: i32,
    ///
    /// In some regions Ham radio operators have different bandwidth limitations than others.
    /// If this user is a licensed operator, set this flag.
    /// Also, "long_name" should be their licence number.
    #[prost(bool, tag = "6")]
    pub is_licensed: bool,
    ///
    /// Indicates that the user's role in the mesh
    #[prost(enumeration = "config::device_config::Role", tag = "7")]
    pub role: i32,
}
///
/// A message used in our Dynamic Source Routing protocol (RFC 4728 based)
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::doc_lazy_continuation)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RouteDiscovery {
    ///
    /// The list of nodenums this packet has visited so far
    #[prost(fixed32, repeated, tag = "1")]
    pub route: ::prost::alloc::vec::Vec<u32>,
}
///
/// A Routing control Data packet handled by the routing module
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::doc_lazy_continuation)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Routing {
    #[prost(oneof = "routing::Variant", tags = "1, 2, 3")]
    pub variant: ::core::option::Option<routing::Variant>,
}
/// Nested message and enum types in `Routing`.
pub mod routing {
    ///
    /// A failure in delivering a message (usually used for routing control messages, but might be provided in addition to ack.fail_id to provide
    /// details on the type of failure).
    #[derive(serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    #[allow(clippy::doc_lazy_continuation)]
    #[derive(
        Clone,
        Copy,
        Debug,
        PartialEq,
        Eq,
        Hash,
        PartialOrd,
        Ord,
        ::prost::Enumeration
    )]
    #[repr(i32)]
    pub enum Error {
        ///
        /// This message is not a failure
        None = 0,
        ///
        /// Our node doesn't have a route to the requested destination anymore.
        NoRoute = 1,
        ///
        /// We received a nak while trying to forward on your behalf
        GotNak = 2,
        ///
        /// TODO: REPLACE
        Timeout = 3,
        ///
        /// No suitable interface could be found for delivering this packet
        NoInterface = 4,
        ///
        /// We reached the max retransmission count (typically for naive flood routing)
        MaxRetransmit = 5,
        ///
        /// No suitable channel was found for sending this packet (i.e. was requested channel index disabled?)
        NoChannel = 6,
        ///
        /// The packet was too big for sending (exceeds interface MTU after encoding)
        TooLarge = 7,
        ///
        /// The request had want_response set, the request reached the destination node, but no service on that node wants to send a response
        /// (possibly due to bad channel permissions)
        NoResponse = 8,
        ///
        /// Cannot send currently because duty cycle regulations will be violated.
        DutyCycleLimit = 9,
        ///
        /// The application layer service on the remote node received your request, but considered your request somehow invalid
        BadRequest = 32,
        ///
        /// The application layer service on the remote node received your request, but considered your request not authorized
        /// (i.e you did not send the request on the required bound channel)
        NotAuthorized = 33,
    }
    impl Error {
        /// String value of the enum field names used in the ProtoBuf definition.
        ///
        /// The values are not transformed in any way and thus are considered stable
        /// (if the ProtoBuf definition does not change) and safe for programmatic use.
        pub fn as_str_name(&self) -> &'static str {
            match self {
                Error::None => "NONE",
                Error::NoRoute => "NO_ROUTE",
                Error::GotNak => "GOT_NAK",
                Error::Timeout => "TIMEOUT",
                Error::NoInterface => "NO_INTERFACE",
                Error::MaxRetransmit => "MAX_RETRANSMIT",
                Error::NoChannel => "NO_CHANNEL",
                Error::TooLarge => "TOO_LARGE",
                Error::NoResponse => "NO_RESPONSE",
                Error::DutyCycleLimit => "DUTY_CYCLE_LIMIT",
                Error::BadRequest => "BAD_REQUEST",
                Error::NotAuthorized => "NOT_AUTHORIZED",
            }
        }
        /// Creates an enum from field names used in the ProtoBuf definition.
        pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
            match value {
                "NONE" => Some(Self::None),
                "NO_ROUTE" => Some(Self::NoRoute),
                "GOT_NAK" => Some(Self::GotNak),
                "TIMEOUT" => Some(Self::Timeout),
                "NO_INTERFACE" => Some(Self::NoInterface),
                "MAX_RETRANSMIT" => Some(Self::MaxRetransmit),
                "NO_CHANNEL" => Some(Self::NoChannel),
                "TOO_LARGE" => Some(Self::TooLarge),
                "NO_RESPONSE" => Some(Self::NoResponse),
                "DUTY_CYCLE_LIMIT" => Some(Self::DutyCycleLimit),
                "BAD_REQUEST" => Some(Self::BadRequest),
                "NOT_AUTHORIZED" => Some(Self::NotAuthorized),
                _ => None,
            }
        }
    }
    #[derive(serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    #[allow(clippy::doc_lazy_continuation)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Variant {
        ///
        /// A route request going from the requester
        #[prost(message, tag = "1")]
        RouteRequest(super::RouteDiscovery),
        ///
        /// A route reply
        #[prost(message, tag = "2")]
        RouteReply(super::RouteDiscovery),
        ///
        /// A failure in delivering a message (usually used for routing control messages, but might be provided
        /// in addition to ack.fail_id to provide details on the type of failure).
        #[prost(enumeration = "Error", tag = "3")]
        ErrorReason(i32),
    }
}
///
/// (Formerly called SubPacket)
/// The payload portion fo a packet, this is the actual bytes that are sent
/// inside a radio packet (because from/to are broken out by the comms library)
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::doc_lazy_continuation)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Data {
    ///
    /// Formerly named typ and of type Type
    #[prost(enumeration = "PortNum", tag = "1")]
    pub portnum: i32,
    ///
    /// TODO: REPLACE
    #[prost(bytes = "vec", tag = "2")]
    pub payload: ::prost::alloc::vec::Vec<u8>,
    ///
    /// Not normally used, but for testing a sender can request that recipient
    /// responds in kind (i.e. if it received a position, it should unicast back it's position).
    /// Note: that if you set this on a broadcast you will receive many replies.
    #[prost(bool, tag = "3")]
    pub want_response: bool,
    ///
    /// The address of the destination node.
    /// This field is is filled in by the mesh radio device software, application
    /// layer software should never need it.
    /// RouteDiscovery messages _must_ populate this.
    /// Other message types might need to if they are doing multihop routing.
    #[prost(fixed32, tag = "4")]
    pub dest: u32,
    ///
    /// The address of the original sender for this message.
    /// This field should _only_ be populated for reliable multihop packets (to keep
    /// packets small).
    #[prost(fixed32, tag = "5")]
    pub source: u32,
    ///
    /// Only used in routing or response messages.
    /// Indicates the original message ID that this message is reporting failure on. (formerly called original_id)
    #[prost(fixed32, tag = "6")]
    pub request_id: u32,
    ///
    /// If set, this message is intened to be a reply to a previously sent message with the defined id.
    #[prost(fixed32, tag = "7")]
    pub reply_id: u32,
    ///
    /// Defaults to false. If true, then what is in the payload should be treated as an emoji like giving
    /// a message a heart or poop emoji.
    #[prost(fixed32, tag = "8")]
    pub emoji: u32,
}
///
/// Waypoint message, used to share arbitrary locations across the mesh
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::doc_lazy_continuation)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Waypoint {
    ///
    /// Id of the waypoint
    #[prost(uint32, tag = "1")]
    pub id: u32,
    ///
    /// latitude_i
    #[prost(sfixed32, tag = "2")]
    pub latitude_i: i32,
    ///
    /// longitude_i
    #[prost(sfixed32, tag = "3")]
    pub longitude_i: i32,
    ///
    /// Time the waypoint is to expire (epoch)
    #[prost(uint32, tag = "4")]
    pub expire: u32,
    ///
    /// If greater than zero, treat the value as a nodenum only allowing them to update the waypoint.
    /// If zero, the waypoint is open to be edited by any member of the mesh.
    #[prost(uint32, tag = "5")]
    pub locked_to: u32,
    ///
    /// Name of the waypoint - max 30 chars
    #[prost(string, tag = "6")]
    pub name: ::prost::alloc::string::String,
    ///
    /// Description of the waypoint - max 100 chars
    #[prost(string, tag = "7")]
    pub description: ::prost::alloc::string::String,
    ///
    /// Designator icon for the waypoint in the form of a unicode emoji
    #[prost(fixed32, tag = "8")]
    pub icon: u32,
}
///
/// This message will be proxied over the PhoneAPI for the client to deliver to the MQTT server
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::doc_lazy_continuation)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MqttClientProxyMessage {
    ///
    /// The MQTT topic this message will be sent /received on
    #[prost(string, tag = "1")]
    pub topic: ::prost::alloc::string::String,
    ///
    /// Whether the message should be retained (or not)
    #[prost(bool, tag = "4")]
    pub retained: bool,
    ///
    /// The actual service envelope payload or text for mqtt pub / sub
    #[prost(oneof = "mqtt_client_proxy_message::PayloadVariant", tags = "2, 3")]
    pub payload_variant: ::core::option::Option<
        mqtt_client_proxy_message::PayloadVariant,
    >,
}
/// Nested message and enum types in `MqttClientProxyMessage`.
pub mod mqtt_client_proxy_message {
    ///
    /// The actual service envelope payload or text for mqtt pub / sub
    #[derive(serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    #[allow(clippy::doc_lazy_continuation)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum PayloadVariant {
        ///
        /// Bytes
        #[prost(bytes, tag = "2")]
        Data(::prost::alloc::vec::Vec<u8>),
        ///
        /// Text
        #[prost(string, tag = "3")]
        Text(::prost::alloc::string::String),
    }
}
///
/// A packet envelope sent/received over the mesh
/// only payload_variant is sent in the payload portion of the LORA packet.
/// The other fields are either not sent at all, or sent in the special 16 byte LORA header.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::doc_lazy_continuation)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MeshPacket {
    ///
    /// The sending node number.
    /// Note: Our crypto implementation uses this field as well.
    /// See \[crypto\](/docs/overview/encryption) for details.
    #[prost(fixed32, tag = "1")]
    pub from: u32,
    ///
    /// The (immediate) destination for this packet
    #[prost(fixed32, tag = "2")]
    pub to: u32,
    ///
    /// (Usually) If set, this indicates the index in the secondary_channels table that this packet was sent/received on.
    /// If unset, packet was on the primary channel.
    /// A particular node might know only a subset of channels in use on the mesh.
    /// Therefore channel_index is inherently a local concept and meaningless to send between nodes.
    /// Very briefly, while sending and receiving deep inside the device Router code, this field instead
    /// contains the 'channel hash' instead of the index.
    /// This 'trick' is only used while the payload_variant is an 'encrypted'.
    #[prost(uint32, tag = "3")]
    pub channel: u32,
    ///
    /// A unique ID for this packet.
    /// Always 0 for no-ack packets or non broadcast packets (and therefore take zero bytes of space).
    /// Otherwise a unique ID for this packet, useful for flooding algorithms.
    /// ID only needs to be unique on a _per sender_ basis, and it only
    /// needs to be unique for a few minutes (long enough to last for the length of
    /// any ACK or the completion of a mesh broadcast flood).
    /// Note: Our crypto implementation uses this id as well.
    /// See \[crypto\](/docs/overview/encryption) for details.
    #[prost(fixed32, tag = "6")]
    pub id: u32,
    ///
    /// The time this message was received by the esp32 (secs since 1970).
    /// Note: this field is _never_ sent on the radio link itself (to save space) Times
    /// are typically not sent over the mesh, but they will be added to any Packet
    /// (chain of SubPacket) sent to the phone (so the phone can know exact time of reception)
    #[prost(fixed32, tag = "7")]
    pub rx_time: u32,
    ///
    /// *Never* sent over the radio links.
    /// Set during reception to indicate the SNR of this packet.
    /// Used to collect statistics on current link quality.
    #[prost(float, tag = "8")]
    pub rx_snr: f32,
    ///
    /// If unset treated as zero (no forwarding, send to adjacent nodes only)
    /// if 1, allow hopping through one node, etc...
    /// For our usecase real world topologies probably have a max of about 3.
    /// This field is normally placed into a few of bits in the header.
    #[prost(uint32, tag = "9")]
    pub hop_limit: u32,
    ///
    /// This packet is being sent as a reliable message, we would prefer it to arrive at the destination.
    /// We would like to receive a ack packet in response.
    /// Broadcasts messages treat this flag specially: Since acks for broadcasts would
    /// rapidly flood the channel, the normal ack behavior is suppressed.
    /// Instead, the original sender listens to see if at least one node is rebroadcasting this packet (because naive flooding algorithm).
    /// If it hears that the odds (given typical LoRa topologies) the odds are very high that every node should eventually receive the message.
    /// So FloodingRouter.cpp generates an implicit ack which is delivered to the original sender.
    /// If after some time we don't hear anyone rebroadcast our packet, we will timeout and retransmit, using the regular resend logic.
    /// Note: This flag is normally sent in a flag bit in the header when sent over the wire
    #[prost(bool, tag = "10")]
    pub want_ack: bool,
    ///
    /// The priority of this message for sending.
    /// See MeshPacket.Priority description for more details.
    #[prost(enumeration = "mesh_packet::Priority", tag = "11")]
    pub priority: i32,
    ///
    /// rssi of received packet. Only sent to phone for dispay purposes.
    #[prost(int32, tag = "12")]
    pub rx_rssi: i32,
    ///
    /// Describe if this message is delayed
    #[deprecated]
    #[prost(enumeration = "mesh_packet::Delayed", tag = "13")]
    pub delayed: i32,
    ///
    /// Describes whether this packet passed via MQTT somewhere along the path it currently took.
    #[prost(bool, tag = "14")]
    pub via_mqtt: bool,
    ///
    /// Hop limit with which the original packet started. Sent via LoRa using three bits in the unencrypted header.
    /// When receiving a packet, the difference between hop_start and hop_limit gives how many hops it traveled.
    #[prost(uint32, tag = "15")]
    pub hop_start: u32,
    #[prost(oneof = "mesh_packet::PayloadVariant", tags = "4, 5")]
    pub payload_variant: ::core::option::Option<mesh_packet::PayloadVariant>,
}
/// Nested message and enum types in `MeshPacket`.
pub mod mesh_packet {
    ///
    /// The priority of this message for sending.
    /// Higher priorities are sent first (when managing the transmit queue).
    /// This field is never sent over the air, it is only used internally inside of a local device node.
    /// API clients (either on the local node or connected directly to the node)
    /// can set this parameter if necessary.
    /// (values must be <= 127 to keep protobuf field to one byte in size.
    /// Detailed background on this field:
    /// I noticed a funny side effect of lora being so slow: Usually when making
    /// a protocol there isn’t much need to use message priority to change the order
    /// of transmission (because interfaces are fairly fast).
    /// But for lora where packets can take a few seconds each, it is very important
    /// to make sure that critical packets are sent ASAP.
    /// In the case of meshtastic that means we want to send protocol acks as soon as possible
    /// (to prevent unneeded retransmissions), we want routing messages to be sent next,
    /// then messages marked as reliable and finally 'background' packets like periodic position updates.
    /// So I bit the bullet and implemented a new (internal - not sent over the air)
    /// field in MeshPacket called 'priority'.
    /// And the transmission queue in the router object is now a priority queue.
    #[derive(serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    #[allow(clippy::doc_lazy_continuation)]
    #[derive(
        Clone,
        Copy,
        Debug,
        PartialEq,
        Eq,
        Hash,
        PartialOrd,
        Ord,
        ::prost::Enumeration
    )]
    #[repr(i32)]
    pub enum Priority {
        ///
        /// Treated as Priority.DEFAULT
        Unset = 0,
        ///
        /// TODO: REPLACE
        Min = 1,
        ///
        /// Background position updates are sent with very low priority -
        /// if the link is super congested they might not go out at all
        Background = 10,
        ///
        /// This priority is used for most messages that don't have a priority set
        Default = 64,
        ///
        /// If priority is unset but the message is marked as want_ack,
        /// assume it is important and use a slightly higher priority
        Reliable = 70,
        ///
        /// Ack/naks are sent with very high priority to ensure that retransmission
        /// stops as soon as possible
        Ack = 120,
        ///
        /// TODO: REPLACE
        Max = 127,
    }
    impl Priority {
        /// String value of the enum field names used in the ProtoBuf definition.
        ///
        /// The values are not transformed in any way and thus are considered stable
        /// (if the ProtoBuf definition does not change) and safe for programmatic use.
        pub fn as_str_name(&self) -> &'static str {
            match self {
                Priority::Unset => "UNSET",
                Priority::Min => "MIN",
                Priority::Background => "BACKGROUND",
                Priority::Default => "DEFAULT",
                Priority::Reliable => "RELIABLE",
                Priority::Ack => "ACK",
                Priority::Max => "MAX",
            }
        }
        /// Creates an enum from field names used in the ProtoBuf definition.
        pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
            match value {
                "UNSET" => Some(Self::Unset),
                "MIN" => Some(Self::Min),
                "BACKGROUND" => Some(Self::Background),
                "DEFAULT" => Some(Self::Default),
                "RELIABLE" => Some(Self::Reliable),
                "ACK" => Some(Self::Ack),
                "MAX" => Some(Self::Max),
                _ => None,
            }
        }
    }
    ///
    /// Identify if this is a delayed packet
    #[derive(serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    #[allow(clippy::doc_lazy_continuation)]
    #[derive(
        Clone,
        Copy,
        Debug,
        PartialEq,
        Eq,
        Hash,
        PartialOrd,
        Ord,
        ::prost::Enumeration
    )]
    #[repr(i32)]
    pub enum Delayed {
        ///
        /// If unset, the message is being sent in real time.
        NoDelay = 0,
        ///
        /// The message is delayed and was originally a broadcast
        Broadcast = 1,
        ///
        /// The message is delayed and was originally a direct message
        Direct = 2,
    }
    impl Delayed {
        /// String value of the enum field names used in the ProtoBuf definition.
        ///
        /// The values are not transformed in any way and thus are considered stable
        /// (if the ProtoBuf definition does not change) and safe for programmatic use.
        pub fn as_str_name(&self) -> &'static str {
            match self {
                Delayed::NoDelay => "NO_DELAY",
                Delayed::Broadcast => "DELAYED_BROADCAST",
                Delayed::Direct => "DELAYED_DIRECT",
            }
        }
        /// Creates an enum from field names used in the ProtoBuf definition.
        pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
            match value {
                "NO_DELAY" => Some(Self::NoDelay),
                "DELAYED_BROADCAST" => Some(Self::Broadcast),
                "DELAYED_DIRECT" => Some(Self::Direct),
                _ => None,
            }
        }
    }
    #[derive(serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    #[allow(clippy::doc_lazy_continuation)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum PayloadVariant {
        ///
        /// TODO: REPLACE
        #[prost(message, tag = "4")]
        Decoded(super::Data),
        ///
        /// TODO: REPLACE
        #[prost(bytes, tag = "5")]
        Encrypted(::prost::alloc::vec::Vec<u8>),
    }
}
///
/// The bluetooth to device link:
/// Old BTLE protocol docs from TODO, merge in above and make real docs...
/// use protocol buffers, and NanoPB
/// messages from device to phone:
/// POSITION_UPDATE (..., time)
/// TEXT_RECEIVED(from, text, time)
/// OPAQUE_RECEIVED(from, payload, time) (for signal messages or other applications)
/// messages from phone to device:
/// SET_MYID(id, human readable long, human readable short) (send down the unique ID
/// string used for this node, a human readable string shown for that id, and a very
/// short human readable string suitable for oled screen) SEND_OPAQUE(dest, payload)
/// (for signal messages or other applications) SEND_TEXT(dest, text) Get all
/// nodes() (returns list of nodes, with full info, last time seen, loc, battery
/// level etc) SET_CONFIG (switches device to a new set of radio params and
/// preshared key, drops all existing nodes, force our node to rejoin this new group)
/// Full information about a node on the mesh
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::doc_lazy_continuation)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct NodeInfo {
    ///
    /// The node number
    #[prost(uint32, tag = "1")]
    pub num: u32,
    ///
    /// The user info for this node
    #[prost(message, optional, tag = "2")]
    pub user: ::core::option::Option<User>,
    ///
    /// This position data. Note: before 1.2.14 we would also store the last time we've heard from this node in position.time, that is no longer true.
    /// Position.time now indicates the last time we received a POSITION from that node.
    #[prost(message, optional, tag = "3")]
    pub position: ::core::option::Option<Position>,
    ///
    /// Returns the Signal-to-noise ratio (SNR) of the last received message,
    /// as measured by the receiver. Return SNR of the last received message in dB
    #[prost(float, tag = "4")]
    pub snr: f32,
    ///
    /// Set to indicate the last time we received a packet from this node
    #[prost(fixed32, tag = "5")]
    pub last_heard: u32,
    ///
    /// The latest device metrics for the node.
    #[prost(message, optional, tag = "6")]
    pub device_metrics: ::core::option::Option<DeviceMetrics>,
    ///
    /// local channel index we heard that node on. Only populated if its not the default channel.
    #[prost(uint32, tag = "7")]
    pub channel: u32,
    ///
    /// True if we witnessed the node over MQTT instead of LoRA transport
    #[prost(bool, tag = "8")]
    pub via_mqtt: bool,
    ///
    /// Number of hops away from us this node is (0 if adjacent)
    #[prost(uint32, tag = "9")]
    pub hops_away: u32,
    ///
    /// True if node is in our favorites list
    /// Persists between NodeDB internal clean ups
    #[prost(bool, tag = "10")]
    pub is_favorite: bool,
}
///
/// Unique local debugging info for this node
/// Note: we don't include position or the user info, because that will come in the
/// Sent to the phone in response to WantNodes.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::doc_lazy_continuation)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MyNodeInfo {
    ///
    /// Tells the phone what our node number is, default starting value is
    /// lowbyte of macaddr, but it will be fixed if that is already in use
    #[prost(uint32, tag = "1")]
    pub my_node_num: u32,
    ///
    /// The total number of reboots this node has ever encountered
    /// (well - since the last time we discarded preferences)
    #[prost(uint32, tag = "8")]
    pub reboot_count: u32,
    ///
    /// The minimum app version that can talk to this device.
    /// Phone/PC apps should compare this to their build number and if too low tell the user they must update their app
    #[prost(uint32, tag = "11")]
    pub min_app_version: u32,
}
///
/// Debug output from the device.
/// To minimize the size of records inside the device code, if a time/source/level is not set
/// on the message it is assumed to be a continuation of the previously sent message.
/// This allows the device code to use fixed maxlen 64 byte strings for messages,
/// and then extend as needed by emitting multiple records.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::doc_lazy_continuation)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct LogRecord {
    ///
    /// Log levels, chosen to match python logging conventions.
    #[prost(string, tag = "1")]
    pub message: ::prost::alloc::string::String,
    ///
    /// Seconds since 1970 - or 0 for unknown/unset
    #[prost(fixed32, tag = "2")]
    pub time: u32,
    ///
    /// Usually based on thread name - if known
    #[prost(string, tag = "3")]
    pub source: ::prost::alloc::string::String,
    ///
    /// Not yet set
    #[prost(enumeration = "log_record::Level", tag = "4")]
    pub level: i32,
}
/// Nested message and enum types in `LogRecord`.
pub mod log_record {
    ///
    /// Log levels, chosen to match python logging conventions.
    #[derive(serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    #[allow(clippy::doc_lazy_continuation)]
    #[derive(
        Clone,
        Copy,
        Debug,
        PartialEq,
        Eq,
        Hash,
        PartialOrd,
        Ord,
        ::prost::Enumeration
    )]
    #[repr(i32)]
    pub enum Level {
        ///
        /// Log levels, chosen to match python logging conventions.
        Unset = 0,
        ///
        /// Log levels, chosen to match python logging conventions.
        Critical = 50,
        ///
        /// Log levels, chosen to match python logging conventions.
        Error = 40,
        ///
        /// Log levels, chosen to match python logging conventions.
        Warning = 30,
        ///
        /// Log levels, chosen to match python logging conventions.
        Info = 20,
        ///
        /// Log levels, chosen to match python logging conventions.
        Debug = 10,
        ///
        /// Log levels, chosen to match python logging conventions.
        Trace = 5,
    }
    impl Level {
        /// String value of the enum field names used in the ProtoBuf definition.
        ///
        /// The values are not transformed in any way and thus are considered stable
        /// (if the ProtoBuf definition does not change) and safe for programmatic use.
        pub fn as_str_name(&self) -> &'static str {
            match self {
                Level::Unset => "UNSET",
                Level::Critical => "CRITICAL",
                Level::Error => "ERROR",
                Level::Warning => "WARNING",
                Level::Info => "INFO",
                Level::Debug => "DEBUG",
                Level::Trace => "TRACE",
            }
        }
        /// Creates an enum from field names used in the ProtoBuf definition.
        pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
            match value {
                "UNSET" => Some(Self::Unset),
                "CRITICAL" => Some(Self::Critical),
                "ERROR" => Some(Self::Error),
                "WARNING" => Some(Self::Warning),
                "INFO" => Some(Self::Info),
                "DEBUG" => Some(Self::Debug),
                "TRACE" => Some(Self::Trace),
                _ => None,
            }
        }
    }
}
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::doc_lazy_continuation)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct QueueStatus {
    /// Last attempt to queue status, ErrorCode
    #[prost(int32, tag = "1")]
    pub res: i32,
    /// Free entries in the outgoing queue
    #[prost(uint32, tag = "2")]
    pub free: u32,
    /// Maximum entries in the outgoing queue
    #[prost(uint32, tag = "3")]
    pub maxlen: u32,
    /// What was mesh packet id that generated this response?
    #[prost(uint32, tag = "4")]
    pub mesh_packet_id: u32,
}
///
/// Packets from the radio to the phone will appear on the fromRadio characteristic.
/// It will support READ and NOTIFY. When a new packet arrives the device will BLE notify?
/// It will sit in that descriptor until consumed by the phone,
/// at which point the next item in the FIFO will be populated.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::doc_lazy_continuation)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct FromRadio {
    ///
    /// The packet id, used to allow the phone to request missing read packets from the FIFO,
    /// see our bluetooth docs
    #[prost(uint32, tag = "1")]
    pub id: u32,
    ///
    /// Log levels, chosen to match python logging conventions.
    #[prost(
        oneof = "from_radio::PayloadVariant",
        tags = "2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14"
    )]
    pub payload_variant: ::core::option::Option<from_radio::PayloadVariant>,
}
/// Nested message and enum types in `FromRadio`.
pub mod from_radio {
    ///
    /// Log levels, chosen to match python logging conventions.
    #[derive(serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    #[allow(clippy::doc_lazy_continuation)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum PayloadVariant {
        ///
        /// Log levels, chosen to match python logging conventions.
        #[prost(message, tag = "2")]
        Packet(super::MeshPacket),
        ///
        /// Tells the phone what our node number is, can be -1 if we've not yet joined a mesh.
        /// NOTE: This ID must not change - to keep (minimal) compatibility with <1.2 version of android apps.
        #[prost(message, tag = "3")]
        MyInfo(super::MyNodeInfo),
        ///
        /// One packet is sent for each node in the on radio DB
        /// starts over with the first node in our DB
        #[prost(message, tag = "4")]
        NodeInfo(super::NodeInfo),
        ///
        /// Include a part of the config (was: RadioConfig radio)
        #[prost(message, tag = "5")]
        Config(super::Config),
        ///
        /// Set to send debug console output over our protobuf stream
        #[prost(message, tag = "6")]
        LogRecord(super::LogRecord),
        ///
        /// Sent as true once the device has finished sending all of the responses to want_config
        /// recipient should check if this ID matches our original request nonce, if
        /// not, it means your config responses haven't started yet.
        /// NOTE: This ID must not change - to keep (minimal) compatibility with <1.2 version of android apps.
        #[prost(uint32, tag = "7")]
        ConfigCompleteId(u32),
        ///
        /// Sent to tell clients the radio has just rebooted.
        /// Set to true if present.
        /// Not used on all transports, currently just used for the serial console.
        /// NOTE: This ID must not change - to keep (minimal) compatibility with <1.2 version of android apps.
        #[prost(bool, tag = "8")]
        Rebooted(bool),
        ///
        /// Include module config
        #[prost(message, tag = "9")]
        ModuleConfig(super::ModuleConfig),
        ///
        /// One packet is sent for each channel
        #[prost(message, tag = "10")]
        Channel(super::Channel),
        ///
        /// Queue status info
        #[prost(message, tag = "11")]
        QueueStatus(super::QueueStatus),
        ///
        /// File Transfer Chunk
        #[prost(message, tag = "12")]
        XmodemPacket(super::XModem),
        ///
        /// Device metadata message
        #[prost(message, tag = "13")]
        Metadata(super::DeviceMetadata),
        ///
        /// MQTT Client Proxy Message (device sending to client / phone for publishing to MQTT)
        #[prost(message, tag = "14")]
        MqttClientProxyMessage(super::MqttClientProxyMessage),
    }
}
///
/// Packets/commands to the radio will be written (reliably) to the toRadio characteristic.
/// Once the write completes the phone can assume it is handled.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::doc_lazy_continuation)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ToRadio {
    ///
    /// Log levels, chosen to match python logging conventions.
    #[prost(oneof = "to_radio::PayloadVariant", tags = "1, 3, 4, 5, 6, 7")]
    pub payload_variant: ::core::option::Option<to_radio::PayloadVariant>,
}
/// Nested message and enum types in `ToRadio`.
pub mod to_radio {
    ///
    /// Log levels, chosen to match python logging conventions.
    #[derive(serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    #[allow(clippy::doc_lazy_continuation)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum PayloadVariant {
        ///
        /// Send this packet on the mesh
        #[prost(message, tag = "1")]
        Packet(super::MeshPacket),
        ///
        /// Phone wants radio to send full node db to the phone, This is
        /// typically the first packet sent to the radio when the phone gets a
        /// bluetooth connection. The radio will respond by sending back a
        /// MyNodeInfo, a owner, a radio config and a series of
        /// FromRadio.node_infos, and config_complete
        /// the integer you write into this field will be reported back in the
        /// config_complete_id response this allows clients to never be confused by
        /// a stale old partially sent config.
        #[prost(uint32, tag = "3")]
        WantConfigId(u32),
        ///
        /// Tell API server we are disconnecting now.
        /// This is useful for serial links where there is no hardware/protocol based notification that the client has dropped the link.
        /// (Sending this message is optional for clients)
        #[prost(bool, tag = "4")]
        Disconnect(bool),
        #[prost(message, tag = "5")]
        XmodemPacket(super::XModem),
        ///
        /// MQTT Client Proxy Message (for client / phone subscribed to MQTT sending to device)
        #[prost(message, tag = "6")]
        MqttClientProxyMessage(super::MqttClientProxyMessage),
        ///
        /// Heartbeat message (used to keep the device connection awake on serial)
        #[prost(message, tag = "7")]
        Heartbeat(super::Heartbeat),
    }
}
///
/// Compressed message payload
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::doc_lazy_continuation)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Compressed {
    ///
    /// PortNum to determine the how to handle the compressed payload.
    #[prost(enumeration = "PortNum", tag = "1")]
    pub portnum: i32,
    ///
    /// Compressed data.
    #[prost(bytes = "vec", tag = "2")]
    pub data: ::prost::alloc::vec::Vec<u8>,
}
///
/// Full info on edges for a single node
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::doc_lazy_continuation)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct NeighborInfo {
    ///
    /// The node ID of the node sending info on its neighbors
    #[prost(uint32, tag = "1")]
    pub node_id: u32,
    ///
    /// Field to pass neighbor info for the next sending cycle
    #[prost(uint32, tag = "2")]
    pub last_sent_by_id: u32,
    ///
    /// Broadcast interval of the represented node (in seconds)
    #[prost(uint32, tag = "3")]
    pub node_broadcast_interval_secs: u32,
    ///
    /// The list of out edges from this node
    #[prost(message, repeated, tag = "4")]
    pub neighbors: ::prost::alloc::vec::Vec<Neighbor>,
}
///
/// A single edge in the mesh
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::doc_lazy_continuation)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Neighbor {
    ///
    /// Node ID of neighbor
    #[prost(uint32, tag = "1")]
    pub node_id: u32,
    ///
    /// SNR of last heard message
    #[prost(float, tag = "2")]
    pub snr: f32,
    ///
    /// Reception time (in secs since 1970) of last message that was last sent by this ID.
    /// Note: this is for local storage only and will not be sent out over the mesh.
    #[prost(fixed32, tag = "3")]
    pub last_rx_time: u32,
    ///
    /// Broadcast interval of this neighbor (in seconds).
    /// Note: this is for local storage only and will not be sent out over the mesh.
    #[prost(uint32, tag = "4")]
    pub node_broadcast_interval_secs: u32,
}
///
/// Device metadata response
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::doc_lazy_continuation)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DeviceMetadata {
    ///
    /// Device firmware version string
    #[prost(string, tag = "1")]
    pub firmware_version: ::prost::alloc::string::String,
    ///
    /// Device state version
    #[prost(uint32, tag = "2")]
    pub device_state_version: u32,
    ///
    /// Indicates whether the device can shutdown CPU natively or via power management chip
    #[prost(bool, tag = "3")]
    pub can_shutdown: bool,
    ///
    /// Indicates that the device has native wifi capability
    #[prost(bool, tag = "4")]
    pub has_wifi: bool,
    ///
    /// Indicates that the device has native bluetooth capability
    #[prost(bool, tag = "5")]
    pub has_bluetooth: bool,
    ///
    /// Indicates that the device has an ethernet peripheral
    #[prost(bool, tag = "6")]
    pub has_ethernet: bool,
    ///
    /// Indicates that the device's role in the mesh
    #[prost(enumeration = "config::device_config::Role", tag = "7")]
    pub role: i32,
    ///
    /// Indicates the device's current enabled position flags
    #[prost(uint32, tag = "8")]
    pub position_flags: u32,
    ///
    /// Device hardware model
    #[prost(enumeration = "HardwareModel", tag = "9")]
    pub hw_model: i32,
    ///
    /// Has Remote Hardware enabled
    #[prost(bool, tag = "10")]
    pub has_remote_hardware: bool,
}
///
/// A heartbeat message is sent to the node from the client to keep the connection alive.
/// This is currently only needed to keep serial connections alive, but can be used by any PhoneAPI.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::doc_lazy_continuation)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Heartbeat {}
///
/// RemoteHardwarePins associated with a node
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::doc_lazy_continuation)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct NodeRemoteHardwarePin {
    ///
    /// The node_num exposing the available gpio pin
    #[prost(uint32, tag = "1")]
    pub node_num: u32,
    ///
    /// The the available gpio pin for usage with RemoteHardware module
    #[prost(message, optional, tag = "2")]
    pub pin: ::core::option::Option<RemoteHardwarePin>,
}
///
/// Note: these enum names must EXACTLY match the string used in the device
/// bin/build-all.sh script.
/// Because they will be used to find firmware filenames in the android app for OTA updates.
/// To match the old style filenames, _ is converted to -, p is converted to .
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::doc_lazy_continuation)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum HardwareModel {
    ///
    /// TODO: REPLACE
    Unset = 0,
    ///
    /// TODO: REPLACE
    TloraV2 = 1,
    ///
    /// TODO: REPLACE
    TloraV1 = 2,
    ///
    /// TODO: REPLACE
    TloraV211p6 = 3,
    ///
    /// TODO: REPLACE
    Tbeam = 4,
    ///
    /// The original heltec WiFi_Lora_32_V2, which had battery voltage sensing hooked to GPIO 13
    /// (see HELTEC_V2 for the new version).
    HeltecV20 = 5,
    ///
    /// TODO: REPLACE
    TbeamV0p7 = 6,
    ///
    /// TODO: REPLACE
    TEcho = 7,
    ///
    /// TODO: REPLACE
    TloraV11p3 = 8,
    ///
    /// TODO: REPLACE
    Rak4631 = 9,
    ///
    /// The new version of the heltec WiFi_Lora_32_V2 board that has battery sensing hooked to GPIO 37.
    /// Sadly they did not update anything on the silkscreen to identify this board
    HeltecV21 = 10,
    ///
    /// Ancient heltec WiFi_Lora_32 board
    HeltecV1 = 11,
    ///
    /// New T-BEAM with ESP32-S3 CPU
    LilygoTbeamS3Core = 12,
    ///
    /// RAK WisBlock ESP32 core: <https://docs.rakwireless.com/Product-Categories/WisBlock/RAK11200/Overview/>
    Rak11200 = 13,
    ///
    /// B&Q Consulting Nano Edition G1: <https://uniteng.com/wiki/doku.php?id=meshtastic:nano>
    NanoG1 = 14,
    ///
    /// TODO: REPLACE
    TloraV211p8 = 15,
    ///
    /// TODO: REPLACE
    TloraT3S3 = 16,
    ///
    /// B&Q Consulting Nano G1 Explorer: <https://wiki.uniteng.com/en/meshtastic/nano-g1-explorer>
    NanoG1Explorer = 17,
    ///
    /// B&Q Consulting Nano G2 Ultra: <https://wiki.uniteng.com/en/meshtastic/nano-g2-ultra>
    NanoG2Ultra = 18,
    ///
    /// LoRAType device: <https://loratype.org/>
    LoraType = 19,
    ///
    /// B&Q Consulting Station Edition G1: <https://uniteng.com/wiki/doku.php?id=meshtastic:station>
    StationG1 = 25,
    ///
    /// RAK11310 (RP2040 + SX1262)
    Rak11310 = 26,
    ///
    /// Makerfabs SenseLoRA Receiver (RP2040 + RFM96)
    SenseloraRp2040 = 27,
    ///
    /// Makerfabs SenseLoRA Industrial Monitor (ESP32-S3 + RFM96)
    SenseloraS3 = 28,
    ///
    /// Canary Radio Company - CanaryOne: <https://canaryradio.io/products/canaryone>
    Canaryone = 29,
    ///
    /// Waveshare RP2040 LoRa - <https://www.waveshare.com/rp2040-lora.htm>
    Rp2040Lora = 30,
    ///
    /// B&Q Consulting Station G2: <https://wiki.uniteng.com/en/meshtastic/station-g2>
    StationG2 = 31,
    ///
    /// ---------------------------------------------------------------------------
    /// Less common/prototype boards listed here (needs one more byte over the air)
    /// ---------------------------------------------------------------------------
    LoraRelayV1 = 32,
    ///
    /// TODO: REPLACE
    Nrf52840dk = 33,
    ///
    /// TODO: REPLACE
    Ppr = 34,
    ///
    /// TODO: REPLACE
    Genieblocks = 35,
    ///
    /// TODO: REPLACE
    Nrf52Unknown = 36,
    ///
    /// TODO: REPLACE
    Portduino = 37,
    ///
    /// The simulator built into the android app
    AndroidSim = 38,
    ///
    /// Custom DIY device based on @NanoVHF schematics: <https://github.com/NanoVHF/Meshtastic-DIY/tree/main/Schematics>
    DiyV1 = 39,
    ///
    /// nRF52840 Dongle : <https://www.nordicsemi.com/Products/Development-hardware/nrf52840-dongle/>
    Nrf52840Pca10059 = 40,
    ///
    /// Custom Disaster Radio esp32 v3 device <https://github.com/sudomesh/disaster-radio/tree/master/hardware/board_esp32_v3>
    DrDev = 41,
    ///
    /// M5 esp32 based MCU modules with enclosure, TFT and LORA Shields. All Variants (Basic, Core, Fire, Core2, Paper) <https://m5stack.com/>
    M5stack = 42,
    ///
    /// New Heltec LoRA32 with ESP32-S3 CPU
    HeltecV3 = 43,
    ///
    /// New Heltec Wireless Stick Lite with ESP32-S3 CPU
    HeltecWslV3 = 44,
    ///
    /// New BETAFPV ELRS Micro TX Module 2.4G with ESP32 CPU
    Betafpv2400Tx = 45,
    ///
    /// BetaFPV ExpressLRS "Nano" TX Module 900MHz with ESP32 CPU
    Betafpv900NanoTx = 46,
    ///
    /// Raspberry Pi Pico (W) with Waveshare SX1262 LoRa Node Module
    RpiPico = 47,
    ///
    /// Heltec Wireless Tracker with ESP32-S3 CPU, built-in GPS, and TFT
    /// Newer V1.1, version is written on the PCB near the display.
    HeltecWirelessTracker = 48,
    ///
    /// Heltec Wireless Paper with ESP32-S3 CPU and E-Ink display
    HeltecWirelessPaper = 49,
    ///
    /// LilyGo T-Deck with ESP32-S3 CPU, Keyboard and IPS display
    TDeck = 50,
    ///
    /// LilyGo T-Watch S3 with ESP32-S3 CPU and IPS display
    TWatchS3 = 51,
    ///
    /// Bobricius Picomputer with ESP32-S3 CPU, Keyboard and IPS display
    PicomputerS3 = 52,
    ///
    /// Heltec HT-CT62 with ESP32-C3 CPU and SX1262 LoRa
    HeltecHt62 = 53,
    ///
    /// EBYTE SPI LoRa module and ESP32-S3
    EbyteEsp32S3 = 54,
    ///
    /// Waveshare ESP32-S3-PICO with PICO LoRa HAT and 2.9inch e-Ink
    Esp32S3Pico = 55,
    ///
    /// CircuitMess Chatter 2 LLCC68 Lora Module and ESP32 Wroom
    /// Lora module can be swapped out for a Heltec RA-62 which is "almost" pin compatible
    /// with one cut and one jumper Meshtastic works
    Chatter2 = 56,
    ///
    /// Heltec Wireless Paper, With ESP32-S3 CPU and E-Ink display
    /// Older "V1.0" Variant, has no "version sticker"
    /// E-Ink model is DEPG0213BNS800
    /// Tab on the screen protector is RED
    /// Flex connector marking is FPC-7528B
    HeltecWirelessPaperV10 = 57,
    ///
    /// Heltec Wireless Tracker with ESP32-S3 CPU, built-in GPS, and TFT
    /// Older "V1.0" Variant
    HeltecWirelessTrackerV10 = 58,
    ///
    /// unPhone with ESP32-S3, TFT touchscreen,  LSM6DS3TR-C accelerometer and gyroscope
    Unphone = 59,
    ///
    /// Teledatics TD-LORAC NRF52840 based M.2 LoRA module
    /// Compatible with the TD-WRLS development board
    TdLorac = 60,
    ///
    /// CDEBYTE EoRa-S3 board using their own MM modules, clone of LILYGO T3S3
    CdebyteEoraS3 = 61,
    ///
    /// ------------------------------------------------------------------------------------------------------------------------------------------
    /// Reserved ID For developing private Ports. These will show up in live traffic sparsely, so we can use a high number. Keep it within 8 bits.
    /// ------------------------------------------------------------------------------------------------------------------------------------------
    PrivateHw = 255,
}
impl HardwareModel {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            HardwareModel::Unset => "UNSET",
            HardwareModel::TloraV2 => "TLORA_V2",
            HardwareModel::TloraV1 => "TLORA_V1",
            HardwareModel::TloraV211p6 => "TLORA_V2_1_1P6",
            HardwareModel::Tbeam => "TBEAM",
            HardwareModel::HeltecV20 => "HELTEC_V2_0",
            HardwareModel::TbeamV0p7 => "TBEAM_V0P7",
            HardwareModel::TEcho => "T_ECHO",
            HardwareModel::TloraV11p3 => "TLORA_V1_1P3",
            HardwareModel::Rak4631 => "RAK4631",
            HardwareModel::HeltecV21 => "HELTEC_V2_1",
            HardwareModel::HeltecV1 => "HELTEC_V1",
            HardwareModel::LilygoTbeamS3Core => "LILYGO_TBEAM_S3_CORE",
            HardwareModel::Rak11200 => "RAK11200",
            HardwareModel::NanoG1 => "NANO_G1",
            HardwareModel::TloraV211p8 => "TLORA_V2_1_1P8",
            HardwareModel::TloraT3S3 => "TLORA_T3_S3",
            HardwareModel::NanoG1Explorer => "NANO_G1_EXPLORER",
            HardwareModel::NanoG2Ultra => "NANO_G2_ULTRA",
            HardwareModel::LoraType => "LORA_TYPE",
            HardwareModel::StationG1 => "STATION_G1",
            HardwareModel::Rak11310 => "RAK11310",
            HardwareModel::SenseloraRp2040 => "SENSELORA_RP2040",
            HardwareModel::SenseloraS3 => "SENSELORA_S3",
            HardwareModel::Canaryone => "CANARYONE",
            HardwareModel::Rp2040Lora => "RP2040_LORA",
            HardwareModel::StationG2 => "STATION_G2",
            HardwareModel::LoraRelayV1 => "LORA_RELAY_V1",
            HardwareModel::Nrf52840dk => "NRF52840DK",
            HardwareModel::Ppr => "PPR",
            HardwareModel::Genieblocks => "GENIEBLOCKS",
            HardwareModel::Nrf52Unknown => "NRF52_UNKNOWN",
            HardwareModel::Portduino => "PORTDUINO",
            HardwareModel::AndroidSim => "ANDROID_SIM",
            HardwareModel::DiyV1 => "DIY_V1",
            HardwareModel::Nrf52840Pca10059 => "NRF52840_PCA10059",
            HardwareModel::DrDev => "DR_DEV",
            HardwareModel::M5stack => "M5STACK",
            HardwareModel::HeltecV3 => "HELTEC_V3",
            HardwareModel::HeltecWslV3 => "HELTEC_WSL_V3",
            HardwareModel::Betafpv2400Tx => "BETAFPV_2400_TX",
            HardwareModel::Betafpv900NanoTx => "BETAFPV_900_NANO_TX",
            HardwareModel::RpiPico => "RPI_PICO",
            HardwareModel::HeltecWirelessTracker => "HELTEC_WIRELESS_TRACKER",
            HardwareModel::HeltecWirelessPaper => "HELTEC_WIRELESS_PAPER",
            HardwareModel::TDeck => "T_DECK",
            HardwareModel::TWatchS3 => "T_WATCH_S3",
            HardwareModel::PicomputerS3 => "PICOMPUTER_S3",
            HardwareModel::HeltecHt62 => "HELTEC_HT62",
            HardwareModel::EbyteEsp32S3 => "EBYTE_ESP32_S3",
            HardwareModel::Esp32S3Pico => "ESP32_S3_PICO",
            HardwareModel::Chatter2 => "CHATTER_2",
            HardwareModel::HeltecWirelessPaperV10 => "HELTEC_WIRELESS_PAPER_V1_0",
            HardwareModel::HeltecWirelessTrackerV10 => "HELTEC_WIRELESS_TRACKER_V1_0",
            HardwareModel::Unphone => "UNPHONE",
            HardwareModel::TdLorac => "TD_LORAC",
            HardwareModel::CdebyteEoraS3 => "CDEBYTE_EORA_S3",
            HardwareModel::PrivateHw => "PRIVATE_HW",
        }
    }
    /// Creates an enum from field names used in the ProtoBuf definition.
    pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
        match value {
            "UNSET" => Some(Self::Unset),
            "TLORA_V2" => Some(Self::TloraV2),
            "TLORA_V1" => Some(Self::TloraV1),
            "TLORA_V2_1_1P6" => Some(Self::TloraV211p6),
            "TBEAM" => Some(Self::Tbeam),
            "HELTEC_V2_0" => Some(Self::HeltecV20),
            "TBEAM_V0P7" => Some(Self::TbeamV0p7),
            "T_ECHO" => Some(Self::TEcho),
            "TLORA_V1_1P3" => Some(Self::TloraV11p3),
            "RAK4631" => Some(Self::Rak4631),
            "HELTEC_V2_1" => Some(Self::HeltecV21),
            "HELTEC_V1" => Some(Self::HeltecV1),
            "LILYGO_TBEAM_S3_CORE" => Some(Self::LilygoTbeamS3Core),
            "RAK11200" => Some(Self::Rak11200),
            "NANO_G1" => Some(Self::NanoG1),
            "TLORA_V2_1_1P8" => Some(Self::TloraV211p8),
            "TLORA_T3_S3" => Some(Self::TloraT3S3),
            "NANO_G1_EXPLORER" => Some(Self::NanoG1Explorer),
            "NANO_G2_ULTRA" => Some(Self::NanoG2Ultra),
            "LORA_TYPE" => Some(Self::LoraType),
            "STATION_G1" => Some(Self::StationG1),
            "RAK11310" => Some(Self::Rak11310),
            "SENSELORA_RP2040" => Some(Self::SenseloraRp2040),
            "SENSELORA_S3" => Some(Self::SenseloraS3),
            "CANARYONE" => Some(Self::Canaryone),
            "RP2040_LORA" => Some(Self::Rp2040Lora),
            "STATION_G2" => Some(Self::StationG2),
            "LORA_RELAY_V1" => Some(Self::LoraRelayV1),
            "NRF52840DK" => Some(Self::Nrf52840dk),
            "PPR" => Some(Self::Ppr),
            "GENIEBLOCKS" => Some(Self::Genieblocks),
            "NRF52_UNKNOWN" => Some(Self::Nrf52Unknown),
            "PORTDUINO" => Some(Self::Portduino),
            "ANDROID_SIM" => Some(Self::AndroidSim),
            "DIY_V1" => Some(Self::DiyV1),
            "NRF52840_PCA10059" => Some(Self::Nrf52840Pca10059),
            "DR_DEV" => Some(Self::DrDev),
            "M5STACK" => Some(Self::M5stack),
            "HELTEC_V3" => Some(Self::HeltecV3),
            "HELTEC_WSL_V3" => Some(Self::HeltecWslV3),
            "BETAFPV_2400_TX" => Some(Self::Betafpv2400Tx),
            "BETAFPV_900_NANO_TX" => Some(Self::Betafpv900NanoTx),
            "RPI_PICO" => Some(Self::RpiPico),
            "HELTEC_WIRELESS_TRACKER" => Some(Self::HeltecWirelessTracker),
            "HELTEC_WIRELESS_PAPER" => Some(Self::HeltecWirelessPaper),
            "T_DECK" => Some(Self::TDeck),
            "T_WATCH_S3" => Some(Self::TWatchS3),
            "PICOMPUTER_S3" => Some(Self::PicomputerS3),
            "HELTEC_HT62" => Some(Self::HeltecHt62),
            "EBYTE_ESP32_S3" => Some(Self::EbyteEsp32S3),
            "ESP32_S3_PICO" => Some(Self::Esp32S3Pico),
            "CHATTER_2" => Some(Self::Chatter2),
            "HELTEC_WIRELESS_PAPER_V1_0" => Some(Self::HeltecWirelessPaperV10),
            "HELTEC_WIRELESS_TRACKER_V1_0" => Some(Self::HeltecWirelessTrackerV10),
            "UNPHONE" => Some(Self::Unphone),
            "TD_LORAC" => Some(Self::TdLorac),
            "CDEBYTE_EORA_S3" => Some(Self::CdebyteEoraS3),
            "PRIVATE_HW" => Some(Self::PrivateHw),
            _ => None,
        }
    }
}
///
/// Shared constants between device and phone
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::doc_lazy_continuation)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum Constants {
    ///
    /// First enum must be zero, and we are just using this enum to
    /// pass int constants between two very different environments
    Zero = 0,
    ///
    /// From mesh.options
    /// note: this payload length is ONLY the bytes that are sent inside of the Data protobuf (excluding protobuf overhead). The 16 byte header is
    /// outside of this envelope
    DataPayloadLen = 237,
}
impl Constants {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            Constants::Zero => "ZERO",
            Constants::DataPayloadLen => "DATA_PAYLOAD_LEN",
        }
    }
    /// Creates an enum from field names used in the ProtoBuf definition.
    pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
        match value {
            "ZERO" => Some(Self::Zero),
            "DATA_PAYLOAD_LEN" => Some(Self::DataPayloadLen),
            _ => None,
        }
    }
}
///
/// Error codes for critical errors
/// The device might report these fault codes on the screen.
/// If you encounter a fault code, please post on the meshtastic.discourse.group
/// and we'll try to help.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::doc_lazy_continuation)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum CriticalErrorCode {
    ///
    /// TODO: REPLACE
    None = 0,
    ///
    /// A software bug was detected while trying to send lora
    TxWatchdog = 1,
    ///
    /// A software bug was detected on entry to sleep
    SleepEnterWait = 2,
    ///
    /// No Lora radio hardware could be found
    NoRadio = 3,
    ///
    /// Not normally used
    Unspecified = 4,
    ///
    /// We failed while configuring a UBlox GPS
    UbloxUnitFailed = 5,
    ///
    /// This board was expected to have a power management chip and it is missing or broken
    NoAxp192 = 6,
    ///
    /// The channel tried to set a radio setting which is not supported by this chipset,
    /// radio comms settings are now undefined.
    InvalidRadioSetting = 7,
    ///
    /// Radio transmit hardware failure. We sent data to the radio chip, but it didn't
    /// reply with an interrupt.
    TransmitFailed = 8,
    ///
    /// We detected that the main CPU voltage dropped below the minimum acceptable value
    Brownout = 9,
    /// Selftest of SX1262 radio chip failed
    Sx1262Failure = 10,
    ///
    /// A (likely software but possibly hardware) failure was detected while trying to send packets.
    /// If this occurs on your board, please post in the forum so that we can ask you to collect some information to allow fixing this bug
    RadioSpiBug = 11,
}
impl CriticalErrorCode {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            CriticalErrorCode::None => "NONE",
            CriticalErrorCode::TxWatchdog => "TX_WATCHDOG",
            CriticalErrorCode::SleepEnterWait => "SLEEP_ENTER_WAIT",
            CriticalErrorCode::NoRadio => "NO_RADIO",
            CriticalErrorCode::Unspecified => "UNSPECIFIED",
            CriticalErrorCode::UbloxUnitFailed => "UBLOX_UNIT_FAILED",
            CriticalErrorCode::NoAxp192 => "NO_AXP192",
            CriticalErrorCode::InvalidRadioSetting => "INVALID_RADIO_SETTING",
            CriticalErrorCode::TransmitFailed => "TRANSMIT_FAILED",
            CriticalErrorCode::Brownout => "BROWNOUT",
            CriticalErrorCode::Sx1262Failure => "SX1262_FAILURE",
            CriticalErrorCode::RadioSpiBug => "RADIO_SPI_BUG",
        }
    }
    /// Creates an enum from field names used in the ProtoBuf definition.
    pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
        match value {
            "NONE" => Some(Self::None),
            "TX_WATCHDOG" => Some(Self::TxWatchdog),
            "SLEEP_ENTER_WAIT" => Some(Self::SleepEnterWait),
            "NO_RADIO" => Some(Self::NoRadio),
            "UNSPECIFIED" => Some(Self::Unspecified),
            "UBLOX_UNIT_FAILED" => Some(Self::UbloxUnitFailed),
            "NO_AXP192" => Some(Self::NoAxp192),
            "INVALID_RADIO_SETTING" => Some(Self::InvalidRadioSetting),
            "TRANSMIT_FAILED" => Some(Self::TransmitFailed),
            "BROWNOUT" => Some(Self::Brownout),
            "SX1262_FAILURE" => Some(Self::Sx1262Failure),
            "RADIO_SPI_BUG" => Some(Self::RadioSpiBug),
            _ => None,
        }
    }
}
///
/// This message is handled by the Admin module and is responsible for all settings/channel read/write operations.
/// This message is used to do settings operations to both remote AND local nodes.
/// (Prior to 1.2 these operations were done via special ToRadio operations)
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::doc_lazy_continuation)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct AdminMessage {
    ///
    /// TODO: REPLACE
    #[prost(
        oneof = "admin_message::PayloadVariant",
        tags = "1, 2, 3, 4, 5, 6, 7, 8, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 64, 65, 95, 96, 97, 98, 99, 100"
    )]
    pub payload_variant: ::core::option::Option<admin_message::PayloadVariant>,
}
/// Nested message and enum types in `AdminMessage`.
pub mod admin_message {
    ///
    /// TODO: REPLACE
    #[derive(serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    #[allow(clippy::doc_lazy_continuation)]
    #[derive(
        Clone,
        Copy,
        Debug,
        PartialEq,
        Eq,
        Hash,
        PartialOrd,
        Ord,
        ::prost::Enumeration
    )]
    #[repr(i32)]
    pub enum ConfigType {
        ///
        /// TODO: REPLACE
        DeviceConfig = 0,
        ///
        /// TODO: REPLACE
        PositionConfig = 1,
        ///
        /// TODO: REPLACE
        PowerConfig = 2,
        ///
        /// TODO: REPLACE
        NetworkConfig = 3,
        ///
        /// TODO: REPLACE
        DisplayConfig = 4,
        ///
        /// TODO: REPLACE
        LoraConfig = 5,
        ///
        /// TODO: REPLACE
        BluetoothConfig = 6,
    }
    impl ConfigType {
        /// String value of the enum field names used in the ProtoBuf definition.
        ///
        /// The values are not transformed in any way and thus are considered stable
        /// (if the ProtoBuf definition does not change) and safe for programmatic use.
        pub fn as_str_name(&self) -> &'static str {
            match self {
                ConfigType::DeviceConfig => "DEVICE_CONFIG",
                ConfigType::PositionConfig => "POSITION_CONFIG",
                ConfigType::PowerConfig => "POWER_CONFIG",
                ConfigType::NetworkConfig => "NETWORK_CONFIG",
                ConfigType::DisplayConfig => "DISPLAY_CONFIG",
                ConfigType::LoraConfig => "LORA_CONFIG",
                ConfigType::BluetoothConfig => "BLUETOOTH_CONFIG",
            }
        }
        /// Creates an enum from field names used in the ProtoBuf definition.
        pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
            match value {
                "DEVICE_CONFIG" => Some(Self::DeviceConfig),
                "POSITION_CONFIG" => Some(Self::PositionConfig),
                "POWER_CONFIG" => Some(Self::PowerConfig),
                "NETWORK_CONFIG" => Some(Self::NetworkConfig),
                "DISPLAY_CONFIG" => Some(Self::DisplayConfig),
                "LORA_CONFIG" => Some(Self::LoraConfig),
                "BLUETOOTH_CONFIG" => Some(Self::BluetoothConfig),
                _ => None,
            }
        }
    }
    ///
    /// TODO: REPLACE
    #[derive(serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    #[allow(clippy::doc_lazy_continuation)]
    #[derive(
        Clone,
        Copy,
        Debug,
        PartialEq,
        Eq,
        Hash,
        PartialOrd,
        Ord,
        ::prost::Enumeration
    )]
    #[repr(i32)]
    pub enum ModuleConfigType {
        ///
        /// TODO: REPLACE
        MqttConfig = 0,
        ///
        /// TODO: REPLACE
        SerialConfig = 1,
        ///
        /// TODO: REPLACE
        ExtnotifConfig = 2,
        ///
        /// TODO: REPLACE
        StoreforwardConfig = 3,
        ///
        /// TODO: REPLACE
        RangetestConfig = 4,
        ///
        /// TODO: REPLACE
        TelemetryConfig = 5,
        ///
        /// TODO: REPLACE
        CannedmsgConfig = 6,
        ///
        /// TODO: REPLACE
        AudioConfig = 7,
        ///
        /// TODO: REPLACE
        RemotehardwareConfig = 8,
        ///
        /// TODO: REPLACE
        NeighborinfoConfig = 9,
        ///
        /// TODO: REPLACE
        AmbientlightingConfig = 10,
        ///
        /// TODO: REPLACE
        DetectionsensorConfig = 11,
        ///
        /// TODO: REPLACE
        PaxcounterConfig = 12,
    }
    impl ModuleConfigType {
        /// String value of the enum field names used in the ProtoBuf definition.
        ///
        /// The values are not transformed in any way and thus are considered stable
        /// (if the ProtoBuf definition does not change) and safe for programmatic use.
        pub fn as_str_name(&self) -> &'static str {
            match self {
                ModuleConfigType::MqttConfig => "MQTT_CONFIG",
                ModuleConfigType::SerialConfig => "SERIAL_CONFIG",
                ModuleConfigType::ExtnotifConfig => "EXTNOTIF_CONFIG",
                ModuleConfigType::StoreforwardConfig => "STOREFORWARD_CONFIG",
                ModuleConfigType::RangetestConfig => "RANGETEST_CONFIG",
                ModuleConfigType::TelemetryConfig => "TELEMETRY_CONFIG",
                ModuleConfigType::CannedmsgConfig => "CANNEDMSG_CONFIG",
                ModuleConfigType::AudioConfig => "AUDIO_CONFIG",
                ModuleConfigType::RemotehardwareConfig => "REMOTEHARDWARE_CONFIG",
                ModuleConfigType::NeighborinfoConfig => "NEIGHBORINFO_CONFIG",
                ModuleConfigType::AmbientlightingConfig => "AMBIENTLIGHTING_CONFIG",
                ModuleConfigType::DetectionsensorConfig => "DETECTIONSENSOR_CONFIG",
                ModuleConfigType::PaxcounterConfig => "PAXCOUNTER_CONFIG",
            }
        }
        /// Creates an enum from field names used in the ProtoBuf definition.
        pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
            match value {
                "MQTT_CONFIG" => Some(Self::MqttConfig),
                "SERIAL_CONFIG" => Some(Self::SerialConfig),
                "EXTNOTIF_CONFIG" => Some(Self::ExtnotifConfig),
                "STOREFORWARD_CONFIG" => Some(Self::StoreforwardConfig),
                "RANGETEST_CONFIG" => Some(Self::RangetestConfig),
                "TELEMETRY_CONFIG" => Some(Self::TelemetryConfig),
                "CANNEDMSG_CONFIG" => Some(Self::CannedmsgConfig),
                "AUDIO_CONFIG" => Some(Self::AudioConfig),
                "REMOTEHARDWARE_CONFIG" => Some(Self::RemotehardwareConfig),
                "NEIGHBORINFO_CONFIG" => Some(Self::NeighborinfoConfig),
                "AMBIENTLIGHTING_CONFIG" => Some(Self::AmbientlightingConfig),
                "DETECTIONSENSOR_CONFIG" => Some(Self::DetectionsensorConfig),
                "PAXCOUNTER_CONFIG" => Some(Self::PaxcounterConfig),
                _ => None,
            }
        }
    }
    ///
    /// TODO: REPLACE
    #[derive(serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    #[allow(clippy::doc_lazy_continuation)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum PayloadVariant {
        ///
        /// Send the specified channel in the response to this message
        /// NOTE: This field is sent with the channel index + 1 (to ensure we never try to send 'zero' - which protobufs treats as not present)
        #[prost(uint32, tag = "1")]
        GetChannelRequest(u32),
        ///
        /// TODO: REPLACE
        #[prost(message, tag = "2")]
        GetChannelResponse(super::Channel),
        ///
        /// Send the current owner data in the response to this message.
        #[prost(bool, tag = "3")]
        GetOwnerRequest(bool),
        ///
        /// TODO: REPLACE
        #[prost(message, tag = "4")]
        GetOwnerResponse(super::User),
        ///
        /// Ask for the following config data to be sent
        #[prost(enumeration = "ConfigType", tag = "5")]
        GetConfigRequest(i32),
        ///
        /// Send the current Config in the response to this message.
        #[prost(message, tag = "6")]
        GetConfigResponse(super::Config),
        ///
        /// Ask for the following config data to be sent
        #[prost(enumeration = "ModuleConfigType", tag = "7")]
        GetModuleConfigRequest(i32),
        ///
        /// Send the current Config in the response to this message.
        #[prost(message, tag = "8")]
        GetModuleConfigResponse(super::ModuleConfig),
        ///
        /// Get the Canned Message Module messages in the response to this message.
        #[prost(bool, tag = "10")]
        GetCannedMessageModuleMessagesRequest(bool),
        ///
        /// Get the Canned Message Module messages in the response to this message.
        #[prost(string, tag = "11")]
        GetCannedMessageModuleMessagesResponse(::prost::alloc::string::String),
        ///
        /// Request the node to send device metadata (firmware, protobuf version, etc)
        #[prost(bool, tag = "12")]
        GetDeviceMetadataRequest(bool),
        ///
        /// Device metadata response
        #[prost(message, tag = "13")]
        GetDeviceMetadataResponse(super::DeviceMetadata),
        ///
        /// Get the Ringtone in the response to this message.
        #[prost(bool, tag = "14")]
        GetRingtoneRequest(bool),
        ///
        /// Get the Ringtone in the response to this message.
        #[prost(string, tag = "15")]
        GetRingtoneResponse(::prost::alloc::string::String),
        ///
        /// Request the node to send it's connection status
        #[prost(bool, tag = "16")]
        GetDeviceConnectionStatusRequest(bool),
        ///
        /// Device connection status response
        #[prost(message, tag = "17")]
        GetDeviceConnectionStatusResponse(super::DeviceConnectionStatus),
        ///
        /// Setup a node for licensed amateur (ham) radio operation
        #[prost(message, tag = "18")]
        SetHamMode(super::HamParameters),
        ///
        /// Get the mesh's nodes with their available gpio pins for RemoteHardware module use
        #[prost(bool, tag = "19")]
        GetNodeRemoteHardwarePinsRequest(bool),
        ///
        /// Respond with the mesh's nodes with their available gpio pins for RemoteHardware module use
        #[prost(message, tag = "20")]
        GetNodeRemoteHardwarePinsResponse(super::NodeRemoteHardwarePinsResponse),
        ///
        /// Enter (UF2) DFU mode
        /// Only implemented on NRF52 currently
        #[prost(bool, tag = "21")]
        EnterDfuModeRequest(bool),
        ///
        /// Delete the file by the specified path from the device
        #[prost(string, tag = "22")]
        DeleteFileRequest(::prost::alloc::string::String),
        ///
        /// Set the owner for this node
        #[prost(message, tag = "32")]
        SetOwner(super::User),
        ///
        /// Set channels (using the new API).
        /// A special channel is the "primary channel".
        /// The other records are secondary channels.
        /// Note: only one channel can be marked as primary.
        /// If the client sets a particular channel to be primary, the previous channel will be set to SECONDARY automatically.
        #[prost(message, tag = "33")]
        SetChannel(super::Channel),
        ///
        /// Set the current Config
        #[prost(message, tag = "34")]
        SetConfig(super::Config),
        ///
        /// Set the current Config
        #[prost(message, tag = "35")]
        SetModuleConfig(super::ModuleConfig),
        ///
        /// Set the Canned Message Module messages text.
        #[prost(string, tag = "36")]
        SetCannedMessageModuleMessages(::prost::alloc::string::String),
        ///
        /// Set the ringtone for ExternalNotification.
        #[prost(string, tag = "37")]
        SetRingtoneMessage(::prost::alloc::string::String),
        ///
        /// Remove the node by the specified node-num from the NodeDB on the device
        #[prost(uint32, tag = "38")]
        RemoveByNodenum(u32),
        ///
        /// Set specified node-num to be favorited on the NodeDB on the device
        #[prost(uint32, tag = "39")]
        SetFavoriteNode(u32),
        ///
        /// Set specified node-num to be un-favorited on the NodeDB on the device
        #[prost(uint32, tag = "40")]
        RemoveFavoriteNode(u32),
        ///
        /// Set fixed position data on the node and then set the position.fixed_position = true
        #[prost(message, tag = "41")]
        SetFixedPosition(super::Position),
        ///
        /// Clear fixed position coordinates and then set position.fixed_position = false
        #[prost(bool, tag = "42")]
        RemoveFixedPosition(bool),
        ///
        /// Begins an edit transaction for config, module config, owner, and channel settings changes
        /// This will delay the standard *implicit* save to the file system and subsequent reboot behavior until committed (commit_edit_settings)
        #[prost(bool, tag = "64")]
        BeginEditSettings(bool),
        ///
        /// Commits an open transaction for any edits made to config, module config, owner, and channel settings
        #[prost(bool, tag = "65")]
        CommitEditSettings(bool),
        ///
        /// Tell the node to reboot into the OTA Firmware in this many seconds (or <0 to cancel reboot)
        /// Only Implemented for ESP32 Devices. This needs to be issued to send a new main firmware via bluetooth.
        #[prost(int32, tag = "95")]
        RebootOtaSeconds(i32),
        ///
        /// This message is only supported for the simulator Portduino build.
        /// If received the simulator will exit successfully.
        #[prost(bool, tag = "96")]
        ExitSimulator(bool),
        ///
        /// Tell the node to reboot in this many seconds (or <0 to cancel reboot)
        #[prost(int32, tag = "97")]
        RebootSeconds(i32),
        ///
        /// Tell the node to shutdown in this many seconds (or <0 to cancel shutdown)
        #[prost(int32, tag = "98")]
        ShutdownSeconds(i32),
        ///
        /// Tell the node to factory reset, all device settings will be returned to factory defaults.
        #[prost(int32, tag = "99")]
        FactoryReset(i32),
        ///
        /// Tell the node to reset the nodedb.
        #[prost(int32, tag = "100")]
        NodedbReset(i32),
    }
}
///
/// Parameters for setting up Meshtastic for ameteur radio usage
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::doc_lazy_continuation)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct HamParameters {
    ///
    /// Amateur radio call sign, eg. KD2ABC
    #[prost(string, tag = "1")]
    pub call_sign: ::prost::alloc::string::String,
    ///
    /// Transmit power in dBm at the LoRA transceiver, not including any amplification
    #[prost(int32, tag = "2")]
    pub tx_power: i32,
    ///
    /// The selected frequency of LoRA operation
    /// Please respect your local laws, regulations, and band plans.
    /// Ensure your radio is capable of operating of the selected frequency before setting this.
    #[prost(float, tag = "3")]
    pub frequency: f32,
    ///
    /// Optional short name of user
    #[prost(string, tag = "4")]
    pub short_name: ::prost::alloc::string::String,
}
///
/// Response envelope for node_remote_hardware_pins
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::doc_lazy_continuation)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct NodeRemoteHardwarePinsResponse {
    ///
    /// Nodes and their respective remote hardware GPIO pins
    #[prost(message, repeated, tag = "1")]
    pub node_remote_hardware_pins: ::prost::alloc::vec::Vec<NodeRemoteHardwarePin>,
}
///
/// This is the most compact possible representation for a set of channels.
/// It includes only one PRIMARY channel (which must be first) and
/// any SECONDARY channels.
/// No DISABLED channels are included.
/// This abstraction is used only on the the 'app side' of the world (ie python, javascript and android etc) to show a group of Channels as a (long) URL
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::doc_lazy_continuation)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ChannelSet {
    ///
    /// Channel list with settings
    #[prost(message, repeated, tag = "1")]
    pub settings: ::prost::alloc::vec::Vec<ChannelSettings>,
    ///
    /// LoRa config
    #[prost(message, optional, tag = "2")]
    pub lora_config: ::core::option::Option<config::LoRaConfig>,
}
///
/// Packets for the official ATAK Plugin
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::doc_lazy_continuation)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct TakPacket {
    ///
    /// Are the payloads strings compressed for LoRA transport?
    #[prost(bool, tag = "1")]
    pub is_compressed: bool,
    ///
    /// The contact / callsign for ATAK user
    #[prost(message, optional, tag = "2")]
    pub contact: ::core::option::Option<Contact>,
    ///
    /// The group for ATAK user
    #[prost(message, optional, tag = "3")]
    pub group: ::core::option::Option<Group>,
    ///
    /// The status of the ATAK EUD
    #[prost(message, optional, tag = "4")]
    pub status: ::core::option::Option<Status>,
    ///
    /// The payload of the packet
    #[prost(oneof = "tak_packet::PayloadVariant", tags = "5, 6")]
    pub payload_variant: ::core::option::Option<tak_packet::PayloadVariant>,
}
/// Nested message and enum types in `TAKPacket`.
pub mod tak_packet {
    ///
    /// The payload of the packet
    #[derive(serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    #[allow(clippy::doc_lazy_continuation)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum PayloadVariant {
        ///
        /// TAK position report
        #[prost(message, tag = "5")]
        Pli(super::Pli),
        ///
        /// ATAK GeoChat message
        #[prost(message, tag = "6")]
        Chat(super::GeoChat),
    }
}
///
/// ATAK GeoChat message
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::doc_lazy_continuation)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GeoChat {
    ///
    /// The text message
    #[prost(string, tag = "1")]
    pub message: ::prost::alloc::string::String,
    ///
    /// Uid recipient of the message
    #[prost(string, optional, tag = "2")]
    pub to: ::core::option::Option<::prost::alloc::string::String>,
}
///
/// ATAK Group
/// <__group role='Team Member' name='Cyan'/>
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::doc_lazy_continuation)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Group {
    ///
    /// Role of the group member
    #[prost(enumeration = "MemberRole", tag = "1")]
    pub role: i32,
    ///
    /// Team (color)
    /// Default Cyan
    #[prost(enumeration = "Team", tag = "2")]
    pub team: i32,
}
///
/// ATAK EUD Status
/// <status battery='100' />
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::doc_lazy_continuation)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Status {
    ///
    /// Battery level
    #[prost(uint32, tag = "1")]
    pub battery: u32,
}
///
/// ATAK Contact
/// <contact endpoint='0.0.0.0:4242:tcp' phone='+12345678' callsign='FALKE'/>
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::doc_lazy_continuation)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Contact {
    ///
    /// Callsign
    #[prost(string, tag = "1")]
    pub callsign: ::prost::alloc::string::String,
    ///
    /// Device callsign
    ///
    ///
    /// IP address of endpoint in integer form (0.0.0.0 default)
    #[prost(string, tag = "2")]
    pub device_callsign: ::prost::alloc::string::String,
}
///
/// Position Location Information from ATAK
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::doc_lazy_continuation)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Pli {
    ///
    /// The new preferred location encoding, multiply by 1e-7 to get degrees
    /// in floating point
    #[prost(sfixed32, tag = "1")]
    pub latitude_i: i32,
    ///
    /// The new preferred location encoding, multiply by 1e-7 to get degrees
    /// in floating point
    #[prost(sfixed32, tag = "2")]
    pub longitude_i: i32,
    ///
    /// Altitude (ATAK prefers HAE)
    #[prost(int32, tag = "3")]
    pub altitude: i32,
    ///
    /// Speed
    #[prost(uint32, tag = "4")]
    pub speed: u32,
    ///
    /// Course in degrees
    #[prost(uint32, tag = "5")]
    pub course: u32,
}
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::doc_lazy_continuation)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum Team {
    ///
    /// Unspecifed
    UnspecifedColor = 0,
    ///
    /// White
    White = 1,
    ///
    /// Yellow
    Yellow = 2,
    ///
    /// Orange
    Orange = 3,
    ///
    /// Magenta
    Magenta = 4,
    ///
    /// Red
    Red = 5,
    ///
    /// Maroon
    Maroon = 6,
    ///
    /// Purple
    Purple = 7,
    ///
    /// Dark Blue
    DarkBlue = 8,
    ///
    /// Blue
    Blue = 9,
    ///
    /// Cyan
    Cyan = 10,
    ///
    /// Teal
    Teal = 11,
    ///
    /// Green
    Green = 12,
    ///
    /// Dark Green
    DarkGreen = 13,
    ///
    /// Brown
    Brown = 14,
}
impl Team {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            Team::UnspecifedColor => "Unspecifed_Color",
            Team::White => "White",
            Team::Yellow => "Yellow",
            Team::Orange => "Orange",
            Team::Magenta => "Magenta",
            Team::Red => "Red",
            Team::Maroon => "Maroon",
            Team::Purple => "Purple",
            Team::DarkBlue => "Dark_Blue",
            Team::Blue => "Blue",
            Team::Cyan => "Cyan",
            Team::Teal => "Teal",
            Team::Green => "Green",
            Team::DarkGreen => "Dark_Green",
            Team::Brown => "Brown",
        }
    }
    /// Creates an enum from field names used in the ProtoBuf definition.
    pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
        match value {
            "Unspecifed_Color" => Some(Self::UnspecifedColor),
            "White" => Some(Self::White),
            "Yellow" => Some(Self::Yellow),
            "Orange" => Some(Self::Orange),
            "Magenta" => Some(Self::Magenta),
            "Red" => Some(Self::Red),
            "Maroon" => Some(Self::Maroon),
            "Purple" => Some(Self::Purple),
            "Dark_Blue" => Some(Self::DarkBlue),
            "Blue" => Some(Self::Blue),
            "Cyan" => Some(Self::Cyan),
            "Teal" => Some(Self::Teal),
            "Green" => Some(Self::Green),
            "Dark_Green" => Some(Self::DarkGreen),
            "Brown" => Some(Self::Brown),
            _ => None,
        }
    }
}
///
/// Role of the group member
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::doc_lazy_continuation)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum MemberRole {
    ///
    /// Unspecifed
    Unspecifed = 0,
    ///
    /// Team Member
    TeamMember = 1,
    ///
    /// Team Lead
    TeamLead = 2,
    ///
    /// Headquarters
    Hq = 3,
    ///
    /// Airsoft enthusiast
    Sniper = 4,
    ///
    /// Medic
    Medic = 5,
    ///
    /// ForwardObserver
    ForwardObserver = 6,
    ///
    /// Radio Telephone Operator
    Rto = 7,
    ///
    /// Doggo
    K9 = 8,
}
impl MemberRole {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            MemberRole::Unspecifed => "Unspecifed",
            MemberRole::TeamMember => "TeamMember",
            MemberRole::TeamLead => "TeamLead",
            MemberRole::Hq => "HQ",
            MemberRole::Sniper => "Sniper",
            MemberRole::Medic => "Medic",
            MemberRole::ForwardObserver => "ForwardObserver",
            MemberRole::Rto => "RTO",
            MemberRole::K9 => "K9",
        }
    }
    /// Creates an enum from field names used in the ProtoBuf definition.
    pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
        match value {
            "Unspecifed" => Some(Self::Unspecifed),
            "TeamMember" => Some(Self::TeamMember),
            "TeamLead" => Some(Self::TeamLead),
            "HQ" => Some(Self::Hq),
            "Sniper" => Some(Self::Sniper),
            "Medic" => Some(Self::Medic),
            "ForwardObserver" => Some(Self::ForwardObserver),
            "RTO" => Some(Self::Rto),
            "K9" => Some(Self::K9),
            _ => None,
        }
    }
}
///
/// Canned message module configuration.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::doc_lazy_continuation)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CannedMessageModuleConfig {
    ///
    /// Predefined messages for canned message module separated by '|' characters.
    #[prost(string, tag = "1")]
    pub messages: ::prost::alloc::string::String,
}
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::doc_lazy_continuation)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct LocalConfig {
    ///
    /// The part of the config that is specific to the Device
    #[prost(message, optional, tag = "1")]
    pub device: ::core::option::Option<config::DeviceConfig>,
    ///
    /// The part of the config that is specific to the GPS Position
    #[prost(message, optional, tag = "2")]
    pub position: ::core::option::Option<config::PositionConfig>,
    ///
    /// The part of the config that is specific to the Power settings
    #[prost(message, optional, tag = "3")]
    pub power: ::core::option::Option<config::PowerConfig>,
    ///
    /// The part of the config that is specific to the Wifi Settings
    #[prost(message, optional, tag = "4")]
    pub network: ::core::option::Option<config::NetworkConfig>,
    ///
    /// The part of the config that is specific to the Display
    #[prost(message, optional, tag = "5")]
    pub display: ::core::option::Option<config::DisplayConfig>,
    ///
    /// The part of the config that is specific to the Lora Radio
    #[prost(message, optional, tag = "6")]
    pub lora: ::core::option::Option<config::LoRaConfig>,
    ///
    /// The part of the config that is specific to the Bluetooth settings
    #[prost(message, optional, tag = "7")]
    pub bluetooth: ::core::option::Option<config::BluetoothConfig>,
    ///
    /// A version integer used to invalidate old save files when we make
    /// incompatible changes This integer is set at build time and is private to
    /// NodeDB.cpp in the device code.
    #[prost(uint32, tag = "8")]
    pub version: u32,
}
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::doc_lazy_continuation)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct LocalModuleConfig {
    ///
    /// The part of the config that is specific to the MQTT module
    #[prost(message, optional, tag = "1")]
    pub mqtt: ::core::option::Option<module_config::MqttConfig>,
    ///
    /// The part of the config that is specific to the Serial module
    #[prost(message, optional, tag = "2")]
    pub serial: ::core::option::Option<module_config::SerialConfig>,
    ///
    /// The part of the config that is specific to the ExternalNotification module
    #[prost(message, optional, tag = "3")]
    pub external_notification: ::core::option::Option<
        module_config::ExternalNotificationConfig,
    >,
    ///
    /// The part of the config that is specific to the Store & Forward module
    #[prost(message, optional, tag = "4")]
    pub store_forward: ::core::option::Option<module_config::StoreForwardConfig>,
    ///
    /// The part of the config that is specific to the RangeTest module
    #[prost(message, optional, tag = "5")]
    pub range_test: ::core::option::Option<module_config::RangeTestConfig>,
    ///
    /// The part of the config that is specific to the Telemetry module
    #[prost(message, optional, tag = "6")]
    pub telemetry: ::core::option::Option<module_config::TelemetryConfig>,
    ///
    /// The part of the config that is specific to the Canned Message module
    #[prost(message, optional, tag = "7")]
    pub canned_message: ::core::option::Option<module_config::CannedMessageConfig>,
    ///
    /// The part of the config that is specific to the Audio module
    #[prost(message, optional, tag = "9")]
    pub audio: ::core::option::Option<module_config::AudioConfig>,
    ///
    /// The part of the config that is specific to the Remote Hardware module
    #[prost(message, optional, tag = "10")]
    pub remote_hardware: ::core::option::Option<module_config::RemoteHardwareConfig>,
    ///
    /// The part of the config that is specific to the Neighbor Info module
    #[prost(message, optional, tag = "11")]
    pub neighbor_info: ::core::option::Option<module_config::NeighborInfoConfig>,
    ///
    /// The part of the config that is specific to the Ambient Lighting module
    #[prost(message, optional, tag = "12")]
    pub ambient_lighting: ::core::option::Option<module_config::AmbientLightingConfig>,
    ///
    /// The part of the config that is specific to the Detection Sensor module
    #[prost(message, optional, tag = "13")]
    pub detection_sensor: ::core::option::Option<module_config::DetectionSensorConfig>,
    ///
    /// Paxcounter Config
    #[prost(message, optional, tag = "14")]
    pub paxcounter: ::core::option::Option<module_config::PaxcounterConfig>,
    ///
    /// A version integer used to invalidate old save files when we make
    /// incompatible changes This integer is set at build time and is private to
    /// NodeDB.cpp in the device code.
    #[prost(uint32, tag = "8")]
    pub version: u32,
}
///
/// This abstraction is used to contain any configuration for provisioning a node on any client.
/// It is useful for importing and exporting configurations.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::doc_lazy_continuation)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DeviceProfile {
    ///
    /// Long name for the node
    #[prost(string, optional, tag = "1")]
    pub long_name: ::core::option::Option<::prost::alloc::string::String>,
    ///
    /// Short name of the node
    #[prost(string, optional, tag = "2")]
    pub short_name: ::core::option::Option<::prost::alloc::string::String>,
    ///
    /// The url of the channels from our node
    #[prost(string, optional, tag = "3")]
    pub channel_url: ::core::option::Option<::prost::alloc::string::String>,
    ///
    /// The Config of the node
    #[prost(message, optional, tag = "4")]
    pub config: ::core::option::Option<LocalConfig>,
    ///
    /// The ModuleConfig of the node
    #[prost(message, optional, tag = "5")]
    pub module_config: ::core::option::Option<LocalModuleConfig>,
}
///
/// Position with static location information only for NodeDBLite
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::doc_lazy_continuation)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PositionLite {
    ///
    /// The new preferred location encoding, multiply by 1e-7 to get degrees
    /// in floating point
    #[prost(sfixed32, tag = "1")]
    pub latitude_i: i32,
    ///
    /// TODO: REPLACE
    #[prost(sfixed32, tag = "2")]
    pub longitude_i: i32,
    ///
    /// In meters above MSL (but see issue #359)
    #[prost(int32, tag = "3")]
    pub altitude: i32,
    ///
    /// This is usually not sent over the mesh (to save space), but it is sent
    /// from the phone so that the local device can set its RTC If it is sent over
    /// the mesh (because there are devices on the mesh without GPS), it will only
    /// be sent by devices which has a hardware GPS clock.
    /// seconds since 1970
    #[prost(fixed32, tag = "4")]
    pub time: u32,
    ///
    /// TODO: REPLACE
    #[prost(enumeration = "position::LocSource", tag = "5")]
    pub location_source: i32,
}
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::doc_lazy_continuation)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct NodeInfoLite {
    ///
    /// The node number
    #[prost(uint32, tag = "1")]
    pub num: u32,
    ///
    /// The user info for this node
    #[prost(message, optional, tag = "2")]
    pub user: ::core::option::Option<User>,
    ///
    /// This position data. Note: before 1.2.14 we would also store the last time we've heard from this node in position.time, that is no longer true.
    /// Position.time now indicates the last time we received a POSITION from that node.
    #[prost(message, optional, tag = "3")]
    pub position: ::core::option::Option<PositionLite>,
    ///
    /// Returns the Signal-to-noise ratio (SNR) of the last received message,
    /// as measured by the receiver. Return SNR of the last received message in dB
    #[prost(float, tag = "4")]
    pub snr: f32,
    ///
    /// Set to indicate the last time we received a packet from this node
    #[prost(fixed32, tag = "5")]
    pub last_heard: u32,
    ///
    /// The latest device metrics for the node.
    #[prost(message, optional, tag = "6")]
    pub device_metrics: ::core::option::Option<DeviceMetrics>,
    ///
    /// local channel index we heard that node on. Only populated if its not the default channel.
    #[prost(uint32, tag = "7")]
    pub channel: u32,
    ///
    /// True if we witnessed the node over MQTT instead of LoRA transport
    #[prost(bool, tag = "8")]
    pub via_mqtt: bool,
    ///
    /// Number of hops away from us this node is (0 if adjacent)
    #[prost(uint32, tag = "9")]
    pub hops_away: u32,
    ///
    /// True if node is in our favorites list
    /// Persists between NodeDB internal clean ups
    #[prost(bool, tag = "10")]
    pub is_favorite: bool,
}
///
/// This message is never sent over the wire, but it is used for serializing DB
/// state to flash in the device code
/// FIXME, since we write this each time we enter deep sleep (and have infinite
/// flash) it would be better to use some sort of append only data structure for
/// the receive queue and use the preferences store for the other stuff
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::doc_lazy_continuation)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DeviceState {
    ///
    /// Read only settings/info about this node
    #[prost(message, optional, tag = "2")]
    pub my_node: ::core::option::Option<MyNodeInfo>,
    ///
    /// My owner info
    #[prost(message, optional, tag = "3")]
    pub owner: ::core::option::Option<User>,
    ///
    /// Received packets saved for delivery to the phone
    #[prost(message, repeated, tag = "5")]
    pub receive_queue: ::prost::alloc::vec::Vec<MeshPacket>,
    ///
    /// A version integer used to invalidate old save files when we make
    /// incompatible changes This integer is set at build time and is private to
    /// NodeDB.cpp in the device code.
    #[prost(uint32, tag = "8")]
    pub version: u32,
    ///
    /// We keep the last received text message (only) stored in the device flash,
    /// so we can show it on the screen.
    /// Might be null
    #[prost(message, optional, tag = "7")]
    pub rx_text_message: ::core::option::Option<MeshPacket>,
    ///
    /// Used only during development.
    /// Indicates developer is testing and changes should never be saved to flash.
    /// Deprecated in 2.3.1
    #[deprecated]
    #[prost(bool, tag = "9")]
    pub no_save: bool,
    ///
    /// Some GPS receivers seem to have bogus settings from the factory, so we always do one factory reset.
    #[prost(bool, tag = "11")]
    pub did_gps_reset: bool,
    ///
    /// We keep the last received waypoint stored in the device flash,
    /// so we can show it on the screen.
    /// Might be null
    #[prost(message, optional, tag = "12")]
    pub rx_waypoint: ::core::option::Option<MeshPacket>,
    ///
    /// The mesh's nodes with their available gpio pins for RemoteHardware module
    #[prost(message, repeated, tag = "13")]
    pub node_remote_hardware_pins: ::prost::alloc::vec::Vec<NodeRemoteHardwarePin>,
    ///
    /// New lite version of NodeDB to decrease memory footprint
    #[prost(message, repeated, tag = "14")]
    pub node_db_lite: ::prost::alloc::vec::Vec<NodeInfoLite>,
}
///
/// The on-disk saved channels
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::doc_lazy_continuation)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ChannelFile {
    ///
    /// The channels our node knows about
    #[prost(message, repeated, tag = "1")]
    pub channels: ::prost::alloc::vec::Vec<Channel>,
    ///
    /// A version integer used to invalidate old save files when we make
    /// incompatible changes This integer is set at build time and is private to
    /// NodeDB.cpp in the device code.
    #[prost(uint32, tag = "2")]
    pub version: u32,
}
///
/// This can be used for customizing the firmware distribution. If populated,
/// show a secondary bootup screen with custom logo and text for 2.5 seconds.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::doc_lazy_continuation)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct OemStore {
    ///
    /// The Logo width in Px
    #[prost(uint32, tag = "1")]
    pub oem_icon_width: u32,
    ///
    /// The Logo height in Px
    #[prost(uint32, tag = "2")]
    pub oem_icon_height: u32,
    ///
    /// The Logo in XBM bytechar format
    #[prost(bytes = "vec", tag = "3")]
    pub oem_icon_bits: ::prost::alloc::vec::Vec<u8>,
    ///
    /// Use this font for the OEM text.
    #[prost(enumeration = "ScreenFonts", tag = "4")]
    pub oem_font: i32,
    ///
    /// Use this font for the OEM text.
    #[prost(string, tag = "5")]
    pub oem_text: ::prost::alloc::string::String,
    ///
    /// The default device encryption key, 16 or 32 byte
    #[prost(bytes = "vec", tag = "6")]
    pub oem_aes_key: ::prost::alloc::vec::Vec<u8>,
    ///
    /// A Preset LocalConfig to apply during factory reset
    #[prost(message, optional, tag = "7")]
    pub oem_local_config: ::core::option::Option<LocalConfig>,
    ///
    /// A Preset LocalModuleConfig to apply during factory reset
    #[prost(message, optional, tag = "8")]
    pub oem_local_module_config: ::core::option::Option<LocalModuleConfig>,
}
///
/// Font sizes for the device screen
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::doc_lazy_continuation)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum ScreenFonts {
    ///
    /// TODO: REPLACE
    FontSmall = 0,
    ///
    /// TODO: REPLACE
    FontMedium = 1,
    ///
    /// TODO: REPLACE
    FontLarge = 2,
}
impl ScreenFonts {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            ScreenFonts::FontSmall => "FONT_SMALL",
            ScreenFonts::FontMedium => "FONT_MEDIUM",
            ScreenFonts::FontLarge => "FONT_LARGE",
        }
    }
    /// Creates an enum from field names used in the ProtoBuf definition.
    pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
        match value {
            "FONT_SMALL" => Some(Self::FontSmall),
            "FONT_MEDIUM" => Some(Self::FontMedium),
            "FONT_LARGE" => Some(Self::FontLarge),
            _ => None,
        }
    }
}
///
/// This message wraps a MeshPacket with extra metadata about the sender and how it arrived.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::doc_lazy_continuation)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ServiceEnvelope {
    ///
    /// The (probably encrypted) packet
    #[prost(message, optional, tag = "1")]
    pub packet: ::core::option::Option<MeshPacket>,
    ///
    /// The global channel ID it was sent on
    #[prost(string, tag = "2")]
    pub channel_id: ::prost::alloc::string::String,
    ///
    /// The sending gateway node ID. Can we use this to authenticate/prevent fake
    /// nodeid impersonation for senders? - i.e. use gateway/mesh id (which is authenticated) + local node id as
    /// the globally trusted nodenum
    #[prost(string, tag = "3")]
    pub gateway_id: ::prost::alloc::string::String,
}
///
/// Information about a node intended to be reported unencrypted to a map using MQTT.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::doc_lazy_continuation)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MapReport {
    ///
    /// A full name for this user, i.e. "Kevin Hester"
    #[prost(string, tag = "1")]
    pub long_name: ::prost::alloc::string::String,
    ///
    /// A VERY short name, ideally two characters.
    /// Suitable for a tiny OLED screen
    #[prost(string, tag = "2")]
    pub short_name: ::prost::alloc::string::String,
    ///
    /// Role of the node that applies specific settings for a particular use-case
    #[prost(enumeration = "config::device_config::Role", tag = "3")]
    pub role: i32,
    ///
    /// Hardware model of the node, i.e. T-Beam, Heltec V3, etc...
    #[prost(enumeration = "HardwareModel", tag = "4")]
    pub hw_model: i32,
    ///
    /// Device firmware version string
    #[prost(string, tag = "5")]
    pub firmware_version: ::prost::alloc::string::String,
    ///
    /// The region code for the radio (US, CN, EU433, etc...)
    #[prost(enumeration = "config::lo_ra_config::RegionCode", tag = "6")]
    pub region: i32,
    ///
    /// Modem preset used by the radio (LongFast, MediumSlow, etc...)
    #[prost(enumeration = "config::lo_ra_config::ModemPreset", tag = "7")]
    pub modem_preset: i32,
    ///
    /// Whether the node has a channel with default PSK and name (LongFast, MediumSlow, etc...)
    /// and it uses the default frequency slot given the region and modem preset.
    #[prost(bool, tag = "8")]
    pub has_default_channel: bool,
    ///
    /// Latitude: multiply by 1e-7 to get degrees in floating point
    #[prost(sfixed32, tag = "9")]
    pub latitude_i: i32,
    ///
    /// Longitude: multiply by 1e-7 to get degrees in floating point
    #[prost(sfixed32, tag = "10")]
    pub longitude_i: i32,
    ///
    /// Altitude in meters above MSL
    #[prost(int32, tag = "11")]
    pub altitude: i32,
    ///
    /// Indicates the bits of precision for latitude and longitude set by the sending node
    #[prost(uint32, tag = "12")]
    pub position_precision: u32,
    ///
    /// Number of online nodes (heard in the last 2 hours) this node has in its list that were received locally (not via MQTT)
    #[prost(uint32, tag = "13")]
    pub num_online_local_nodes: u32,
}
///
/// TODO: REPLACE
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::doc_lazy_continuation)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Paxcount {
    ///
    /// seen Wifi devices
    #[prost(uint32, tag = "1")]
    pub wifi: u32,
    ///
    /// Seen BLE devices
    #[prost(uint32, tag = "2")]
    pub ble: u32,
    ///
    /// Uptime in seconds
    #[prost(uint32, tag = "3")]
    pub uptime: u32,
}
///
/// An example app to show off the module system. This message is used for
/// REMOTE_HARDWARE_APP PortNums.
/// Also provides easy remote access to any GPIO.
/// In the future other remote hardware operations can be added based on user interest
/// (i.e. serial output, spi/i2c input/output).
/// FIXME - currently this feature is turned on by default which is dangerous
/// because no security yet (beyond the channel mechanism).
/// It should be off by default and then protected based on some TBD mechanism
/// (a special channel once multichannel support is included?)
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::doc_lazy_continuation)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct HardwareMessage {
    ///
    /// What type of HardwareMessage is this?
    #[prost(enumeration = "hardware_message::Type", tag = "1")]
    pub r#type: i32,
    ///
    /// What gpios are we changing. Not used for all MessageTypes, see MessageType for details
    #[prost(uint64, tag = "2")]
    pub gpio_mask: u64,
    ///
    /// For gpios that were listed in gpio_mask as valid, what are the signal levels for those gpios.
    /// Not used for all MessageTypes, see MessageType for details
    #[prost(uint64, tag = "3")]
    pub gpio_value: u64,
}
/// Nested message and enum types in `HardwareMessage`.
pub mod hardware_message {
    ///
    /// TODO: REPLACE
    #[derive(serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    #[allow(clippy::doc_lazy_continuation)]
    #[derive(
        Clone,
        Copy,
        Debug,
        PartialEq,
        Eq,
        Hash,
        PartialOrd,
        Ord,
        ::prost::Enumeration
    )]
    #[repr(i32)]
    pub enum Type {
        ///
        /// Unset/unused
        Unset = 0,
        ///
        /// Set gpio gpios based on gpio_mask/gpio_value
        WriteGpios = 1,
        ///
        /// We are now interested in watching the gpio_mask gpios.
        /// If the selected gpios change, please broadcast GPIOS_CHANGED.
        /// Will implicitly change the gpios requested to be INPUT gpios.
        WatchGpios = 2,
        ///
        /// The gpios listed in gpio_mask have changed, the new values are listed in gpio_value
        GpiosChanged = 3,
        ///
        /// Read the gpios specified in gpio_mask, send back a READ_GPIOS_REPLY reply with gpio_value populated
        ReadGpios = 4,
        ///
        /// A reply to READ_GPIOS. gpio_mask and gpio_value will be populated
        ReadGpiosReply = 5,
    }
    impl Type {
        /// String value of the enum field names used in the ProtoBuf definition.
        ///
        /// The values are not transformed in any way and thus are considered stable
        /// (if the ProtoBuf definition does not change) and safe for programmatic use.
        pub fn as_str_name(&self) -> &'static str {
            match self {
                Type::Unset => "UNSET",
                Type::WriteGpios => "WRITE_GPIOS",
                Type::WatchGpios => "WATCH_GPIOS",
                Type::GpiosChanged => "GPIOS_CHANGED",
                Type::ReadGpios => "READ_GPIOS",
                Type::ReadGpiosReply => "READ_GPIOS_REPLY",
            }
        }
        /// Creates an enum from field names used in the ProtoBuf definition.
        pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
            match value {
                "UNSET" => Some(Self::Unset),
                "WRITE_GPIOS" => Some(Self::WriteGpios),
                "WATCH_GPIOS" => Some(Self::WatchGpios),
                "GPIOS_CHANGED" => Some(Self::GpiosChanged),
                "READ_GPIOS" => Some(Self::ReadGpios),
                "READ_GPIOS_REPLY" => Some(Self::ReadGpiosReply),
                _ => None,
            }
        }
    }
}
///
/// Canned message module configuration.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::doc_lazy_continuation)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RtttlConfig {
    ///
    /// Ringtone for PWM Buzzer in RTTTL Format.
    #[prost(string, tag = "1")]
    pub ringtone: ::prost::alloc::string::String,
}
///
/// TODO: REPLACE
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::doc_lazy_continuation)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct StoreAndForward {
    ///
    /// TODO: REPLACE
    #[prost(enumeration = "store_and_forward::RequestResponse", tag = "1")]
    pub rr: i32,
    ///
    /// TODO: REPLACE
    #[prost(oneof = "store_and_forward::Variant", tags = "2, 3, 4, 5")]
    pub variant: ::core::option::Option<store_and_forward::Variant>,
}
/// Nested message and enum types in `StoreAndForward`.
pub mod store_and_forward {
    ///
    /// TODO: REPLACE
    #[derive(serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    #[allow(clippy::doc_lazy_continuation)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Statistics {
        ///
        /// Number of messages we have ever seen
        #[prost(uint32, tag = "1")]
        pub messages_total: u32,
        ///
        /// Number of messages we have currently saved our history.
        #[prost(uint32, tag = "2")]
        pub messages_saved: u32,
        ///
        /// Maximum number of messages we will save
        #[prost(uint32, tag = "3")]
        pub messages_max: u32,
        ///
        /// Router uptime in seconds
        #[prost(uint32, tag = "4")]
        pub up_time: u32,
        ///
        /// Number of times any client sent a request to the S&F.
        #[prost(uint32, tag = "5")]
        pub requests: u32,
        ///
        /// Number of times the history was requested.
        #[prost(uint32, tag = "6")]
        pub requests_history: u32,
        ///
        /// Is the heartbeat enabled on the server?
        #[prost(bool, tag = "7")]
        pub heartbeat: bool,
        ///
        /// Maximum number of messages the server will return.
        #[prost(uint32, tag = "8")]
        pub return_max: u32,
        ///
        /// Maximum history window in minutes the server will return messages from.
        #[prost(uint32, tag = "9")]
        pub return_window: u32,
    }
    ///
    /// TODO: REPLACE
    #[derive(serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    #[allow(clippy::doc_lazy_continuation)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct History {
        ///
        /// Number of that will be sent to the client
        #[prost(uint32, tag = "1")]
        pub history_messages: u32,
        ///
        /// The window of messages that was used to filter the history client requested
        #[prost(uint32, tag = "2")]
        pub window: u32,
        ///
        /// Index in the packet history of the last message sent in a previous request to the server.
        /// Will be sent to the client before sending the history and can be set in a subsequent request to avoid getting packets the server already sent to the client.
        #[prost(uint32, tag = "3")]
        pub last_request: u32,
    }
    ///
    /// TODO: REPLACE
    #[derive(serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    #[allow(clippy::doc_lazy_continuation)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Heartbeat {
        ///
        /// Period in seconds that the heartbeat is sent out that will be sent to the client
        #[prost(uint32, tag = "1")]
        pub period: u32,
        ///
        /// If set, this is not the primary Store & Forward router on the mesh
        #[prost(uint32, tag = "2")]
        pub secondary: u32,
    }
    ///
    /// 001 - 063 = From Router
    /// 064 - 127 = From Client
    #[derive(serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    #[allow(clippy::doc_lazy_continuation)]
    #[derive(
        Clone,
        Copy,
        Debug,
        PartialEq,
        Eq,
        Hash,
        PartialOrd,
        Ord,
        ::prost::Enumeration
    )]
    #[repr(i32)]
    pub enum RequestResponse {
        ///
        /// Unset/unused
        Unset = 0,
        ///
        /// Router is an in error state.
        RouterError = 1,
        ///
        /// Router heartbeat
        RouterHeartbeat = 2,
        ///
        /// Router has requested the client respond. This can work as a
        /// "are you there" message.
        RouterPing = 3,
        ///
        /// The response to a "Ping"
        RouterPong = 4,
        ///
        /// Router is currently busy. Please try again later.
        RouterBusy = 5,
        ///
        /// Router is responding to a request for history.
        RouterHistory = 6,
        ///
        /// Router is responding to a request for stats.
        RouterStats = 7,
        ///
        /// Router sends a text message from its history that was a direct message.
        RouterTextDirect = 8,
        ///
        /// Router sends a text message from its history that was a broadcast.
        RouterTextBroadcast = 9,
        ///
        /// Client is an in error state.
        ClientError = 64,
        ///
        /// Client has requested a replay from the router.
        ClientHistory = 65,
        ///
        /// Client has requested stats from the router.
        ClientStats = 66,
        ///
        /// Client has requested the router respond. This can work as a
        /// "are you there" message.
        ClientPing = 67,
        ///
        /// The response to a "Ping"
        ClientPong = 68,
        ///
        /// Client has requested that the router abort processing the client's request
        ClientAbort = 106,
    }
    impl RequestResponse {
        /// String value of the enum field names used in the ProtoBuf definition.
        ///
        /// The values are not transformed in any way and thus are considered stable
        /// (if the ProtoBuf definition does not change) and safe for programmatic use.
        pub fn as_str_name(&self) -> &'static str {
            match self {
                RequestResponse::Unset => "UNSET",
                RequestResponse::RouterError => "ROUTER_ERROR",
                RequestResponse::RouterHeartbeat => "ROUTER_HEARTBEAT",
                RequestResponse::RouterPing => "ROUTER_PING",
                RequestResponse::RouterPong => "ROUTER_PONG",
                RequestResponse::RouterBusy => "ROUTER_BUSY",
                RequestResponse::RouterHistory => "ROUTER_HISTORY",
                RequestResponse::RouterStats => "ROUTER_STATS",
                RequestResponse::RouterTextDirect => "ROUTER_TEXT_DIRECT",
                RequestResponse::RouterTextBroadcast => "ROUTER_TEXT_BROADCAST",
                RequestResponse::ClientError => "CLIENT_ERROR",
                RequestResponse::ClientHistory => "CLIENT_HISTORY",
                RequestResponse::ClientStats => "CLIENT_STATS",
                RequestResponse::ClientPing => "CLIENT_PING",
                RequestResponse::ClientPong => "CLIENT_PONG",
                RequestResponse::ClientAbort => "CLIENT_ABORT",
            }
        }
        /// Creates an enum from field names used in the ProtoBuf definition.
        pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
            match value {
                "UNSET" => Some(Self::Unset),
                "ROUTER_ERROR" => Some(Self::RouterError),
                "ROUTER_HEARTBEAT" => Some(Self::RouterHeartbeat),
                "ROUTER_PING" => Some(Self::RouterPing),
                "ROUTER_PONG" => Some(Self::RouterPong),
                "ROUTER_BUSY" => Some(Self::RouterBusy),
                "ROUTER_HISTORY" => Some(Self::RouterHistory),
                "ROUTER_STATS" => Some(Self::RouterStats),
                "ROUTER_TEXT_DIRECT" => Some(Self::RouterTextDirect),
                "ROUTER_TEXT_BROADCAST" => Some(Self::RouterTextBroadcast),
                "CLIENT_ERROR" => Some(Self::ClientError),
                "CLIENT_HISTORY" => Some(Self::ClientHistory),
                "CLIENT_STATS" => Some(Self::ClientStats),
                "CLIENT_PING" => Some(Self::ClientPing),
                "CLIENT_PONG" => Some(Self::ClientPong),
                "CLIENT_ABORT" => Some(Self::ClientAbort),
                _ => None,
            }
        }
    }
    ///
    /// TODO: REPLACE
    #[derive(serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    #[allow(clippy::doc_lazy_continuation)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Variant {
        ///
        /// TODO: REPLACE
        #[prost(message, tag = "2")]
        Stats(Statistics),
        ///
        /// TODO: REPLACE
        #[prost(message, tag = "3")]
        History(History),
        ///
        /// TODO: REPLACE
        #[prost(message, tag = "4")]
        Heartbeat(Heartbeat),
        ///
        /// Text from history message.
        #[prost(bytes, tag = "5")]
        Text(::prost::alloc::vec::Vec<u8>),
    }
}
