pub(crate) const USDC_ADDRESS_STR: &str = "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174";
pub(crate) const USDC_DECIMALS: u32 = 6;

pub(crate) mod proxy {
    use alloy::primitives::U256;
    use alloy::sol;
    use anyhow::{Context, Result};
    use polymarket_client_sdk::POLYGON;
    use polymarket_client_sdk::types::{Address, B256};

    use crate::auth;

    // Polymarket Proxy Wallet Factory interface (CallType: INVALID=0, CALL=1, DELEGATECALL=2)
    sol! {
        #[sol(rpc)]
        interface IProxyWallet {
            struct ProxyCall {
                uint8 typeCode;
                address to;
                uint256 value;
                bytes data;
            }

            function proxy(ProxyCall[] memory calls)
                external payable returns (bytes[] memory returnValues);
        }
    }

    const PROXY_FACTORY: Address =
        polymarket_client_sdk::types::address!("0xaB45c5A4B0c941a2F231C04C3f49182e1A254052");

    pub fn is_proxy_mode(signature_type: Option<&str>) -> Result<bool> {
        Ok(crate::config::resolve_signature_type(signature_type)? == "proxy")
    }

    pub fn derive_proxy_address(private_key: Option<&str>) -> Result<Address> {
        let signer = auth::resolve_signer(private_key)?;
        let eoa = polymarket_client_sdk::auth::Signer::address(&signer);
        polymarket_client_sdk::derive_proxy_wallet(eoa, POLYGON)
            .ok_or_else(|| anyhow::anyhow!("Proxy wallet derivation not supported on this chain"))
    }

    pub async fn send_call(
        private_key: Option<&str>,
        use_proxy: bool,
        target: Address,
        calldata: Vec<u8>,
    ) -> Result<(B256, u64)> {
        use alloy::providers::Provider as _;

        let provider = auth::create_provider(private_key).await?;

        let (tx_hash, block_number) = if use_proxy {
            let factory = IProxyWallet::new(PROXY_FACTORY, &provider);
            let call = IProxyWallet::ProxyCall {
                typeCode: 1,
                to: target,
                value: U256::ZERO,
                data: calldata.into(),
            };
            let pending = factory.proxy(vec![call]).send().await?;
            let hash = *pending.tx_hash();
            let receipt = pending.get_receipt().await?;
            (hash, receipt.block_number)
        } else {
            let tx = alloy::rpc::types::TransactionRequest::default()
                .to(target)
                .input(alloy::primitives::Bytes::from(calldata).into());
            let pending = provider.send_transaction(tx).await?;
            let hash = *pending.tx_hash();
            let receipt = pending.get_receipt().await?;
            (hash, receipt.block_number)
        };

        let block_number = block_number.context("Block number not available in receipt")?;
        Ok((tx_hash, block_number))
    }
}

pub(crate) mod approve;
pub(crate) mod bridge;
pub(crate) mod clob;
pub(crate) mod comments;
pub(crate) mod ctf;
pub(crate) mod data;
pub(crate) mod events;
pub(crate) mod markets;
pub(crate) mod profiles;
pub(crate) mod series;
pub(crate) mod setup;
pub(crate) mod sports;
pub(crate) mod tags;
pub(crate) mod upgrade;
pub(crate) mod wallet;

pub(crate) fn is_numeric_id(id: &str) -> bool {
    id.parse::<u64>().is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_numeric_id_pure_digits() {
        assert!(is_numeric_id("12345"));
        assert!(is_numeric_id("0"));
    }

    #[test]
    fn is_numeric_id_rejects_non_digits() {
        assert!(!is_numeric_id("will-trump-win"));
        assert!(!is_numeric_id("0x123abc"));
        assert!(!is_numeric_id("123 456"));
    }

    #[test]
    fn is_numeric_id_rejects_empty() {
        assert!(!is_numeric_id(""));
    }
}
