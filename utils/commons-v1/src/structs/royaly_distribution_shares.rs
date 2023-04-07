use super::*;

#[derive(SchemaType, Serialize, Eq, PartialEq, PartialOrd, Ord, Debug)]
pub struct Shares {
    pub creator: Amount,
    pub minter: Amount,
    pub owner: Amount,
    pub bictory: Amount,
}

impl Shares {
    pub fn adjust_owner_share(&mut self) {
        self.owner -= self.creator + self.minter + self.bictory
    }
}
