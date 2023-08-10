use ethers::types::{Bytes, Selector};
use serde::Deserialize;
use serde_with::DeserializeAs;

/// Helper for deserializing a function selector from the hex string representation into a [`Selector`].
pub struct SelectorDeserializer(Selector);

impl<'de> DeserializeAs<'de, Selector> for SelectorDeserializer {
    fn deserialize_as<D>(deserializer: D) -> Result<Selector, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let bytes = Bytes::deserialize(deserializer)?;
        bytes.to_vec().try_into().map_err(|_| {
            serde::de::Error::invalid_value(serde::de::Unexpected::Bytes(&bytes), &"0x01234567")
        })
    }
}
