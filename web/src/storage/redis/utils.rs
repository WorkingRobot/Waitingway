use serde::{Deserialize, Serialize};

use crate::config::RedisConfig;

pub trait RedisKey: Serialize {
    const PREFIX: &'static str;

    #[inline]
    fn to_key<'a>(&'a self, config: impl Into<&'a RedisConfig>) -> postcard::Result<Vec<u8>> {
        // namespace:prefix:postcard-serialized-key

        let mut ret = config.into().namespace.clone();
        ret.push(':');
        ret.push_str(Self::PREFIX);
        ret.push(':');
        postcard::to_extend(self, ret.into_bytes())
    }
}

pub trait RedisValue: Serialize + for<'de> Deserialize<'de> {
    #[inline]
    fn to_value(&self) -> postcard::Result<Vec<u8>> {
        postcard::to_allocvec(self)
    }

    #[inline]
    fn from_value(value: &[u8]) -> postcard::Result<Self> {
        postcard::from_bytes(value)
    }
}
#[cfg(test)]
mod tests {
    use crate::subscriptions::Subscriber;

    use super::*;

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    enum TestKey {
        A(u32),
        B(i64),
    }

    impl RedisKey for TestKey {
        const PREFIX: &'static str = "testkey";
    }

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct TestValue {
        data: String,
    }

    impl RedisValue for TestValue {}

    #[test]
    fn test_redis_key_to_key() {
        let config = RedisConfig {
            namespace: "testnamespace".to_string(),
            ..Default::default()
        };

        let key_a = TestKey::A(42);
        let key_b = TestKey::B(64);

        let serialized_key_a = key_a.to_key(&config).unwrap();
        let serialized_key_b = key_b.to_key(&config).unwrap();

        assert!(serialized_key_a.starts_with(b"testnamespace:testkey:"));
        assert!(serialized_key_b.starts_with(b"testnamespace:testkey:"));
    }

    #[test]
    fn test_redis_value_to_value() {
        let value = TestValue {
            data: "testdata".to_string(),
        };

        let serialized_value = value.to_value().unwrap();
        let deserialized_value: TestValue = RedisValue::from_value(&serialized_value).unwrap();

        assert_eq!(value, deserialized_value);
    }

    #[test]
    fn test_redis_value_from_value() {
        let value = TestValue {
            data: "testdata".to_string(),
        };

        let serialized_value = value.to_value().unwrap();
        let deserialized_value: TestValue = RedisValue::from_value(&serialized_value).unwrap();

        assert_eq!(value, deserialized_value);
    }

    #[test]
    fn test_user() {
        let user = Subscriber::Discord(246746545561665537);
        let data = vec![0, 129, 128, 136, 174, 208, 213, 167, 182, 3];

        let serialize_user = user.to_value().unwrap();
        assert_eq!(serialize_user, data);

        let deserialized_user = Subscriber::from_value(&serialize_user).unwrap();
        assert_eq!(user, deserialized_user);
    }
}
