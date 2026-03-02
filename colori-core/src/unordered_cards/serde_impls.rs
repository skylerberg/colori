use serde::{Deserialize, Serialize};

use super::bitset::{UnorderedBuyers, UnorderedCards};
use super::registry::{BUYER_REGISTRY, CARD_REGISTRY};
use crate::types::{BuyerInstance, CardInstance};

// Serde for UnorderedCards: serialize as Vec<CardInstance>, deserialize and rebuild bitset
impl Serialize for UnorderedCards {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let cards: Vec<CardInstance> = CARD_REGISTRY.with(|r| {
            let reg = r.borrow();
            self.iter()
                .map(|id| CardInstance {
                    instance_id: id as u32,
                    card: reg[id as usize],
                })
                .collect()
        });
        cards.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for UnorderedCards {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let cards = Vec::<CardInstance>::deserialize(deserializer)?;
        let mut bitset = UnorderedCards::new();
        CARD_REGISTRY.with(|r| {
            let mut reg = r.borrow_mut();
            for c in &cards {
                let id = c.instance_id as u8;
                reg[id as usize] = c.card;
                bitset.insert(id);
            }
        });
        Ok(bitset)
    }
}

// Serde for UnorderedBuyers: serialize as Vec<BuyerInstance>, deserialize and rebuild bitset
impl Serialize for UnorderedBuyers {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let buyers: Vec<BuyerInstance> = BUYER_REGISTRY.with(|r| {
            let reg = r.borrow();
            self.iter()
                .map(|id| BuyerInstance {
                    instance_id: id as u32,
                    buyer: reg[id as usize],
                })
                .collect()
        });
        buyers.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for UnorderedBuyers {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let buyers = Vec::<BuyerInstance>::deserialize(deserializer)?;
        let mut bitset = UnorderedBuyers::new();
        BUYER_REGISTRY.with(|r| {
            let mut reg = r.borrow_mut();
            for b in &buyers {
                let id = b.instance_id as u8;
                reg[id as usize] = b.buyer;
                bitset.insert(id);
            }
        });
        Ok(bitset)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::registry::{set_buyer_registry, set_card_registry};
    use crate::types::{BuyerCard, Card};

    #[test]
    fn test_serde_roundtrip_cards() {
        let mut registry = [Card::BasicRed; 256];
        registry[0] = Card::BasicRed;
        registry[5] = Card::BasicYellow;
        registry[10] = Card::BasicBlue;
        set_card_registry(&registry);

        let mut s = UnorderedCards::new();
        s.insert(0);
        s.insert(5);
        s.insert(10);

        let json = serde_json::to_string(&s).unwrap();
        let deserialized: UnorderedCards = serde_json::from_str(&json).unwrap();
        assert_eq!(s, deserialized);
    }

    #[test]
    fn test_serde_roundtrip_empty() {
        set_card_registry(&[Card::BasicRed; 256]);
        let s = UnorderedCards::new();
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(json, "[]");
        let deserialized: UnorderedCards = serde_json::from_str(&json).unwrap();
        assert_eq!(s, deserialized);
    }

    #[test]
    fn test_serde_roundtrip_buyers() {
        let mut registry = [BuyerCard::Textiles2Vermilion; 256];
        registry[0] = BuyerCard::Textiles2Vermilion;
        registry[3] = BuyerCard::Textiles2Amber;
        registry[7] = BuyerCard::Textiles2Chartreuse;
        set_buyer_registry(&registry);

        let mut s = UnorderedBuyers::new();
        s.insert(0);
        s.insert(3);
        s.insert(7);

        let json = serde_json::to_string(&s).unwrap();
        let deserialized: UnorderedBuyers = serde_json::from_str(&json).unwrap();
        assert_eq!(s, deserialized);
    }
}
