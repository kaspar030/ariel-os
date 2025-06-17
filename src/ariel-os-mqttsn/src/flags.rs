use bilge::prelude::*;

#[bitsize(2)]
#[derive(FromBits, Debug)]
pub enum QoS {
    /// A packet will be delivered at most once, but may not be delivered at all.
    Zero = 0b00,
    /// A packet will be delivered at least one time, but possibly more than once.
    One = 0b01,
    /// A packet will be delivered exactly one time.
    Two = 0b10,
    /// A packet will be sent to a known topic id, without establishing a connection beforehand.
    MinusOne = 0b11,
}

#[bitsize(2)]
#[derive(TryFromBits, Debug, PartialEq)]
pub enum TopicIdType {
    IdNormal = 0b00,
    IdPredefined = 0b01,
    ShortName = 0b10,
}

#[bitsize(8)]
#[derive(TryFromBits, DebugBits, PartialEq)]
pub struct Flags {
    pub topic_id_type: TopicIdType,
    pub clean_session: bool,
    pub will: bool,
    pub retain: bool,
    pub qos: QoS,
    pub dup: bool,
}

// #[cfg(test)]
// #[embedded_test::tests]
// mod tests {
//     use crate::flags::{Flags, QoS, TopicIdType};
//     use ariel_os::debug::log::info;
//     use bilge::prelude::Number;
//
//     #[test]
//     fn basic() {
//         assert_eq!(
//             Flags::new(
//                 TopicIdType::IdPredefined,
//                 false,
//                 false,
//                 false,
//                 QoS::MinusOne,
//                 false,
//             ),
//             Flags::try_from(u8::new(0b0_11_0_0_0_01)).unwrap()
//         )
//     }
// }
