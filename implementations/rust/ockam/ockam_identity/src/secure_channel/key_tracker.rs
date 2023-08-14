use ockam_core::Result;
use ockam_vault::KeyId;
use tracing::debug;
use tracing::warn;

use crate::IdentityError;

pub(crate) struct KeyTracker {
    pub(crate) current_key: KeyId,
    pub(crate) previous_key: Option<KeyId>,
    number_of_rekeys: u64,
    max_rekeys_reached: bool,
    renewal_interval: u64,
}

impl KeyTracker {
    pub(crate) fn new(key_id: KeyId, renewal_interval: u64) -> Self {
        KeyTracker {
            current_key: key_id,
            number_of_rekeys: 0,
            max_rekeys_reached: false,
            previous_key: None,
            renewal_interval,
        }
    }
}

impl KeyTracker {
    /// The rekeying algorithm specifies a series of intervals of size self.renewal_interval
    /// where each interval corresponds to a set of contiguous nonces using the same key.
    ///
    /// This function returns the key corresponding to the current nonce.
    ///
    /// This is either:
    ///   - the current key if the nonce falls into the current interval
    ///   - the previous key if the nonce falls before the current interval
    ///   - nothing if the the nonce falls after the current interval -> this indicates that a new key must be created
    ///   - an error if
    ///      - if the the nonce falls before the previous interval
    ///      - if it the previous nonce but is not set
    ///      - we reached the maximum number of rekeyings
    pub(crate) fn get_key(&self, nonce: u64) -> Result<Option<KeyId>> {
        debug!(
            "The current number of rekeys is {}, the rekey interval is {}",
            self.number_of_rekeys, self.renewal_interval
        );

        // for example 2 rekeys happened, renewal every 10 keys
        // current batch of nonces -> 20 to 29
        let current_interval_start = self.number_of_rekeys * self.renewal_interval;

        // if we reached the maximum number of rekeyings we stop operating on this secure channel
        if self.max_rekeys_reached {
            warn!("The maximum number of available rekeying operation has been reached. The last interval was starting at {} and the interval size is {}",
                current_interval_start, self.renewal_interval);
            return Err(IdentityError::InvalidNonce.into());
        };

        if nonce >= current_interval_start {
            let nonce_age = nonce - current_interval_start;
            // if the nonce falls in the current interval return the current key
            if nonce_age < self.renewal_interval {
                Ok(Some(self.current_key.clone()))
            }
            // if the nonce falls in the next interval
            // otherwise indicate that we need to create a new key
            else if nonce_age < self.renewal_interval * 2 {
                Ok(None)
            }
            // otherwise the nonce is too far ahead
            else {
                warn!("This nonce is too far in the future: {}", nonce);
                Err(IdentityError::InvalidNonce.into())
            }
        // else return the previous key (if there is one) if the nonce is not too old
        } else if current_interval_start - nonce <= self.renewal_interval {
            if let Some(previous) = self.previous_key.clone() {
                Ok(Some(previous))
            } else {
                warn!("There should be a previous key for this nonce: {}", nonce);
                Err(IdentityError::InvalidNonce.into())
            }
        } else {
            warn!("This nonce is too old: {}", nonce);
            Err(IdentityError::InvalidNonce.into())
        }
    }

    // Update the key if a key renewal happened
    pub(crate) fn update_key(&mut self, decryption_key: KeyId) -> Result<Option<KeyId>> {
        let mut key_to_delete = None;
        // if the key used for the decryption is not the current key nor the previous key
        // this means that a rekeying happened
        if decryption_key != self.current_key && Some(decryption_key.clone()) != self.previous_key {
            if let Some(previous) = self.previous_key.clone() {
                key_to_delete = Some(previous)
            }
            self.previous_key.replace(self.current_key.clone());
            self.current_key = decryption_key;
            if u64::MAX - self.number_of_rekeys * self.renewal_interval < self.renewal_interval {
                self.max_rekeys_reached = true;
            } else {
                self.number_of_rekeys += 1;
            }
        }
        Ok(key_to_delete)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_key_first_interval() {
        let key_id = "key_id".to_string();
        let key_tracker = KeyTracker::new(key_id.clone(), 10);

        assert_eq!(key_tracker.get_key(0).unwrap(), Some(key_id.clone()));
        assert_eq!(key_tracker.get_key(5).unwrap(), Some(key_id.clone()));
        assert_eq!(key_tracker.get_key(9).unwrap(), Some(key_id));
        assert_eq!(
            key_tracker.get_key(10).unwrap(),
            None,
            "the next key must be created"
        );
        assert_eq!(
            key_tracker.get_key(20).ok(),
            None,
            "this nonce is too far in the future"
        );
        assert_eq!(
            key_tracker.get_key(u64::MAX).ok(),
            None,
            "this nonce is too far in the future"
        );
    }

    #[test]
    fn test_get_key_middle_interval() {
        let key_id = "key_id".to_string();
        let previous_key_id = "previous_key_id".to_string();
        let key_tracker = KeyTracker {
            current_key: key_id.clone(),
            number_of_rekeys: 5,
            max_rekeys_reached: false,
            previous_key: Some(previous_key_id.clone()),
            renewal_interval: 10,
        };

        assert_eq!(
            key_tracker.get_key(0).ok(),
            None,
            "this nonce is too far in the past"
        );
        assert_eq!(
            key_tracker.get_key(30).ok(),
            None,
            "this nonce is too far in the past"
        );
        assert_eq!(
            key_tracker.get_key(39).ok(),
            None,
            "this nonce is too far in the past"
        );
        assert_eq!(
            key_tracker.get_key(40).unwrap(),
            Some(previous_key_id.clone())
        );
        assert_eq!(
            key_tracker.get_key(45).unwrap(),
            Some(previous_key_id.clone())
        );
        assert_eq!(key_tracker.get_key(49).unwrap(), Some(previous_key_id));
        assert_eq!(key_tracker.get_key(50).unwrap(), Some(key_id.clone()));
        assert_eq!(key_tracker.get_key(59).unwrap(), Some(key_id));
        assert_eq!(
            key_tracker.get_key(60).unwrap(),
            None,
            "the next key must be created"
        );
        assert_eq!(
            key_tracker.get_key(u64::MAX).ok(),
            None,
            "this nonce is too far in the future"
        );
    }

    #[test]
    fn test_get_key_last_interval() {
        let key_id = "key_id".to_string();
        let previous_key_id = "previous_key_id".to_string();
        let key_tracker = KeyTracker {
            current_key: key_id,
            number_of_rekeys: 5,
            max_rekeys_reached: true,
            previous_key: Some(previous_key_id),
            renewal_interval: 10,
        };

        assert_eq!(
            key_tracker.get_key(0).ok(),
            None,
            "we reached the last interval already. The channel needs to be recreated"
        );
    }

    #[test]
    fn test_update_key() {
        let key_id = "key_id".to_string();
        let previous_key_id = "previous_key_id".to_string();
        let new_key_id = "new_key_id".to_string();
        let mut key_tracker = KeyTracker {
            current_key: key_id.clone(),
            number_of_rekeys: 5,
            max_rekeys_reached: false,
            previous_key: Some(previous_key_id.clone()),
            renewal_interval: 10,
        };

        assert_eq!(key_tracker.update_key(key_id.clone()).unwrap(), None);
        assert_eq!(
            key_tracker.update_key(previous_key_id.clone()).unwrap(),
            None
        );
        assert_eq!(
            key_tracker.update_key(new_key_id.clone()).unwrap(),
            Some(previous_key_id),
            "the previous key id must be returned in order to be deleted",
        );
        assert_eq!(key_tracker.current_key, new_key_id);
        assert_eq!(key_tracker.previous_key, Some(key_id));
    }

    #[test]
    fn test_update_key_on_last_interval() {
        let key_id = "key_id".to_string();
        let previous_key_id = "previous_key_id".to_string();
        let new_key_id = "new_key_id".to_string();
        let mut key_tracker = KeyTracker {
            current_key: key_id,
            number_of_rekeys: u64::MAX / 10 - 1,
            max_rekeys_reached: false,
            previous_key: Some(previous_key_id),
            renewal_interval: 10,
        };

        // this brings us to the last interval
        key_tracker.update_key(new_key_id).unwrap();
        assert!(
            !key_tracker.max_rekeys_reached,
            "the maximum number of rekeys is not yet reached"
        );

        // now there are no more intervals available
        let new_key_id_2 = "new_key_id_2".to_string();
        key_tracker.update_key(new_key_id_2).unwrap();
        assert!(
            key_tracker.max_rekeys_reached,
            "the maximum number of rekeys is reached now"
        );
    }
}
