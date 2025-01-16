/// This is the inner options message, which basically defines options for
/// a field. When it is used in message or file scope, it applies to all
/// fields.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::doc_lazy_continuation)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct NanoPbOptions {
    /// Allocated size for 'bytes' and 'string' fields.
    /// For string fields, this should include the space for null terminator.
    #[prost(int32, optional, tag = "1")]
    pub max_size: ::core::option::Option<i32>,
    /// Maximum length for 'string' fields. Setting this is equivalent
    /// to setting max_size to a value of length+1.
    #[prost(int32, optional, tag = "14")]
    pub max_length: ::core::option::Option<i32>,
    /// Allocated number of entries in arrays ('repeated' fields)
    #[prost(int32, optional, tag = "2")]
    pub max_count: ::core::option::Option<i32>,
    /// Size of integer fields. Can save some memory if you don't need
    /// full 32 bits for the value.
    #[prost(enumeration = "IntSize", optional, tag = "7", default = "IsDefault")]
    pub int_size: ::core::option::Option<i32>,
    /// Force type of field (callback or static allocation)
    #[prost(enumeration = "FieldType", optional, tag = "3", default = "FtDefault")]
    pub r#type: ::core::option::Option<i32>,
    /// Use long names for enums, i.e. EnumName_EnumValue.
    #[prost(bool, optional, tag = "4", default = "true")]
    pub long_names: ::core::option::Option<bool>,
    /// Add 'packed' attribute to generated structs.
    /// Note: this cannot be used on CPUs that break on unaligned
    /// accesses to variables.
    #[prost(bool, optional, tag = "5", default = "false")]
    pub packed_struct: ::core::option::Option<bool>,
    /// Add 'packed' attribute to generated enums.
    #[prost(bool, optional, tag = "10", default = "false")]
    pub packed_enum: ::core::option::Option<bool>,
    /// Skip this message
    #[prost(bool, optional, tag = "6", default = "false")]
    pub skip_message: ::core::option::Option<bool>,
    /// Generate oneof fields as normal optional fields instead of union.
    #[prost(bool, optional, tag = "8", default = "false")]
    pub no_unions: ::core::option::Option<bool>,
    /// integer type tag for a message
    #[prost(uint32, optional, tag = "9")]
    pub msgid: ::core::option::Option<u32>,
    /// decode oneof as anonymous union
    #[prost(bool, optional, tag = "11", default = "false")]
    pub anonymous_oneof: ::core::option::Option<bool>,
    /// Proto3 singular field does not generate a "has_" flag
    #[prost(bool, optional, tag = "12", default = "false")]
    pub proto3: ::core::option::Option<bool>,
    /// Force proto3 messages to have no "has_" flag.
    /// This was default behavior until nanopb-0.4.0.
    #[prost(bool, optional, tag = "21", default = "false")]
    pub proto3_singular_msgs: ::core::option::Option<bool>,
    /// Generate an enum->string mapping function (can take up lots of space).
    #[prost(bool, optional, tag = "13", default = "false")]
    pub enum_to_string: ::core::option::Option<bool>,
    /// Generate bytes arrays with fixed length
    #[prost(bool, optional, tag = "15", default = "false")]
    pub fixed_length: ::core::option::Option<bool>,
    /// Generate repeated field with fixed count
    #[prost(bool, optional, tag = "16", default = "false")]
    pub fixed_count: ::core::option::Option<bool>,
    /// Generate message-level callback that is called before decoding submessages.
    /// This can be used to set callback fields for submsgs inside oneofs.
    #[prost(bool, optional, tag = "22", default = "false")]
    pub submsg_callback: ::core::option::Option<bool>,
    /// Shorten or remove package names from type names.
    /// This option applies only on the file level.
    #[prost(enumeration = "TypenameMangling", optional, tag = "17", default = "MNone")]
    pub mangle_names: ::core::option::Option<i32>,
    /// Data type for storage associated with callback fields.
    #[prost(string, optional, tag = "18", default = "pb_callback_t")]
    pub callback_datatype: ::core::option::Option<::prost::alloc::string::String>,
    /// Callback function used for encoding and decoding.
    /// Prior to nanopb-0.4.0, the callback was specified in per-field pb_callback_t
    /// structure. This is still supported, but does not work inside e.g. oneof or pointer
    /// fields. Instead, a new method allows specifying a per-message callback that
    /// will be called for all callback fields in a message type.
    #[prost(string, optional, tag = "19", default = "pb_default_field_callback")]
    pub callback_function: ::core::option::Option<::prost::alloc::string::String>,
    /// Select the size of field descriptors. This option has to be defined
    /// for the whole message, not per-field. Usually automatic selection is
    /// ok, but if it results in compilation errors you can increase the field
    /// size here.
    #[prost(enumeration = "DescriptorSize", optional, tag = "20", default = "DsAuto")]
    pub descriptorsize: ::core::option::Option<i32>,
    /// Set default value for has_ fields.
    #[prost(bool, optional, tag = "23", default = "false")]
    pub default_has: ::core::option::Option<bool>,
    /// Extra files to include in generated `.pb.h`
    #[prost(string, repeated, tag = "24")]
    pub include: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
    /// Automatic includes to exclude from generated `.pb.h`
    /// Same as nanopb_generator.py command line flag -x.
    #[prost(string, repeated, tag = "26")]
    pub exclude: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
    /// Package name that applies only for nanopb.
    #[prost(string, optional, tag = "25")]
    pub package: ::core::option::Option<::prost::alloc::string::String>,
    /// Override type of the field in generated C code. Only to be used with related field types
    #[prost(
        enumeration = "::prost_types::field_descriptor_proto::Type",
        optional,
        tag = "27"
    )]
    pub type_override: ::core::option::Option<i32>,
    /// Due to historical reasons, nanopb orders fields in structs by their tag number
    /// instead of the order in .proto. Set this to false to keep the .proto order.
    /// The default value will probably change to false in nanopb-0.5.0.
    #[prost(bool, optional, tag = "28", default = "true")]
    pub sort_by_tag: ::core::option::Option<bool>,
    /// Set the FT_DEFAULT field conversion strategy.
    /// A field that can become a static member of a c struct (e.g. int, bool, etc)
    /// will be a a static field.
    /// Fields with dynamic length are converted to either a pointer or a callback.
    #[prost(enumeration = "FieldType", optional, tag = "29", default = "FtCallback")]
    pub fallback_type: ::core::option::Option<i32>,
}
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::doc_lazy_continuation)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum FieldType {
    /// Automatically decide field type, generate static field if possible.
    FtDefault = 0,
    /// Always generate a callback field.
    FtCallback = 1,
    /// Always generate a dynamically allocated field.
    FtPointer = 4,
    /// Generate a static field or raise an exception if not possible.
    FtStatic = 2,
    /// Ignore the field completely.
    FtIgnore = 3,
    /// Legacy option, use the separate 'fixed_length' option instead
    FtInline = 5,
}
impl FieldType {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            FieldType::FtDefault => "FT_DEFAULT",
            FieldType::FtCallback => "FT_CALLBACK",
            FieldType::FtPointer => "FT_POINTER",
            FieldType::FtStatic => "FT_STATIC",
            FieldType::FtIgnore => "FT_IGNORE",
            FieldType::FtInline => "FT_INLINE",
        }
    }
    /// Creates an enum from field names used in the ProtoBuf definition.
    pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
        match value {
            "FT_DEFAULT" => Some(Self::FtDefault),
            "FT_CALLBACK" => Some(Self::FtCallback),
            "FT_POINTER" => Some(Self::FtPointer),
            "FT_STATIC" => Some(Self::FtStatic),
            "FT_IGNORE" => Some(Self::FtIgnore),
            "FT_INLINE" => Some(Self::FtInline),
            _ => None,
        }
    }
}
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::doc_lazy_continuation)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum IntSize {
    /// Default, 32/64bit based on type in .proto
    IsDefault = 0,
    Is8 = 8,
    Is16 = 16,
    Is32 = 32,
    Is64 = 64,
}
impl IntSize {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            IntSize::IsDefault => "IS_DEFAULT",
            IntSize::Is8 => "IS_8",
            IntSize::Is16 => "IS_16",
            IntSize::Is32 => "IS_32",
            IntSize::Is64 => "IS_64",
        }
    }
    /// Creates an enum from field names used in the ProtoBuf definition.
    pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
        match value {
            "IS_DEFAULT" => Some(Self::IsDefault),
            "IS_8" => Some(Self::Is8),
            "IS_16" => Some(Self::Is16),
            "IS_32" => Some(Self::Is32),
            "IS_64" => Some(Self::Is64),
            _ => None,
        }
    }
}
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::doc_lazy_continuation)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum TypenameMangling {
    /// Default, no typename mangling
    MNone = 0,
    /// Strip current package name
    MStripPackage = 1,
    /// Only use last path component
    MFlatten = 2,
    /// Replace the package name by the initials
    MPackageInitials = 3,
}
impl TypenameMangling {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            TypenameMangling::MNone => "M_NONE",
            TypenameMangling::MStripPackage => "M_STRIP_PACKAGE",
            TypenameMangling::MFlatten => "M_FLATTEN",
            TypenameMangling::MPackageInitials => "M_PACKAGE_INITIALS",
        }
    }
    /// Creates an enum from field names used in the ProtoBuf definition.
    pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
        match value {
            "M_NONE" => Some(Self::MNone),
            "M_STRIP_PACKAGE" => Some(Self::MStripPackage),
            "M_FLATTEN" => Some(Self::MFlatten),
            "M_PACKAGE_INITIALS" => Some(Self::MPackageInitials),
            _ => None,
        }
    }
}
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::doc_lazy_continuation)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum DescriptorSize {
    /// Select minimal size based on field type
    DsAuto = 0,
    /// 1 word; up to 15 byte fields, no arrays
    Ds1 = 1,
    /// 2 words; up to 4095 byte fields, 4095 entry arrays
    Ds2 = 2,
    /// 4 words; up to 2^32-1 byte fields, 2^16-1 entry arrays
    Ds4 = 4,
    /// 8 words; up to 2^32-1 entry arrays
    Ds8 = 8,
}
impl DescriptorSize {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            DescriptorSize::DsAuto => "DS_AUTO",
            DescriptorSize::Ds1 => "DS_1",
            DescriptorSize::Ds2 => "DS_2",
            DescriptorSize::Ds4 => "DS_4",
            DescriptorSize::Ds8 => "DS_8",
        }
    }
    /// Creates an enum from field names used in the ProtoBuf definition.
    pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
        match value {
            "DS_AUTO" => Some(Self::DsAuto),
            "DS_1" => Some(Self::Ds1),
            "DS_2" => Some(Self::Ds2),
            "DS_4" => Some(Self::Ds4),
            "DS_8" => Some(Self::Ds8),
            _ => None,
        }
    }
}
