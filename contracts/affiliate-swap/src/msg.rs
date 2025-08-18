use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Coin, Uint128};
use osmosis_std::types::osmosis::poolmanager::v1beta1::{
    SwapAmountInRoute, SwapAmountInSplitRoute,
};

#[cw_serde]
pub struct InstantiateMsg {
    pub owner: String,
    pub affiliate_addr: String,
    pub affiliate_bps: u16, // out of 10_000 (basis points)
}

#[cw_serde]
pub enum ProxySwap {
    SwapExactAmountIn {
        routes: Vec<SwapAmountInRoute>,
        token_in: Coin,
        token_out_min_amount: Uint128,
    },
    SplitRouteSwapExactAmountIn {
        routes: Vec<SwapAmountInSplitRoute>,
        token_in_denom: String,
        token_out_min_amount: Uint128,
    },
}

#[cw_serde]
pub enum ExecuteMsg {
    ProxySwapWithFee {
        swap: ProxySwap,
    },
    UpdateAffiliate {
        affiliate_addr: String,
        affiliate_bps: u16,
    },
    TransferOwnership {
        new_owner: String,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(ConfigResponse)]
    Config {},
}

#[cw_serde]
pub struct ConfigResponse {
    pub owner: String,
    pub affiliate_addr: String,
    pub affiliate_bps: u16,
}

#[cw_serde]
pub struct SwapResponse {
    pub original_sender: String,
    pub token_out_denom: String,
    pub amount_sent_to_user: Uint128,
    pub amount_sent_to_affiliate: Uint128,
}
