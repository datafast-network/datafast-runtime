use crate::subgraph_filter::filter::SubgraphFilter;
use crate::subgraph_filter::FilterResult;

#[derive(Clone, Debug)]
pub struct EventFilter {
    contract: ethabi::Contract,
    address: ethabi::Address,
}

impl EventFilter {
    pub fn new(abi: &[u8], address: ethabi::Address) -> FilterResult<Self> {
        let contract = ethabi::Contract::load(abi)?;
        Ok(Self { contract, address })
    }
}

impl SubgraphFilter for EventFilter {
    fn get_contract(&self) -> ethabi::Contract {
        self.contract.clone()
    }

    fn get_address(&self) -> &ethabi::Address {
        &self.address
    }
}
