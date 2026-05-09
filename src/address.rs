use artnet_protocol::PortAddress;
use derive_more::{Deref, From, Into};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deref, From, Into)]
pub struct Address(u16);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, From)]
pub struct AddressRange {
    pub base: Address,
    pub length: u16,
}

impl Address {
    pub fn as_range(self, length: u16) -> AddressRange {
        assert!(length > 0, "length must be positive");
        AddressRange { base: self, length }
    }
}

pub struct AddressRangeIterator {
    range: AddressRange,
    current: u16,
}

impl Iterator for AddressRangeIterator {
    type Item = Address;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current < self.range.length {
            let next = Address::from(self.range.base.0 + self.current);
            self.current += 1;
            Some(next)
        } else {
            None
        }
    }
}

impl IntoIterator for AddressRange {
    type Item = Address;

    type IntoIter = AddressRangeIterator;

    fn into_iter(self) -> Self::IntoIter {
        AddressRangeIterator {
            range: self,
            current: 0,
        }
    }
}

impl From<PortAddress> for Address {
    fn from(value: PortAddress) -> Self {
        Address(value.into())
    }
}

impl From<[u8; 2]> for Address {
    fn from(value: [u8; 2]) -> Self {
        Address(u16::from_le_bytes(value))
    }
}

impl From<Address> for [u8; 2] {
    fn from(value: Address) -> Self {
        value.to_le_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::Address;

    #[test]
    fn address_conversion() {
        let addr = Address(1024);
        let bytes: [u8; 2] = addr.into();

        let addr2: Address = bytes.into();
        assert_eq!(addr, addr2);
    }

    #[test]
    fn address_range() {
        let addresses = Address::from(0x0001).as_range(3);
        let mut iter = addresses.into_iter();
        assert_eq!(iter.next(), Some(Address::from(0x0001)));
        assert_eq!(iter.next(), Some(Address::from(0x0002)));
        assert_eq!(iter.next(), Some(Address::from(0x0003)));
        assert_eq!(iter.next(), None);
    }
}
