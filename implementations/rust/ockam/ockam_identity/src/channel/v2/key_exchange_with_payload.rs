use crate::channel::decryptor::Decryptor;
use crate::channel::encryptor::Encryptor;
use core::fmt::Formatter;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::KeyExchanger;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Clone)]
pub(crate) struct KeyExchangeWithPayload<T> {
    payload: Vec<u8>,
    _phantom: core::marker::PhantomData<T>,
}

impl<'de, T> KeyExchangeWithPayload<T>
where
    T: Serialize + DeserializeOwned + Send + Sync + 'static,
{
    pub async fn create(
        item: T,
        key_exchanger: &mut Box<dyn KeyExchanger>,
    ) -> ockam_core::Result<Self> {
        Ok(Self {
            payload: key_exchanger
                .generate_request(&serde_bare::to_vec(&item)?)
                .await?,
            _phantom: Default::default(),
        })
    }

    pub async fn handle_and_decrypt(
        &self,
        key_exchanger: &mut Box<dyn KeyExchanger>,
    ) -> ockam_core::Result<T> {
        let payload = key_exchanger.handle_response(&self.payload).await?;
        let result = serde_bare::from_slice::<T>(&payload);
        let item = result
            .map_err(|error| ockam_core::Error::new(Origin::Channel, Kind::Invalid, error))?;

        Ok(item)
    }
}

impl<T> Serialize for KeyExchangeWithPayload<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bytes(&self.payload)
    }
}

impl<'de, T> Deserialize<'de> for KeyExchangeWithPayload<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor<T> {
            _phantom: core::marker::PhantomData<T>,
        }

        impl<'de, T> serde::de::Visitor<'de> for Visitor<T> {
            type Value = KeyExchangeWithPayload<T>;

            fn expecting(&self, formatter: &mut Formatter) -> core::fmt::Result {
                formatter.write_str("encrypted bytes")
            }

            fn visit_bytes<E>(self, value: &[u8]) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(KeyExchangeWithPayload {
                    payload: value.to_vec(),
                    _phantom: Default::default(),
                })
            }
        }

        deserializer.deserialize_bytes(Visitor {
            _phantom: Default::default(),
        })
    }
}
