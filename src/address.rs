use derive_more::{Deref, From, Into};
use derive_new::new;
use tiny_artnet::PortAddress;

#[derive(new, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct NetAddress {
    pub net: u8,
    pub sub_net: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deref, From, Into)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Universe(u8);

#[derive(new, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, From, Into)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Address {
    pub net: NetAddress,
    pub universe: Universe,
}

impl PartialEq<NetAddress> for Address {
    fn eq(&self, other: &NetAddress) -> bool {
        &self.net == other
    }
}

impl From<u16> for Address {
    fn from(value: u16) -> Self {
        //     | 15 | 8-14 | 4-7    | 0-3      |
        //     | 0  | Net  | SubNet | Universe |
        // (self.net as usize >> 14) + (self.sub_net as usize >> 7) + (self.universe as usize)
        let net: u8 = (value >> 8 & 0x7F) as u8;
        let sub_net = (value >> 4 & 0x0F) as u8;
        let universe = (value & 0x0F) as u8;

        Address {
            net: NetAddress { net, sub_net },
            universe: Universe(universe),
        }
    }
}

impl From<tiny_artnet::PollReply<'_>> for NetAddress {
    fn from(value: tiny_artnet::PollReply) -> Self {
        let net = value.net_switch;
        let sub_net = value.sub_switch;
        NetAddress { net, sub_net }
    }
}

impl From<tiny_artnet::PortAddress> for Address {
    fn from(value: tiny_artnet::PortAddress) -> Self {
        let net = NetAddress::new(value.net, value.sub_net);
        let universe = Universe(value.universe);
        Address { net, universe }
    }
}

impl From<Address> for tiny_artnet::PortAddress {
    fn from(value: Address) -> Self {
        PortAddress {
            sub_net: value.net.sub_net,
            net: value.net.net,
            universe: value.universe.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Address;

    #[test]
    fn address_u16() {
        let address = Address::from(0x0002);
        println!("{address:?}");
    }
}
