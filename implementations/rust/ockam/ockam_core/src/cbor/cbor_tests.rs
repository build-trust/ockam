#[cfg(test)]
mod tests {
    use minicbor::{CborLen, Decode, Encode};

    #[derive(Encode, Decode, CborLen)]
    #[cfg_attr(test, derive(Clone))]
    #[rustfmt::skip]
    #[cbor(map)]
    struct Data {
        #[n(1)] field_1: String,
    }

    #[derive(Encode, Decode, CborLen)]
    #[cfg_attr(test, derive(Clone))]
    #[rustfmt::skip]
    #[cbor(map)]
    struct DataNew {
        #[n(1)] field_1: String,
        #[n(2)] field_2: String,
    }

    #[derive(Encode, Decode, CborLen, Default)]
    #[cfg_attr(test, derive(Clone))]
    #[rustfmt::skip]
    #[cbor(map)]
    struct DataNewWithDefault {
        #[n(1)] field_1: String,
        #[n(2)] field_2: String,
    }

    #[derive(Encode, Decode, CborLen)]
    #[cfg_attr(test, derive(Clone))]
    #[rustfmt::skip]
    #[cbor(map)]
    struct DataNewWithOption {
        #[n(1)] field_1: String,
        #[n(2)] field_2: Option<String>,
    }

    #[test]
    fn test_ok_data_from_datanew() {
        // Data can be decoded from structs with extra fields
        let cases = [
            minicbor::to_vec(DataNew {
                field_1: "a".to_string(),
                field_2: "b".to_string(),
            })
            .unwrap(),
            minicbor::to_vec(DataNewWithDefault {
                field_1: "a".to_string(),
                field_2: "b".to_string(),
            })
            .unwrap(),
            minicbor::to_vec(DataNewWithOption {
                field_1: "a".to_string(),
                field_2: Some("b".to_string()),
            })
            .unwrap(),
        ];
        for case in cases {
            let decoded: Data = minicbor::decode(&case).unwrap();
            assert_eq!("a", decoded.field_1);
        }
    }

    #[test]
    fn test_err_datanew_from_data() {
        // DataNew can't be partially decoded from Data
        let d = Data {
            field_1: "a".to_string(),
        };

        let encoded = minicbor::to_vec(d).unwrap();

        let result: Result<DataNew, minicbor::decode::Error> = minicbor::decode(&encoded);
        assert!(result.is_err());
    }

    #[test]
    fn test_err_datanew_with_default_from_data() {
        // DataNewWithDefault can't be partially decoded from Data
        let a = Data {
            field_1: "a".to_string(),
        };

        let encoded = minicbor::to_vec(a).unwrap();

        let result: Result<DataNewWithDefault, minicbor::decode::Error> =
            minicbor::decode(&encoded);
        assert!(result.is_err());
    }

    #[test]
    fn test_ok_datanew_with_option_from_data() {
        // DataNewWithOption can be partially decoded from Data
        let a = Data {
            field_1: "a".to_string(),
        };

        let encoded = minicbor::to_vec(a).unwrap();

        let decoded: DataNewWithOption = minicbor::decode(&encoded).unwrap();
        assert_eq!("a", decoded.field_1);
        assert!(decoded.field_2.is_none());
    }
}
